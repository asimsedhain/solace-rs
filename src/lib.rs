pub mod cache_session;
pub mod context;
pub mod message;
pub mod session;
pub(crate) mod util;

use enum_primitive::*;
use solace_rs_sys as ffi;
use std::fmt::{self};
use thiserror::Error;

pub use crate::context::Context;
pub use crate::session::Session;

// Generic error
#[derive(Debug, Clone)]
pub struct SolaceError;

impl fmt::Display for SolaceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Solace Error Occured!")
    }
}

enum_from_primitive! {
    #[derive(Debug, PartialEq, Eq)]
    #[repr(u32)]
    pub enum SolaceLogLevel {
        Critical = ffi::solClient_log_level_SOLCLIENT_LOG_CRITICAL,
        Error = ffi::solClient_log_level_SOLCLIENT_LOG_ERROR,
        Warning = ffi::solClient_log_level_SOLCLIENT_LOG_WARNING,
        Notice = ffi::solClient_log_level_SOLCLIENT_LOG_NOTICE,
        Info = ffi::solClient_log_level_SOLCLIENT_LOG_INFO,
        Debug = ffi::solClient_log_level_SOLCLIENT_LOG_DEBUG,
    }
}

enum_from_primitive! {
    #[derive(PartialEq, Eq)]
    #[repr(i32)]
    pub enum SolClientReturnCode {
        Ok=ffi::solClient_returnCode_SOLCLIENT_OK,
        WouldBlock=ffi::solClient_returnCode_SOLCLIENT_WOULD_BLOCK,
        InProgress=ffi::solClient_returnCode_SOLCLIENT_IN_PROGRESS,
        NotReady=ffi::solClient_returnCode_SOLCLIENT_NOT_READY,
        EndOfStream=ffi::solClient_returnCode_SOLCLIENT_EOS,
        NotFound=ffi::solClient_returnCode_SOLCLIENT_NOT_FOUND,
        NoEvent=ffi::solClient_returnCode_SOLCLIENT_NOEVENT,
        Incomplete=ffi::solClient_returnCode_SOLCLIENT_INCOMPLETE,
        Rollback=ffi::solClient_returnCode_SOLCLIENT_ROLLBACK,
        Fail=ffi::solClient_returnCode_SOLCLIENT_FAIL,
    }
}

impl std::fmt::Display for SolClientReturnCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolClientReturnCode::Ok => write!(f, "Ok - The API call was successful."),
            SolClientReturnCode::WouldBlock => write!(
                f,
                "WouldBlock - The API call would block, but non-blocking was requested."
            ),
            SolClientReturnCode::InProgress => write!(
                f,
                "InProgress - An API call is in progress (non-blocking mode)."
            ),
            SolClientReturnCode::NotReady => write!(f, "NotReady - The API could not complete as an object is not ready (for example, the Session is not connected)."),
            SolClientReturnCode::EndOfStream => write!(f, "EndOfStream - A getNext on a structured container returned End-of-Stream."),
            SolClientReturnCode::NotFound => write!(f, "NotFound - A get for a named field in a MAP was not found in the MAP."),
            SolClientReturnCode::NoEvent => write!(f, "NoEvent - solClient_context_processEventsWait returns this if wait is zero and there is no event to process"),
            SolClientReturnCode::Incomplete => write!(f, "Incomplete - The API call completed some, but not all, of the requested function."),
            SolClientReturnCode::Rollback => write!(f, "Rollback - solClient_transactedSession_commit returns this when the transaction has been rolled back."),
            SolClientReturnCode::Fail => write!(f, "Fail - The API call failed."),
        }
    }
}

impl std::fmt::Debug for SolClientReturnCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl SolClientReturnCode {
    pub(crate) fn from_raw(value: i32) -> Self {
        match Self::from_i32(value) {
            Some(rc) => rc,
            None => Self::Fail,
        }
    }

    pub fn is_ok(&self) -> bool {
        *self == Self::Ok
    }
}

#[derive(Error, Debug)]
pub enum ContextError {
    #[error("context thread failed to initialize. SolClient return code: {0:?}")]
    InitializationFailed(SolClientReturnCode),
}

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("session receieved arguments with null value")]
    InvalidArgsNulError(#[from] std::ffi::NulError),
    #[error("session failed to connect. SolClient return code: {0}")]
    ConnectionFailure(SolClientReturnCode),
    #[error("session failed to initialize. SolClient return code: {0}")]
    InitializationFailure(SolClientReturnCode),
    #[error("session failed to subscribe on topic. SolClient return code: {0}")]
    SubscriptionFailure(String, SolClientReturnCode),
    #[error("session failed to unsubscribe on topic. SolClient return code: {0}")]
    UnsubscriptionFailure(String, SolClientReturnCode),
    #[error("cache request failed")]
    CacheRequestFailure,
    #[error("could not publish message. SolClient return code: {0}")]
    PublishError(SolClientReturnCode),
}
