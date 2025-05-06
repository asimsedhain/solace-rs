pub mod builder;
pub mod event;

pub use builder::{SessionBuilder, SessionBuilderError};
pub use event::SessionEvent;

use crate::cache_session::CacheSession;
use crate::context::Context;
use crate::endpoint_props::EndpointProps;
use crate::flow::builder::FlowBuilder;
use crate::message::{InboundMessage, Message, OutboundMessage};
use crate::util::get_last_error_info;
use crate::SessionError;
use crate::SolClientReturnCode;
use solace_rs_sys::{self as ffi, solClient_opaqueMsg_pt};
use std::ffi::CString;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use tracing::warn;

type Result<T> = std::result::Result<T, SessionError>;

pub struct Session<
    'session,
    M: FnMut(InboundMessage) + Send + 'session,
    E: FnMut(SessionEvent) + Send + 'session,
> {
    pub(crate) lifetime: PhantomData<&'session ()>,

    // Pointer to session
    // This pointer must never be allowed to leave the struct
    pub(crate) _session_ptr: ffi::solClient_opaqueSession_pt,
    // The `context` field is never accessed, but implicitly does
    // reference counting via the `Drop` trait.
    #[allow(dead_code)]
    pub(crate) context: Context,

    // These fields are used to store the fn callback. The mutable reference to this fn is passed to the FFI library,
    #[allow(dead_code, clippy::redundant_allocation)]
    _msg_fn_ptr: Option<Box<Box<M>>>,
    #[allow(dead_code, clippy::redundant_allocation)]
    _event_fn_ptr: Option<Box<Box<E>>>,
}

unsafe impl<M: FnMut(InboundMessage) + Send, E: FnMut(SessionEvent) + Send> Send
    for Session<'_, M, E>
{
}

