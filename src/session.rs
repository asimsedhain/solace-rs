use crate::context::SolContext;
use crate::solace::ffi;
use crate::{Result, SolaceError};
use std::ffi::CString;
use std::ptr;

pub struct SolSession {
    // Pointer to session
    // This pointer must never be allowed to leave the struct
    _session_pt: ffi::solClient_opaqueSession_pt,
}

// TODO
// These are temp callbacks
// Implement callbacks that can be passed in
extern "C" fn on_event(
    opaque_session_p: ffi::solClient_opaqueSession_pt,
    event_info_p: ffi::solClient_session_eventCallbackInfo_pt,
    user_p: *mut ::std::os::raw::c_void,
) {
    println!("some event recienved");
}

extern "C" fn on_message(
    opaque_session_p: ffi::solClient_opaqueSession_pt,
    msg_p: ffi::solClient_opaqueMsg_pt,
    user_p: *mut ::std::os::raw::c_void,
) -> ffi::solClient_rxMsgCallback_returnCode_t {
    unsafe {
        ffi::solClient_msg_dump(msg_p, ptr::null_mut(), 0);
    }
    println!("Message callback");

    ffi::solClient_rxMsgCallback_returnCode_SOLCLIENT_CALLBACK_OK
}

impl SolSession {
    pub fn new<T>(
        host_name: T,
        vpn_name: T,
        username: T,
        password: T,
        // since the solace_context has the threading library
        // might be good to get the context entirely instead of a reference to the context
        context: &SolContext,
        //on_message: Fn,
        //on_event: Fn,
    ) -> Result<Self>
    where
        T: Into<Vec<u8>>,
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
                    callback_p: Some(on_message),
                    user_p: ptr::null_mut(),
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

        if session_create_result != ffi::solClient_returnCode_SOLCLIENT_OK {
            panic!("Could not initialize solace session");
            //return Err(SolaceError);
        }

        let connection_result;
        unsafe {
            connection_result = ffi::solClient_session_connect(session_pt);
        }
        if connection_result == ffi::solClient_returnCode_SOLCLIENT_OK {
            Ok(SolSession {
                _session_pt: session_pt,
            })
        } else {
            println!("Solace did not connect properly");
            println!("Returned: {}", connection_result);

            Err(SolaceError)
        }
    }

    #[allow(dead_code)]
    pub fn publish(&self) -> Result<()> {
        todo!();
    }

    #[allow(dead_code)]
    pub fn subscribe() -> Result<()> {
        todo!();
    }

    #[allow(dead_code)]
    pub fn unsubscribe() -> Result<()> {
        todo!();
    }
}

impl Drop for SolSession {
    fn drop(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SolaceLogLevel;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn it_works() {
        let solace_context = SolContext::new(SolaceLogLevel::Info).unwrap();
        println!("Context created");
        let host_name = "tcp://localhost:55554";
        let vpn_name = "default";
        let username = "default";
        let password = "";

        let solace_session =
            SolSession::new(host_name, vpn_name, username, password, &solace_context);
        assert!(solace_session.is_ok());

        println!("Session created");

        sleep(Duration::new(120, 0));

        assert_eq!(true, true);
    }
}
