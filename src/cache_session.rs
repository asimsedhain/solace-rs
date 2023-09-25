use std::{
    ffi::CString,
    ops::{Deref, DerefMut},
    ptr,
};

use num_traits::FromPrimitive;
use solace_rs_sys as ffi;

use crate::{Session, SessionError, SolClientReturnCode};

pub struct CacheSession {
    // Pointer to session
    // This pointer must never be allowed to leave the struct
    pub(crate) _cache_session_pt: ffi::solClient_opaqueCacheSession_pt,
    pub(crate) session: Session,
}

impl Deref for CacheSession {
    type Target = Session;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl DerefMut for CacheSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}

impl CacheSession {
    pub(crate) fn new<N>(
        session: Session,
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

        let cache_session_props = [
            ffi::SOLCLIENT_CACHESESSION_PROP_CACHE_NAME.as_ptr(),
            c_cache_name.as_ptr() as *const u8,
            ffi::SOLCLIENT_CACHESESSION_PROP_DEFAULT_MAX_MSGS.as_ptr(),
            c_max_message.as_ptr() as *const u8,
            ffi::SOLCLIENT_CACHESESSION_PROP_MAX_AGE.as_ptr(),
            c_max_age.as_ptr() as *const u8,
            ffi::SOLCLIENT_CACHESESSION_PROP_REQUESTREPLY_TIMEOUT_MS.as_ptr(),
            c_timeout_ms.as_ptr() as *const u8,
            ptr::null(),
        ]
        .as_mut_ptr() as *mut *const i8;

        let mut cache_session_pt: ffi::solClient_opaqueCacheSession_pt = ptr::null_mut();

        let cache_create_result = unsafe {
            ffi::solClient_session_createCacheSession(
                cache_session_props,
                session._session_pt,
                &mut cache_session_pt,
            )
        };

        if SolClientReturnCode::from_i32(cache_create_result) != Some(SolClientReturnCode::Ok) {
            return Err(SessionError::InitializationFailure);
        }

        Ok(CacheSession {
            session,
            _cache_session_pt: cache_session_pt,
        })
    }

    pub fn blocking_cache_request<T>(self, topic: T, request_id: u64) -> Result<(), SessionError>
    where
        T: Into<Vec<u8>>,
    {
        let c_topic = CString::new(topic)?;
        let flags = ffi::SOLCLIENT_CACHEREQUEST_FLAGS_LIVEDATA_FLOWTHRU;

        let request_result = unsafe {
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

        if SolClientReturnCode::from_i32(request_result) != Some(SolClientReturnCode::Ok) {
            return Err(SessionError::CacheRequestFailure);
        }

        Ok(())
    }
}
