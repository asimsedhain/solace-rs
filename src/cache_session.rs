use std::ops::{Deref, DerefMut};

use solace_rs_sys as ffi;

use crate::{Session, SessionError};

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

// TODO
impl CacheSession {
    pub fn send_cache_request<T>(topic: T, request_id: u64) -> Result<(), SessionError>
    where
        T: Into<Vec<u8>>,
    {
        todo!()
    }
}
