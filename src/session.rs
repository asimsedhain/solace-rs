use crate::cache_session::CacheSession;
use crate::context::Context;
use crate::message::{Message, OutboundMessage};
use crate::SessionError;
use crate::SolClientReturnCode;
use num_traits::FromPrimitive;
use solace_rs_sys as ffi;
use std::ffi::CString;
use std::ptr;
use tracing::warn;

type Result<T> = std::result::Result<T, SessionError>;

pub struct Session {
    // Pointer to session
    // This pointer must never be allowed to leave the struct
    pub(crate) _session_pt: ffi::solClient_opaqueSession_pt,
    // The `context` field is never accessed, but implicitly does
    // reference counting via the `Drop` trait.
    #[allow(dead_code)]
    pub(crate) context: Context,
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

    // TODO
    pub fn cache_session<N>(
        self,
        cache_name: N,
        max_message: Option<u64>,
        max_age: Option<u64>,
        timeout_ms: Option<u64>,
    ) -> Result<CacheSession>
    where
        N: Into<Vec<u8>>,
    {
        let c_cache_name = CString::new(cache_name)?;

        let cache_session_props = [
            ffi::SOLCLIENT_CACHESESSION_PROP_CACHE_NAME.as_ptr(),
            c_cache_name.as_ptr() as *const u8,
            ptr::null(),
        ]
        .as_mut_ptr() as *mut *const i8;

        let mut cache_session_pt: ffi::solClient_opaqueCacheSession_pt = ptr::null_mut();

        let cache_create_result = unsafe {
            ffi::solClient_session_createCacheSession(
                cache_session_props,
                self._session_pt,
                &mut cache_session_pt,
            )
        };

        if SolClientReturnCode::from_i32(cache_create_result) != Some(SolClientReturnCode::Ok) {
            return Err(SessionError::InitializationFailure);
        }

        Ok(CacheSession {
            session: self,
            _cache_session_pt: cache_session_pt,
        })
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
