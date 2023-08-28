mod error;
mod util;

use crate::context::SolContext;
use crate::event::SessionEvent;
use crate::message::{InboundMessage, Message, OutboundMessage};
use crate::solace::ffi;
use crate::SolClientReturnCode;
use error::SessionError;
use num_traits::FromPrimitive;
use std::ffi::{c_void, CString};
use std::marker::PhantomData;
use std::ptr;
use util::{on_event_trampoline, on_message_trampoline};

type Result<T> = std::result::Result<T, SessionError>;

pub struct SolSession<'a> {
    // Pointer to session
    // This pointer must never be allowed to leave the struct
    pub(crate) _session_pt: ffi::solClient_opaqueSession_pt,
    // phantomdata to tie context with session
    // context needs to be alive for session to function
    _phantom_context: PhantomData<&'a SolContext>,
}

impl<'a> SolSession<'a> {
    pub fn new<H, V, U, P, M, E>(
        host_name: H,
        vpn_name: V,
        username: U,
        password: P,
        // since the solace_context has the threading library
        // might be good to get the context entirely instead of a reference to the context
        context: &'a SolContext,
        on_message: Option<M>,
        on_event: Option<E>,
    ) -> Result<Self>
    where
        H: Into<Vec<u8>>,
        V: Into<Vec<u8>>,
        U: Into<Vec<u8>>,
        P: Into<Vec<u8>>,
        M: FnMut(InboundMessage) + Send + 'static,
        E: FnMut(SessionEvent) + Send + 'static,
    {
        /* Session */
        //solClient_opaqueSession_pt session_p;
        //solClient_session_createFuncInfo_t sessionFuncInfo = SOLCLIENT_SESSION_CREATEFUNC_INITIALIZER;

        // Converting props and storing them session props
        let c_host_name = CString::new(host_name)?;
        let c_vpn_name = CString::new(vpn_name)?;
        let c_username = CString::new(username)?;
        let c_password = CString::new(password)?;

        // Session props is a **char in C
        // it takes in an array of key and values
        // first we specify the key, then the value
        // Session also copies over the props and maintains a copy internally.
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
            Some(mut f) => (on_message_trampoline(&f), &mut f as *mut _ as *mut c_void),
            _ => (None, ptr::null_mut()),
        };

        let (static_on_event_callback, user_on_event) = match on_event {
            Some(mut f) => (on_event_trampoline(&f), &mut f as *mut _ as *mut c_void),
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
                    callback_p: static_on_event_callback,
                    user_p: user_on_event,
                },
                rxMsgInfo: ffi::solClient_session_createRxMsgCallbackFuncInfo {
                    callback_p: static_on_message_callback,
                    user_p: user_on_message,
                },
            };

        let session_create_result = unsafe {
            ffi::solClient_session_create(
                session_props,
                context._ctx,
                &mut session_pt,
                &mut session_func_info,
                std::mem::size_of::<ffi::solClient_session_createFuncInfo_t>(),
            )
        };

        if SolClientReturnCode::from_i32(session_create_result) != Some(SolClientReturnCode::Ok) {
            return Err(SessionError::InitializationFailure);
        }

        let connection_result = unsafe { ffi::solClient_session_connect(session_pt) };

        if SolClientReturnCode::from_i32(connection_result) == Some(SolClientReturnCode::Ok) {
            Ok(SolSession {
                _session_pt: session_pt,
                _phantom_context: PhantomData,
            })
        } else {
            Err(SessionError::ConnectionFailure)
        }
    }

    pub fn publish(&self, message: OutboundMessage) -> Result<()> {
        let send_message_result = unsafe {
            ffi::solClient_session_sendMsg(self._session_pt, message.get_raw_message_ptr())
        };
        assert_eq!(
            SolClientReturnCode::from_i32(send_message_result),
            Some(SolClientReturnCode::Ok)
        );

        Ok(())
    }

    pub fn subscribe<T>(&self, topic: T) -> Result<()>
    where
        T: Into<Vec<u8>>,
    {
        let c_topic = CString::new(topic)?;
        let subscription_result =
            unsafe { ffi::solClient_session_topicSubscribe(self._session_pt, c_topic.as_ptr()) };

        if SolClientReturnCode::from_i32(subscription_result) != Some(SolClientReturnCode::Ok) {
            return Err(SessionError::SubscriptionFailure(
                c_topic.to_string_lossy().into_owned(),
            ));
        }
        Ok(())
    }

    pub fn unsubscribe<T>(&self, topic: T) -> Result<()>
    where
        T: Into<Vec<u8>>,
    {
        let c_topic = CString::new(topic)?;
        let subscription_result =
            unsafe { ffi::solClient_session_topicUnsubscribe(self._session_pt, c_topic.as_ptr()) };

        if SolClientReturnCode::from_i32(subscription_result) != Some(SolClientReturnCode::Ok) {
            return Err(SessionError::UnsubscriptionFailure(
                c_topic.to_string_lossy().into_owned(),
            ));
        }
        Ok(())
    }
}

