pub mod context;
pub mod event;
pub mod message;
pub mod session;
pub(crate) mod util;

use enum_primitive::*;
use solace_rs_sys as ffi;
use std::fmt;
use thiserror::Error;

pub use crate::context::Context;
pub use crate::session::Session;

#[derive(Debug, Clone)]
pub struct SolaceError;

type Result<T> = std::result::Result<T, SolaceError>;

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
    #[derive(Debug, PartialEq, Eq)]
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

#[derive(Error, Debug)]
pub enum ContextError {
    #[error("context thread failed to initialize")]
    InitializationFailed,
}


#[derive(Error, Debug)]
pub enum SessionError {
    #[error("session receieved invalid argument")]
    InvalidArgs(#[from] std::ffi::NulError),
    #[error("session failed to connect")]
    ConnectionFailure,
    #[error("session failed to initialize")]
    InitializationFailure,
    #[error("session failed to subscribe on topic: {0}")]
    SubscriptionFailure(String),
    #[error("session failed to unsubscribe on topic: {0}")]
    UnsubscriptionFailure(String),
}
