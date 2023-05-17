use std::fmt;
pub mod context;
pub mod session;
mod solace;
mod message;

use enum_primitive::*;
use solace::ffi;

#[derive(Debug, Clone)]
pub struct SolaceError;

type Result<T> = std::result::Result<T, SolaceError>;

impl fmt::Display for SolaceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Solace Error Occured!")
    }
}

enum_from_primitive! {
    #[derive(Debug, PartialEq)]
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
    #[derive(Debug, PartialEq)]
    #[repr(i32)]
    pub enum SolaceReturnCode {
        OK = ffi::solClient_returnCode_SOLCLIENT_OK,
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
