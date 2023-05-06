use std::fmt;
pub mod solace;
pub mod context;
pub mod session;

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


