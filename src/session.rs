mod error;
mod util;

use crate::context::RawContext;
use crate::event::SessionEvent;
use crate::message::{InboundMessage, Message, OutboundMessage};
use crate::{ContextError, SolClientReturnCode, SolaceLogLevel};
use error::SessionError;
use num_traits::FromPrimitive;
use solace_rs_sys as ffi;
use std::ffi::CString;
use std::ptr;
use std::sync::Arc;
use tracing::warn;
use util::{on_event_trampoline, on_message_trampoline};

type Result<T> = std::result::Result<T, SessionError>;

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

    pub fn session<H, V, U, P, M, E>(
        &self,
        host_name: H,
        vpn_name: V,
        username: U,
        password: P,
        on_message: Option<M>,
        on_event: Option<E>,
    ) -> Result<Session>
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

        let session_create_result = unsafe {
            ffi::solClient_session_create(
                session_props,
                self.raw.ctx,
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
            Ok(Session {
                _session_pt: session_pt,
                context: self.clone(),
            })
        } else {
            Err(SessionError::ConnectionFailure)
        }
    }
}

pub struct Session {
    // Pointer to session
    // This pointer must never be allowed to leave the struct
    pub(crate) _session_pt: ffi::solClient_opaqueSession_pt,
    // The `context` field is never accessed, but implicitly does
    // reference counting via the `Drop` trait.
    #[allow(dead_code)]
    context: Context,
}

unsafe impl Send for Session {}
unsafe impl Sync for Session {}

impl Session {
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

impl Drop for Session {
    fn drop(&mut self) {
        let session_free_result = unsafe { ffi::solClient_session_destroy(&mut self._session_pt) };
        if SolClientReturnCode::from_i32(session_free_result) != Some(SolClientReturnCode::Ok) {
            warn!("session was not dropped properly");
        }
    }
}
