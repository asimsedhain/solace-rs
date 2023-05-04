use std::fmt;
use std::mem;
use std::ptr;
pub mod solace;

#[macro_use]
extern crate enum_primitive;

use enum_primitive::*;
use solace::ffi;

#[derive(Debug, Clone)]
struct SolaceError;

type Result<T> = std::result::Result<T, SolaceError>;

impl fmt::Display for SolaceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Solace Error Occured!")
    }
}

enum_from_primitive! {
#[derive(Debug, PartialEq)]
#[repr(u32)]
enum SolaceLogLevel {
    Critical = ffi::solClient_log_level_SOLCLIENT_LOG_CRITICAL,
    Error = ffi::solClient_log_level_SOLCLIENT_LOG_ERROR,
    Warning = ffi::solClient_log_level_SOLCLIENT_LOG_WARNING,
    Notice = ffi::solClient_log_level_SOLCLIENT_LOG_NOTICE,
    Info = ffi::solClient_log_level_SOLCLIENT_LOG_INFO,
    Debug = ffi::solClient_log_level_SOLCLIENT_LOG_DEBUG,
    }
}

struct SolContext {
    // This pointer must never be allowed to leave the struct
    ctx: ffi::solClient_opaqueContext_pt,
}

// Solace initializes global variables
// as such it is not safe to have multiple solaces libraries
// in the same project
impl SolContext {
    pub fn new(log_level: SolaceLogLevel) -> Result<Self> {
        //let null_ptr = &mut ptr::null::<i8>();
        let solace_initailization_result =
            unsafe { ffi::solClient_initialize(log_level as u32, ptr::null_mut()) };

        if solace_initailization_result != ffi::solClient_returnCode_SOLCLIENT_OK {
            panic!("Could not initialize solace client");
            //return Err(SolaceError);
        }
        let mut ctx: ffi::solClient_opaqueContext_pt = ptr::null_mut();
        let mut context_func: ffi::solClient_context_createFuncInfo_t =
            ffi::solClient_context_createFuncInfo {
                regFdInfo: ffi::solClient_context_createRegisterFdFuncInfo {
                    regFdFunc_p: None,
                    unregFdFunc_p: None,
                    user_p: ptr::null_mut(),
                },
            };

        let solace_context_result = unsafe {
            ffi::solClient_context_create(
                (&mut ffi::_solClient_contextPropsDefaultWithCreateThread) as *mut *const i8,
                &mut ctx,
                &mut context_func,
                mem::size_of::<ffi::solClient_context_createRegisterFdFuncInfo>(),
            )
        };
        if solace_context_result != ffi::solClient_returnCode_SOLCLIENT_OK {
            panic!("Could not initialize solace context");
            //return Err(SolaceError);
        }
        return Ok(Self { ctx });
    }
}

impl Drop for SolContext {
    fn drop(&mut self) {
        let return_code = unsafe { ffi::solClient_cleanup() };
        if return_code != ffi::solClient_returnCode_SOLCLIENT_OK {
            panic!("Solace context did not drop properly");
        }
    }
}

struct SolSession {
    // Pointer to session
    // This pointer must never be allowed to leave the struct
    session_pt: ffi::solClient_opaqueSession_pt,
}

// TODO
// These are temp callbacks
// Implement callbacks that can be passed in
extern "C" fn on_event(
    opaque_session_p: ffi::solClient_opaqueSession_pt,
    event_info_p: ffi::solClient_session_eventCallbackInfo_pt,
    user_p: *mut ::std::os::raw::c_void,
) {
    println!("Event callback");
}

extern "C" fn on_message(
    opaque_session_p: ffi::solClient_opaqueSession_pt,
    msg_p: ffi::solClient_opaqueMsg_pt,
    user_p: *mut ::std::os::raw::c_void,
) -> ffi::solClient_rxMsgCallback_returnCode_t {
    println!("Message callback");
    return ffi::solClient_rxMsgCallback_returnCode_SOLCLIENT_CALLBACK_OK;
}

impl SolSession {
    pub fn new(
        host_name: String,
        vpn_name: String,
        username: String,
        password: String,
        context: &SolContext,
        //on_message: Fn,
        //on_event: Fn,
    ) -> Result<Self> {
        /* Session */
        //solClient_opaqueSession_pt session_p;
        //solClient_session_createFuncInfo_t sessionFuncInfo = SOLCLIENT_SESSION_CREATEFUNC_INITIALIZER;

        // Converting props and storing them session props
        let c_host_name = std::ffi::CString::new(host_name).expect("Invalid host_name");
        let c_vpn_name = std::ffi::CString::new(vpn_name).expect("Invalid vpn_name");
        let c_username = std::ffi::CString::new(username).expect("Invalid username");
        let c_password = std::ffi::CString::new(password).expect("Invalid password");

        let session_props = [
            ptr::null() as *const u8,
            ffi::SOLCLIENT_SESSION_PROP_HOST.as_ptr(),
            c_host_name.as_ptr() as *const u8,
            ffi::SOLCLIENT_SESSION_PROP_VPN_NAME.as_ptr(),
            c_vpn_name.as_ptr() as *const u8,
            ffi::SOLCLIENT_SESSION_PROP_USERNAME.as_ptr(),
            c_username.as_ptr() as *const u8,
            ffi::SOLCLIENT_SESSION_PROP_PASSWORD.as_ptr(),
            c_password.as_ptr() as *const u8,
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
                context.ctx,
                &mut session_pt,
                &mut session_func_info,
                std::mem::size_of::<ffi::solClient_session_createFuncInfo_t>(),
            )
        };

        if session_create_result != ffi::solClient_returnCode_SOLCLIENT_OK {
            panic!("Could not initialize solace session");
            //return Err(SolaceError);
        }

        Ok(SolSession { session_pt })
    }

    pub fn publish(&self) -> Result<()> {
        Ok(())
    }

    pub fn subscribe() -> Result<()> {
        Ok(())
    }

    pub fn unsubscribe() -> Result<()> {
        Ok(())
    }
}

impl Drop for SolSession {
    fn drop(&mut self) {
        
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let solace_context = SolContext::new(SolaceLogLevel::Info).unwrap();
        println!("Context created");
        let host_name = format!("tcps://localhost:8008");
        let vpn_name = format!("default");
        let username = format!("default");
        let password = format!("");

        let _solace_session =
            SolSession::new(host_name, vpn_name, username, password, &solace_context);
        println!("Session created");

        assert_eq!(true, true);
    }
}
