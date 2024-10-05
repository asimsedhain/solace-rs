use crate::session::builder::SessionBuilder;
use crate::session::builder::SessionBuilderError;
use crate::util::get_last_error_info;
use crate::Session;
use crate::{ContextError, SolClientReturnCode, SolaceLogLevel};
use solace_rs_sys as ffi;
use std::mem;
use std::ptr;
use std::sync::Once;
use tracing::warn;

use crate::message::InboundMessage;
use crate::session::SessionEvent;
use std::sync::Arc;
type Result<T> = std::result::Result<T, ContextError>;

pub(super) struct RawContext {
    // This pointer must never be allowed to leave the struct
    pub(crate) ctx: ffi::solClient_opaqueContext_pt,
}

static SOLACE_GLOBAL_INIT: Once = Once::new();
static mut SOLACE_GLOBAL_INIT_RC: i32 = 0;

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
        SOLACE_GLOBAL_INIT.call_once(|| {
            SOLACE_GLOBAL_INIT_RC =
                unsafe { ffi::solClient_initialize(log_level as u32, ptr::null_mut()) };
        });

        let rc = SolClientReturnCode::from_raw(SOLACE_GLOBAL_INIT_RC);

        if !rc.is_ok() {
            let subcode = get_last_error_info();
            return Err(ContextError::InitializationFailed(rc, subcode));
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
            let subcode = get_last_error_info();
            return Err(ContextError::InitializationFailed(rc, subcode));
        }
        Ok(Self { ctx })
    }
}

impl Drop for RawContext {
    fn drop(&mut self) {
        let return_code = unsafe { ffi::solClient_context_destroy(&mut self.ctx) };
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
    pub(super) raw: Arc<RawContext>,
}

impl Context {
    pub fn new(log_level: SolaceLogLevel) -> std::result::Result<Self, ContextError> {
        Ok(Self {
            raw: Arc::new(unsafe { RawContext::new(log_level) }?),
        })
    }

    pub fn session_builder<Host, Vpn, Username, Password, OnMessage, OnEvent>(
        &self,
    ) -> SessionBuilder<Host, Vpn, Username, Password, OnMessage, OnEvent> {
        SessionBuilder::new(self.clone())
    }

    pub fn session<'session, Host, Vpn, Username, Password, OnMessage, OnEvent>(
        &self,
        host_name: Host,
        vpn_name: Vpn,
        username: Username,
        password: Password,
        on_message: Option<OnMessage>,
        on_event: Option<OnEvent>,
    ) -> std::result::Result<Session<'session>, SessionBuilderError>
    where
        Host: Into<Vec<u8>>,
        Vpn: Into<Vec<u8>>,
        Username: Into<Vec<u8>>,
        Password: Into<Vec<u8>>,
        OnMessage: FnMut(InboundMessage) + Send + 'session,
        OnEvent: FnMut(SessionEvent) + Send + 'session,
    {
        let mut builder = SessionBuilder::new(self.clone())
            .host_name(host_name)
            .vpn_name(vpn_name)
            .username(username)
            .password(password);

        if let Some(on_message) = on_message {
            builder = builder.on_message(on_message);
        }

        if let Some(on_event) = on_event {
            builder = builder.on_event(on_event);
        }

        builder.build()
    }
}
