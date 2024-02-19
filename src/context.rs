use crate::Session;
use crate::SessionError;
use crate::{ContextError, SolClientReturnCode, SolaceLogLevel};
use solace_rs_sys as ffi;
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use tracing::warn;

use crate::event::SessionEvent;
use crate::message::InboundMessage;
use crate::util::{on_event_trampoline, on_message_trampoline};
use std::ffi::CString;
use std::sync::Arc;
type Result<T> = std::result::Result<T, ContextError>;

pub struct RawContext {
    // This pointer must never be allowed to leave the struct
    pub(crate) ctx: ffi::solClient_opaqueContext_pt,
}

impl RawContext {
    /// .
    /// Raw solace context that wraps around the c context
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    ///
    /// # Safety
    /// Context initializes global variables so it is not safe to have multiple solace contexts.
    /// .
    pub unsafe fn new(log_level: SolaceLogLevel) -> Result<Self> {
        let solace_initailization_raw_rc =
            unsafe { ffi::solClient_initialize(log_level as u32, ptr::null_mut()) };

        let rc = SolClientReturnCode::from_raw(solace_initailization_raw_rc);

        if !rc.is_ok() {
            return Err(ContextError::InitializationFailed(rc));
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

        // enable context thread
        let mut conext_props: [*const i8; 3] = [
            solace_rs_sys::SOLCLIENT_CONTEXT_PROP_CREATE_THREAD.as_ptr() as *const i8,
            solace_rs_sys::SOLCLIENT_PROP_ENABLE_VAL.as_ptr() as *const i8,
            ptr::null(),
        ];

        let solace_context_raw_rc = unsafe {
            ffi::solClient_context_create(
                conext_props.as_mut_ptr(),
                &mut ctx,
                &mut context_func,
                mem::size_of::<ffi::solClient_context_createRegisterFdFuncInfo>(),
            )
        };

        let rc = SolClientReturnCode::from_raw(solace_context_raw_rc);

        if !rc.is_ok() {
            return Err(ContextError::InitializationFailed(rc));
        }
        Ok(Self { ctx })
    }
}

impl Drop for RawContext {
    fn drop(&mut self) {
        // TODO
        // shifts cleanup to be context specific
        // only clean up globally when all the contexts have died
        let return_code = unsafe { ffi::solClient_cleanup() };
        if return_code != ffi::solClient_returnCode_SOLCLIENT_OK {
            warn!("Solace context did not drop properly");
        }
    }
}

unsafe impl Send for RawContext {}

unsafe impl Sync for RawContext {}

/// Handle for a Solace context, used to create sessions.
///
/// It is thread safe, and can be safely cloned and shared. Each clone
/// references the same underlying C context. Internally, an `Arc` is
/// used to implement this in a threadsafe way.
///
/// Important: Only initialize one context per application as it initializes global variables upon
/// creation.
/// Also note that this binding deviates from the C API in that each
/// session created from a context initially owns a clone of that
/// context.
///
///
#[derive(Clone)]
pub struct Context {
    raw: Arc<RawContext>,
}

impl Context {
    pub fn new(log_level: SolaceLogLevel) -> std::result::Result<Self, ContextError> {
        Ok(Self {
            raw: Arc::new(unsafe { RawContext::new(log_level) }?),
        })
    }

    pub fn session<'session, H, V, U, P, M, E>(
        &self,
        host_name: H,
        vpn_name: V,
        username: U,
        password: P,
        on_message: Option<M>,
        on_event: Option<E>,
    ) -> std::result::Result<Session<'session>, SessionError>
    where
        H: Into<Vec<u8>>,
        V: Into<Vec<u8>>,
        U: Into<Vec<u8>>,
        P: Into<Vec<u8>>,
        M: FnMut(InboundMessage) + Send + 'session,
        E: FnMut(SessionEvent) + Send + 'session,
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
        // Note: Needs to live long enough for the values to be copied
        let mut session_props = [
            ffi::SOLCLIENT_SESSION_PROP_HOST.as_ptr() as *const i8,
            c_host_name.as_ptr(),
            ffi::SOLCLIENT_SESSION_PROP_VPN_NAME.as_ptr() as *const i8,
            c_vpn_name.as_ptr(),
            ffi::SOLCLIENT_SESSION_PROP_USERNAME.as_ptr() as *const i8,
            c_username.as_ptr(),
            ffi::SOLCLIENT_SESSION_PROP_PASSWORD.as_ptr() as *const i8,
            c_password.as_ptr(),
            ffi::SOLCLIENT_SESSION_PROP_CONNECT_BLOCKING.as_ptr() as *const i8,
            ffi::SOLCLIENT_PROP_ENABLE_VAL.as_ptr() as *const i8,
            ptr::null(),
        ];

        let mut session_pt: ffi::solClient_opaqueSession_pt = ptr::null_mut();

        // Box::into_raw(Box::new(Box::new(f))) as *mut _
        // leaks memory
        // but without it, causes seg fault
        let (static_on_message_callback, user_on_message) = match on_message {
            Some(f) => (
                on_message_trampoline(&f),
                Box::into_raw(Box::new(Box::new(f))) as *mut _,
            ),
            _ => (None, ptr::null_mut()),
        };

        let (static_on_event_callback, user_on_event) = match on_event {
            Some(f) => (
                on_event_trampoline(&f),
                Box::into_raw(Box::new(Box::new(f))) as *mut _,
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
                    callback_p: static_on_event_callback,
                    user_p: user_on_event,
                },
                rxMsgInfo: ffi::solClient_session_createRxMsgCallbackFuncInfo {
                    callback_p: static_on_message_callback,
                    user_p: user_on_message,
                },
            };

        let session_create_raw_rc = unsafe {
            ffi::solClient_session_create(
                session_props.as_mut_ptr(),
                self.raw.ctx,
                &mut session_pt,
                &mut session_func_info,
                std::mem::size_of::<ffi::solClient_session_createFuncInfo_t>(),
            )
        };

        let rc = SolClientReturnCode::from_raw(session_create_raw_rc);

        if !rc.is_ok() {
            return Err(SessionError::InitializationFailure(rc));
        }

        let connection_raw_rc = unsafe { ffi::solClient_session_connect(session_pt) };

        let rc = SolClientReturnCode::from_raw(connection_raw_rc);
        if rc.is_ok() {
            Ok(Session {
                _session_pt: session_pt,
                context: self.clone(),
                lifetime: PhantomData,
            })
        } else {
            Err(SessionError::ConnectionFailure(rc))
        }
    }
}
