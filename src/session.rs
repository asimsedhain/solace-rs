use crate::context::SolContext;
use crate::message::InboundMessage;
use crate::solace::ffi;
use crate::{Result, SolaceError, SolClientReturnCode};
use num_traits::FromPrimitive;
use std::ffi::{c_void, CString};
use std::ptr;

pub struct SolSession {
    // Pointer to session
    // This pointer must never be allowed to leave the struct
    pub(crate) _session_pt: ffi::solClient_opaqueSession_pt,
}

// TODO
// These are temp callbacks
// Implement callbacks that can be passed in
extern "C" fn on_event(
    opaque_session_p: ffi::solClient_opaqueSession_pt, // non-null
    event_info_p: ffi::solClient_session_eventCallbackInfo_pt, //non-null
    user_p: *mut ::std::os::raw::c_void,               // can be null
) {
    println!("some event recienved");
}

fn get_on_message_trampoline<F>(_closure: &F) -> ffi::solClient_session_rxMsgCallbackFunc_t
where
    F: FnMut(InboundMessage) + Send + 'static,
{
    Some(static_on_message::<F>)
}

extern "C" fn static_on_message<F>(
    _opaque_session_p: ffi::solClient_opaqueSession_pt, // non-null
    msg_p: ffi::solClient_opaqueMsg_pt,                 // non-null
    raw_user_closure: *mut ::std::os::raw::c_void,      // can be null
) -> ffi::solClient_rxMsgCallback_returnCode_t
where
    // not completely sure if this is supposed to be FnMut or FnOnce
    // threading takes in FnOnce - that is why I suspect it might be FnOnce.
    // But not enough knowledge to make sure it is FnOnce.
    F: FnMut(InboundMessage) + Send + 'static,
{
    // this function is glue code to allow users to pass in closures
    // we duplicate the message pointer (which does not copy over the binary data)
    // also this function will only be called from the context thread, so it should be thread safe
    // as well

    let non_null_raw_user_closure = std::ptr::NonNull::new(raw_user_closure);

    let Some(raw_user_closure) =  non_null_raw_user_closure else{
        return ffi::solClient_rxMsgCallback_returnCode_SOLCLIENT_CALLBACK_OK;
    };

    let mut dup_msg_ptr = ptr::null_mut();
    unsafe { ffi::solClient_msg_dup(msg_p, &mut dup_msg_ptr) };

    let message = InboundMessage::from(dup_msg_ptr);
    let user_closure = unsafe { &mut *(raw_user_closure.as_ptr() as *mut F) };
    user_closure(message);

    ffi::solClient_rxMsgCallback_returnCode_SOLCLIENT_CALLBACK_OK
}

