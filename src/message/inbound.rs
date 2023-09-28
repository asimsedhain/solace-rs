use super::Message;
use crate::{Result, SolClientReturnCode, SolaceError};
use enum_primitive::*;
use solace_rs_sys as ffi;
use std::convert::From;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::ptr;
use std::time::{Duration, SystemTime};
use tracing::warn;

pub struct InboundMessage<'a> {
    msg_ptr: ffi::solClient_opaqueMsg_pt,
    _phantom: PhantomData<&'a u8>,
}

impl Drop for InboundMessage<'_> {
    fn drop(&mut self) {
        let msg_free_result = unsafe { ffi::solClient_msg_free(&mut self.msg_ptr) };
        if SolClientReturnCode::from_i32(msg_free_result) != Some(SolClientReturnCode::Ok) {
            warn!("warning: message was not dropped properly");
        }
    }
}

impl From<ffi::solClient_opaqueMsg_pt> for InboundMessage<'_> {
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
        Self {
            msg_ptr: ptr,
            _phantom: PhantomData,
        }
    }
}

impl<'a> Message<'a> for InboundMessage<'a> {
    unsafe fn get_raw_message_ptr(&self) -> ffi::solClient_opaqueMsg_pt {
        self.msg_ptr
    }
}

impl<'a> InboundMessage<'a> {
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

    pub fn get_sender_id(&'a self) -> Result<Option<&'a str>> {
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
