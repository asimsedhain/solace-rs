use super::Message;
use crate::{Result, SolClientReturnCode, SolaceError};
use enum_primitive::*;
use solace_rs_sys as ffi;
use std::convert::From;
use std::ffi::CStr;
use std::ptr;
use std::time::{Duration, SystemTime};
use tracing::warn;

pub struct InboundMessage {
    _msg_ptr: ffi::solClient_opaqueMsg_pt,
}

unsafe impl Send for InboundMessage {}

impl Drop for InboundMessage {
    fn drop(&mut self) {
        let msg_free_result = unsafe { ffi::solClient_msg_free(&mut self._msg_ptr) };
        if SolClientReturnCode::from_i32(msg_free_result) != Some(SolClientReturnCode::Ok) {
            warn!("warning: message was not dropped properly");
        }
    }
}

impl From<ffi::solClient_opaqueMsg_pt> for InboundMessage {
    /// .
    ///
    /// # Safety
    ///
    /// From a valid owned pointer.
    /// No other alias should exist for this pointer
    /// InboundMessage will try to free the ptr when it is destroyed
    ///
    /// .
    fn from(ptr: ffi::solClient_opaqueMsg_pt) -> Self {
        Self { _msg_ptr: ptr }
    }
}

impl<'a> Message<'a> for InboundMessage {
    unsafe fn get_raw_message_ptr(&self) -> ffi::solClient_opaqueMsg_pt {
        self._msg_ptr
    }
}

impl InboundMessage {
    pub fn get_receive_timestamp(&self) -> Result<Option<SystemTime>> {
        let mut ts: i64 = 0;
        let op_result =
            unsafe { ffi::solClient_msg_getRcvTimestamp(self.get_raw_message_ptr(), &mut ts) };

        match SolClientReturnCode::from_i32(op_result) {
            Some(SolClientReturnCode::NotFound) => Ok(None),
            Some(SolClientReturnCode::Ok) => Ok(Some(
                SystemTime::UNIX_EPOCH + Duration::from_millis(ts.try_into().unwrap()),
            )),
            _ => Err(SolaceError),
        }
    }

    pub fn get_sender_id(&self) -> Result<Option<&str>> {
        let mut buffer = ptr::null();

        let msg_ops_result =
            unsafe { ffi::solClient_msg_getCorrelationId(self.get_raw_message_ptr(), &mut buffer) };

        match SolClientReturnCode::from_i32(msg_ops_result) {
            Some(SolClientReturnCode::Ok) => (),
            Some(SolClientReturnCode::NotFound) => return Ok(None),
            _ => return Err(SolaceError),
        }

        let c_str = unsafe { CStr::from_ptr(buffer) };

        let str = c_str.to_str().map_err(|_| SolaceError)?;

        Ok(Some(str))
    }

    pub fn is_discard_indication(&self) -> bool {
        let discard_indication =
            unsafe { ffi::solClient_msg_isDiscardIndication(self.get_raw_message_ptr()) };

        if discard_indication == 0 {
            return false;
        }

        true
    }
}