impl<'session, M: FnMut(InboundMessage) + Send, E: FnMut(SessionEvent) + Send>
    Session<'session, M, E>
{
    pub fn publish(&self, message: OutboundMessage) -> Result<()> {
        let send_message_raw_rc = unsafe {
            ffi::solClient_session_sendMsg(self._session_ptr, message.get_raw_message_ptr())
        };

        let rc = SolClientReturnCode::from_raw(send_message_raw_rc);
        if !rc.is_ok() {
            let subcode = get_last_error_info();
            return Err(SessionError::PublishError(rc, subcode));
        }

        Ok(())
    }

    pub fn subscribe<T>(&self, topic: T) -> Result<()>
    where
        T: Into<Vec<u8>>,
    {
        let c_topic = CString::new(topic)?;
        let subscription_raw_rc =
            unsafe { ffi::solClient_session_topicSubscribe(self._session_ptr, c_topic.as_ptr()) };

        let rc = SolClientReturnCode::from_raw(subscription_raw_rc);

        if !rc.is_ok() {
            let subcode = get_last_error_info();
            return Err(SessionError::SubscriptionFailure(
                c_topic.to_string_lossy().into_owned(),
                rc,
                subcode,
            ));
        }
        Ok(())
    }

    pub fn unsubscribe<T>(&self, topic: T) -> Result<()>
    where
        T: Into<Vec<u8>>,
    {
        let c_topic = CString::new(topic)?;
        let subscription_raw_rc =
            unsafe { ffi::solClient_session_topicUnsubscribe(self._session_ptr, c_topic.as_ptr()) };

        let rc = SolClientReturnCode::from_raw(subscription_raw_rc);

        if !rc.is_ok() {
            let subcode = get_last_error_info();
            return Err(SessionError::UnsubscriptionFailure(
                c_topic.to_string_lossy().into_owned(),
                rc,
                subcode,
            ));
        }
        Ok(())
    }

    pub fn request(
        &self,
        message: OutboundMessage,
        timeout_ms: NonZeroU32,
    ) -> Result<InboundMessage> {
        let mut reply_ptr: solClient_opaqueMsg_pt = std::ptr::null_mut();

        let rc = unsafe {
            ffi::solClient_session_sendRequest(
                self._session_ptr,
                message.get_raw_message_ptr(),
                &mut reply_ptr,
                timeout_ms.into(),
            )
        };

        let rc = SolClientReturnCode::from_raw(rc);

        if !rc.is_ok() {
            // reply_ptr is always set to null if rc is not Ok
            // https://docs.solace.com/API-Developer-Online-Ref-Documentation/c/sol_client_8h.html#ac00adf1a9301ebe67fd0790523d5a44b
            debug_assert!(reply_ptr.is_null());

            let subcode = get_last_error_info();
            return Err(SessionError::RequestError(rc, subcode));
        }

        debug_assert!(!reply_ptr.is_null());

        let reply = InboundMessage::from(reply_ptr);

        Ok(reply)
    }

    pub fn cache_session<N>(
        self,
        cache_name: N,
        max_message: Option<u64>,
        max_age: Option<u64>,
        timeout_ms: Option<u64>,
    ) -> Result<CacheSession<'session, M, E>>
    where
        N: Into<Vec<u8>>,
    {
        CacheSession::new(self, cache_name, max_message, max_age, timeout_ms)
    }

    pub fn disconnect(self) -> Result<()> {
        let rc = unsafe { ffi::solClient_session_disconnect(self._session_ptr) };

        let rc = SolClientReturnCode::from_raw(rc);

        if !rc.is_ok() {
            let subcode = get_last_error_info();
            return Err(SessionError::DisconnectError(rc, subcode));
        }
        Ok(())
    }

    pub fn endpoint_provision(
        &self,
        endpoint_props: EndpointProps,
        ignore_already_exists_error: bool,
    ) -> Result<()> {
        let mut flag = ffi::SOLCLIENT_PROVISION_FLAGS_WAITFORCONFIRM;
        if ignore_already_exists_error {
            flag |= ffi::SOLCLIENT_PROVISION_FLAGS_IGNORE_EXIST_ERRORS;
        }

        let rc = unsafe {
            let mut props_raw = endpoint_props.to_raw();
            ffi::solClient_session_endpointProvision(
                props_raw.as_mut_ptr(),
                self._session_ptr,
                flag,
                std::ptr::null_mut(),
                // deprecated params
                std::ptr::null_mut(),
                0,
            )
        };

        let rc = SolClientReturnCode::from_raw(rc);

        if !rc.is_ok() {
            let subcode = get_last_error_info();
            return Err(SessionError::EndpointProvisionError(rc, subcode));
        }
        Ok(())
    }

    pub fn endpoint_deprovision(
        &self,
        endpoint_props: EndpointProps,
        ignore_already_exists_error: bool,
    ) -> Result<()> {
        let mut flag = ffi::SOLCLIENT_PROVISION_FLAGS_WAITFORCONFIRM;
        if ignore_already_exists_error {
            flag |= ffi::SOLCLIENT_PROVISION_FLAGS_IGNORE_EXIST_ERRORS;
        }

        let rc = unsafe {
            ffi::solClient_session_endpointDeprovision(
                endpoint_props.to_raw().as_mut_ptr(),
                self._session_ptr,
                flag,
                std::ptr::null_mut(),
            )
        };

        let rc = SolClientReturnCode::from_raw(rc);

        if !rc.is_ok() {
            let subcode = get_last_error_info();
            return Err(SessionError::EndpointDeprovisionError(rc, subcode));
        }
        Ok(())
    }

    pub fn flow_builder<'builder, OnMessage, OnEvent>(
        &'builder self,
    ) -> FlowBuilder<'builder, 'session, M, E, OnMessage, OnEvent> {
        FlowBuilder::new(self)
    }
}

impl<M: FnMut(InboundMessage) + Send, E: FnMut(SessionEvent) + Send> Drop for Session<'_, M, E> {
    fn drop(&mut self) {
        let session_free_result = unsafe { ffi::solClient_session_destroy(&mut self._session_ptr) };
        let rc = SolClientReturnCode::from_raw(session_free_result);

        if !rc.is_ok() {
            warn!("session was not dropped properly. {rc}");
        }
    }
}
