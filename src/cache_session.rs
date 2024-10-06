use std::{
    ffi::CString,
    ops::{Deref, DerefMut},
    ptr,
};

use solace_rs_sys as ffi;
use tracing::warn;

use crate::{
    message::InboundMessage, session::SessionEvent, util::get_last_error_info, Session,
    SessionError, SolClientReturnCode,
};

pub struct CacheSession<
    'session,
    M: FnMut(InboundMessage) + Send + 'session,
    E: FnMut(SessionEvent) + Send + 'session,
> {
    // Pointer to session
    // This pointer must never be allowed to leave the struct
    pub(crate) _cache_session_pt: ffi::solClient_opaqueCacheSession_pt,
    pub(crate) session: Session<'session, M, E>,
}

unsafe impl<M: FnMut(InboundMessage) + Send, E: FnMut(SessionEvent) + Send> Send
    for CacheSession<'_, M, E>
{
}
unsafe impl<M: FnMut(InboundMessage) + Send, E: FnMut(SessionEvent) + Send> Sync
    for CacheSession<'_, M, E>
{
}

impl<'session, M: FnMut(InboundMessage) + Send, E: FnMut(SessionEvent) + Send> Deref
    for CacheSession<'session, M, E>
{
    type Target = Session<'session, M, E>;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl<M: FnMut(InboundMessage) + Send, E: FnMut(SessionEvent) + Send> Drop
    for CacheSession<'_, M, E>
{
    fn drop(&mut self) {
        let session_free_result =
            unsafe { ffi::solClient_cacheSession_destroy(&mut self._cache_session_pt) };
        let rc = SolClientReturnCode::from_raw(session_free_result);

        if !rc.is_ok() {
            warn!("cache session was not dropped properly. {rc}");
        }
    }
}

impl<M: FnMut(InboundMessage) + Send, E: FnMut(SessionEvent) + Send> DerefMut
    for CacheSession<'_, M, E>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}

impl<'session, M: FnMut(InboundMessage) + Send, E: FnMut(SessionEvent) + Send>
    CacheSession<'session, M, E>
{
    pub(crate) fn new<N>(
        session: Session<'session, M, E>,
        cache_name: N,
        max_message: Option<u64>,
        max_age: Option<u64>,
        timeout_ms: Option<u64>,
    ) -> Result<Self, SessionError>
    where
        N: Into<Vec<u8>>,
    {
        let c_cache_name = CString::new(cache_name)?;
        let c_max_message = CString::new(max_message.unwrap_or(1).to_string())?;
        let c_max_age = CString::new(max_age.unwrap_or(0).to_string())?;
        let c_timeout_ms = CString::new(timeout_ms.unwrap_or(10000).to_string())?;

        // Note: Needs to live long enough for the values to be copied
        let mut cache_session_props = [
            ffi::SOLCLIENT_CACHESESSION_PROP_CACHE_NAME.as_ptr() as *const i8,
            c_cache_name.as_ptr(),
            ffi::SOLCLIENT_CACHESESSION_PROP_DEFAULT_MAX_MSGS.as_ptr() as *const i8,
            c_max_message.as_ptr(),
            ffi::SOLCLIENT_CACHESESSION_PROP_MAX_AGE.as_ptr() as *const i8,
            c_max_age.as_ptr(),
            ffi::SOLCLIENT_CACHESESSION_PROP_REQUESTREPLY_TIMEOUT_MS.as_ptr() as *const i8,
            c_timeout_ms.as_ptr(),
            ptr::null(),
        ];

        let mut cache_session_pt: ffi::solClient_opaqueCacheSession_pt = ptr::null_mut();

        let cache_create_raw_result = unsafe {
            ffi::solClient_session_createCacheSession(
                cache_session_props.as_mut_ptr(),
                session._session_ptr,
                &mut cache_session_pt,
            )
        };

        let rc = SolClientReturnCode::from_raw(cache_create_raw_result);

        if !rc.is_ok() {
            let subcode = get_last_error_info();
            return Err(SessionError::InitializationFailure(rc, subcode));
        }

        Ok(CacheSession {
            session,
            _cache_session_pt: cache_session_pt,
        })
    }

    pub fn blocking_cache_request<T>(
        &self,
        topic: T,
        request_id: u64,
        subscribe: bool,
    ) -> Result<(), SessionError>
    where
        T: Into<Vec<u8>>,
    {
        let c_topic = CString::new(topic)?;

        let flags = if subscribe {
            ffi::SOLCLIENT_CACHEREQUEST_FLAGS_LIVEDATA_FLOWTHRU
                & ffi::SOLCLIENT_CACHEREQUEST_FLAGS_NO_SUBSCRIBE
        } else {
            ffi::SOLCLIENT_CACHEREQUEST_FLAGS_LIVEDATA_FLOWTHRU
        };

        let rc = unsafe {
            ffi::solClient_cacheSession_sendCacheRequest(
                self._cache_session_pt,
                c_topic.as_ptr(),
                request_id,
                None,
                ptr::null_mut(),
                flags,
                0,
            )
        };

        let rc = SolClientReturnCode::from_raw(rc);
        if !rc.is_ok() {
            let subcode = get_last_error_info();
            return Err(SessionError::CacheRequestFailure(rc, subcode));
        }

        Ok(())
    }
}