impl SolSession {
    pub fn new<H, V, U, P, F>(
        host_name: H,
        vpn_name: V,
        username: U,
        password: P,
        // since the solace_context has the threading library
        // might be good to get the context entirely instead of a reference to the context
        context: &SolContext,
        on_message: Option<F>,
        //on_event: Fn,
    ) -> Result<Self>
    where
        H: Into<Vec<u8>>,
        V: Into<Vec<u8>>,
        U: Into<Vec<u8>>,
        P: Into<Vec<u8>>,
        F: FnMut(InboundMessage) + Send + 'static,
    {
        /* Session */
        //solClient_opaqueSession_pt session_p;
        //solClient_session_createFuncInfo_t sessionFuncInfo = SOLCLIENT_SESSION_CREATEFUNC_INITIALIZER;

        // Converting props and storing them session props
        let c_host_name = CString::new(host_name).expect("Invalid vpn_name");
        let c_vpn_name = CString::new(vpn_name).expect("Invalid vpn_name");
        let c_username = CString::new(username).expect("Invalid username");
        let c_password = CString::new(password).expect("Invalid password");

        // Session props is a **char in C
        // it takes in an array of key and values
        // first we specify the key, then the value
        let session_props = [
            ffi::SOLCLIENT_SESSION_PROP_HOST.as_ptr(),
            c_host_name.as_ptr() as *const u8,
            ffi::SOLCLIENT_SESSION_PROP_VPN_NAME.as_ptr(),
            c_vpn_name.as_ptr() as *const u8,
            ffi::SOLCLIENT_SESSION_PROP_USERNAME.as_ptr(),
            c_username.as_ptr() as *const u8,
            ffi::SOLCLIENT_SESSION_PROP_PASSWORD.as_ptr(),
            c_password.as_ptr() as *const u8,
            ffi::SOLCLIENT_SESSION_PROP_CONNECT_BLOCKING.as_ptr(),
            ffi::SOLCLIENT_PROP_ENABLE_VAL.as_ptr(),
            ptr::null(),
        ]
        .as_mut_ptr() as *mut *const i8;

        let mut session_pt: ffi::solClient_opaqueSession_pt = ptr::null_mut();

        let (static_on_message_callback, user_on_message) = match on_message {
            Some(mut f) => (
                get_on_message_trampoline(&f),
                &mut f as *mut _ as *mut c_void,
            ),
            _ => (None, ptr::null_mut()),
        };

        // Function information for Session creation.
        // The application must set the eventInfo callback information. All Sessions must have an event callback registered.
        let mut session_func_info: ffi::solClient_session_createFuncInfo_t =
            ffi::solClient_session_createFuncInfo {
                rxInfo: ffi::solClient_session_createRxCallbackFuncInfo {
                    callback_p: ptr::null_mut(),
                    user_p: ptr::null_mut(),
                },
                eventInfo: ffi::solClient_session_createEventCallbackFuncInfo {
                    callback_p: Some(on_event),
                    user_p: ptr::null_mut(),
                },
                rxMsgInfo: ffi::solClient_session_createRxMsgCallbackFuncInfo {
                    callback_p: static_on_message_callback,
                    user_p: user_on_message,
                },
            };

        // TODO
        // needs to be fixed
        let session_create_result = unsafe {
            ffi::solClient_session_create(
                session_props,
                context._ctx,
                &mut session_pt,
                &mut session_func_info,
                std::mem::size_of::<ffi::solClient_session_createFuncInfo_t>(),
            )
        };

        if SolClientReturnCode::from_i32(session_create_result) != Some(SolClientReturnCode::OK) {
            panic!("Could not initialize solace session");
            //return Err(SolaceError);
        }

        let connection_result = unsafe { ffi::solClient_session_connect(session_pt) };

        if SolClientReturnCode::from_i32(connection_result) == Some(SolClientReturnCode::OK) {
            Ok(SolSession {
                _session_pt: session_pt,
            })
        } else {
            println!("Solace did not connect properly");
            println!("Returned: {}", connection_result);

            Err(SolaceError)
        }
    }

    pub fn publish<T, M>(&self, topic: T, message: M) -> Result<()>
    where
        M: Into<String>,
        T: Into<Vec<u8>>,
    {
        // to accomplish the publishing,
        // we will create a null_ptr
        // allocate it using the provided function
        // attach the destination to the ptr
        // attach the message to the ptr
        //
        // for attaching the message to the ptr, we have a couple of options
        // based on those options, we can create a couple of interfaces
        //
        // solClient_msg_setBinaryAttachmentPtr (solClient_opaqueMsg_pt msg_p, void *buf_p, solClient_uint32_t size)
        // Given a msg_p, set the contents of a Binary Attachment Part to the given pointer and size.
        //
        // solClient_msg_setBinaryAttachment (solClient_opaqueMsg_pt msg_p, const void *buf_p, solClient_uint32_t size)
        // Given a msg_p, set the contents of the binary attachment part by copying in from the given pointer and size.
        //
        // solClient_msg_setBinaryAttachmentString (solClient_opaqueMsg_pt msg_p, const char *buf_p)
        // Given a msg_p, set the contents of the binary attachment part to a UTF-8 or ASCII string by copying in from the given pointer until null-terminated.
        //

        let c_topic = CString::new(topic).expect("Invalid topic");

        let mut msg_ptr: ffi::solClient_opaqueMsg_pt = ptr::null_mut();

        let msg_alloc_result = unsafe { ffi::solClient_msg_alloc(&mut msg_ptr) };
        assert_eq!(
            SolClientReturnCode::from_i32(msg_alloc_result),
            Some(SolClientReturnCode::OK)
        );

        let set_delivery_result = unsafe {
            ffi::solClient_msg_setDeliveryMode(msg_ptr, ffi::SOLCLIENT_DELIVERY_MODE_DIRECT)
        };
        assert_eq!(
            SolClientReturnCode::from_i32(set_delivery_result),
            Some(SolClientReturnCode::OK)
        );

        let mut destination: ffi::solClient_destination = ffi::solClient_destination {
            destType: ffi::solClient_destinationType_SOLCLIENT_TOPIC_DESTINATION,
            dest: c_topic.as_ptr(),
        };

        let set_destination_result = unsafe {
            ffi::solClient_msg_setDestination(
                msg_ptr,
                &mut destination,
                std::mem::size_of::<ffi::solClient_destination>(),
            )
        };
        assert_eq!(
            SolClientReturnCode::from_i32(set_destination_result),
            Some(SolClientReturnCode::OK)
        );

        let c_message = CString::new(message.into()).expect("Invalid message");

        let set_attachment_result =
            unsafe { ffi::solClient_msg_setBinaryAttachmentString(msg_ptr, c_message.as_ptr()) };
        assert_eq!(
            SolClientReturnCode::from_i32(set_attachment_result),
            Some(SolClientReturnCode::OK)
        );

        let send_message_result =
            unsafe { ffi::solClient_session_sendMsg(self._session_pt, msg_ptr) };
        assert_eq!(
            SolClientReturnCode::from_i32(send_message_result),
            Some(SolClientReturnCode::OK)
        );

        unsafe {
            ffi::solClient_msg_free(&mut msg_ptr);
        }

        Ok(())
    }