impl<'a> Drop for SolSession<'a> {
    fn drop(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{
        DeliveryMode, DestinationType, Message, MessageDestination, OutboundMessageBuilder,
    };
    use crate::SolaceLogLevel;
    use std::thread::sleep;
    use std::time::Duration;

    fn create_print_session(context: &SolContext) -> Result<SolSession> {
        /* utility function for creating basic printing sol session
         * just prints the messages to stdout
         */

        let host_name = "tcp://localhost:55554";
        let vpn_name = "default";
        let username = "default";
        let password = "";
        let on_message = |message: InboundMessage| {
            let Ok(payload) = message.get_payload() else{
                println!("on_message handler could not decode bytes");
                return;
            };
            let Ok(payload) = std::str::from_utf8(payload) else{
                println!("on_message handler could not decode");
                return
            };
            println!("on_message handler got: {}", payload);

            let Ok(Some(dest)) = message.get_destination()else{
                println!("on_message handler could not get destination");
                return;
            };
            println!(
                "on_message handler got message on: {:?} {:?}",
                dest.dest_type, dest.dest
            );
        };

        let on_event = |e: SessionEvent| {
            println!("on_event handler got: {}", e);
        };
        SolSession::new(
            host_name,
            vpn_name,
            username,
            password,
            context,
            Some(on_message),
            Some(on_event),
        )
    }

    #[test]
    #[ignore]
    fn it_subscribes_and_publishes() {
        let solace_context = SolContext::new(SolaceLogLevel::Warning)
            .map_err(|_| SessionError::InitializationFailure)
            .unwrap();
        println!("Context created");
        let session = create_print_session(&solace_context).unwrap();
        println!("Session created");

        let topic = "try-me";
        let sub_result = session.subscribe(topic);
        println!("sub result: {:?}", sub_result);
        assert!(sub_result.is_ok());
        println!("Subscribed to {} topic", topic);

        println!("Sleeping for 10 secs before publishig messages",);
        sleep(Duration::new(10, 0));

        for i in 0..10 {
            let message = {
                let dest = MessageDestination::new(DestinationType::Topic, topic).unwrap();

                OutboundMessageBuilder::new()
                    .destination(dest)
                    .delivery_mode(DeliveryMode::Direct)
                    .payload(format!("hello from rust: {}", i))
                    .build()
                    .expect("could not build message")
            };
            session.publish(message).expect("message to be sent");
            sleep(Duration::new(1, 0));
        }

        let sleep_duration = Duration::new(10, 0);
        println!("Sleeping for {:?} before exiting", sleep_duration);
        sleep(sleep_duration);

        let sub_result = session.unsubscribe(topic);
        println!("Unsubscribed from {} topic", topic);
        assert!(sub_result.is_ok());
    }

    #[test]
    #[ignore]
    fn it_subscribes_and_listens() {
        let solace_context = SolContext::new(SolaceLogLevel::Warning)
            .map_err(|_| SessionError::InitializationFailure)
            .unwrap();
        println!("Context created");
        let session = create_print_session(&solace_context).unwrap();

        let topic = "try-me";
        println!("Session created");

        let sub_result = session.subscribe(topic);
        assert!(sub_result.is_ok());
        println!("Subscribed to {} topic", topic);

        let sleep_duration = Duration::new(60, 0);
        println!("Sleeping for {:?}", sleep_duration);
        sleep(sleep_duration);

        let sub_result = session.unsubscribe(topic);
        assert!(sub_result.is_ok());
        println!("Unsubscribed from {} topic", topic);
    }
}