    pub fn subscribe<T>(&self, topic: T) -> Result<()>
    where
        T: Into<Vec<u8>>,
    {
        let c_topic = CString::new(topic).expect("Invalid topic");
        let subscription_result =
            unsafe { ffi::solClient_session_topicSubscribe(self._session_pt, c_topic.as_ptr()) };

        if SolClientReturnCode::from_i32(subscription_result) != Some(SolClientReturnCode::OK) {
            return Err(SolaceError);
        }
        Ok(())
    }

    pub fn unsubscribe<T>(&self, topic: T) -> Result<()>
    where
        T: Into<Vec<u8>>,
    {
        let c_topic = CString::new(topic).expect("Invalid topic");
        let subscription_result =
            unsafe { ffi::solClient_session_topicUnsubscribe(self._session_pt, c_topic.as_ptr()) };

        if SolClientReturnCode::from_i32(subscription_result) != Some(SolClientReturnCode::OK) {
            return Err(SolaceError);
        }
        Ok(())
    }
}

impl Drop for SolSession {
    fn drop(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    use crate::SolaceLogLevel;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn it_works() {
        let solace_context = SolContext::new(SolaceLogLevel::Warning).unwrap();
        println!("Context created");
        let host_name = "tcp://localhost:55554";
        let vpn_name = "default";
        let username = "default";
        let password = "";
        let session_result = SolSession::new(
            host_name,
            vpn_name,
            username,
            password,
            &solace_context,
            None::<fn(_)>,
        );

        let Ok(session) = session_result else{
            panic!();
        };

        let topic = "try-me";
        println!("Session created");
        println!("Subscribing to {} topic", topic);

        let sub_result = session.subscribe(topic);
        assert!(sub_result.is_ok());

        println!("Sleeping for 10 secs before publishig messages",);
        sleep(Duration::new(10, 0));

        for i in 0..10 {
            session
                .publish(topic, format!("hello from rust: {}", i))
                .expect("message to be sent");
            sleep(Duration::new(1, 0));
        }

        let sleep_duration = Duration::new(30, 0);
        println!("Sleeping for {:?}", sleep_duration);
        sleep(sleep_duration);

        println!("Unsubscribing to {} topic", topic);

        let sub_result = session.unsubscribe(topic);
        assert!(sub_result.is_ok());
    }

    #[test]
    fn it_subscribes_and_listens() {
        let solace_context = SolContext::new(SolaceLogLevel::Warning).unwrap();
        println!("Context created");
        let host_name = "tcp://localhost:55554";
        let vpn_name = "default";
        let username = "default";
        let password = "";
        let on_message = |message: InboundMessage| {
            if let Ok(payload) = message.get_payload_as_bytes() {
                if let Ok(m) = std::str::from_utf8(payload) {
                    println!("on_message handler got: {}", m);
                } else {
                    println!("on_message handler could not decode");
                }
            } else {
                println!("on_message handler could not decode bytes");
            }
        };
        let session_result = SolSession::new(
            host_name,
            vpn_name,
            username,
            password,
            &solace_context,
            Some(on_message),
        );

        let Ok(session) = session_result else{
            panic!();
        };

        let topic = "try-me";
        println!("Session created");
        println!("Subscribing to {} topic", topic);

        let sub_result = session.subscribe(topic);
        assert!(sub_result.is_ok());

        let sleep_duration = Duration::new(60, 0);
        println!("Sleeping for {:?}", sleep_duration);
        sleep(sleep_duration);

        println!("Unsubscribing to {} topic", topic);

        let sub_result = session.unsubscribe(topic);
        assert!(sub_result.is_ok());
    }
}
