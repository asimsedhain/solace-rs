use super::{Message, MessageError, Result};
use crate::SolClientReturnCode;
use enum_primitive::*;
use solace_rs_sys as ffi;
use std::convert::From;
use std::ffi::CStr;
use std::time::{Duration, SystemTime};
use std::{fmt, ptr};
use tracing::warn;

pub struct InboundMessage {
    _msg_ptr: ffi::solClient_opaqueMsg_pt,
}

impl fmt::Debug for InboundMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("InboundMessage");
        if self.get_receive_timestamp().is_ok_and(|v| v.is_some()) {
            f.field(
                "receive_timestamp",
                &format_args!("{:?}", self.get_receive_timestamp().unwrap().unwrap()),
            );
        }
        if self.get_sender_id().is_ok_and(|v| v.is_some()) {
            f.field(
                "sender_id",
                &format_args!("{}", self.get_sender_id().unwrap().unwrap()),
            );
        }
        if self.get_sender_timestamp().is_ok_and(|v| v.is_some()) {
            f.field(
                "sender_timestamp",
                &format_args!("{:?}", self.get_sender_timestamp().unwrap().unwrap()),
            );
        }
        if self.get_sequence_number().is_ok_and(|v| v.is_some()) {
            f.field(
                "sequence_number",
                &format_args!("{}", self.get_sequence_number().unwrap().unwrap()),
            );
        }
        if self.get_correlation_id().is_ok_and(|v| v.is_some()) {
            f.field(
                "correlation_id",
                &format_args!("{}", self.get_correlation_id().unwrap().unwrap()),
            );
        }
        if self.get_priority().is_ok_and(|v| v.is_some()) {
            f.field(
                "priority",
                &format_args!("{}", self.get_priority().unwrap().unwrap()),
            );
        }
        if self.is_discard_indication() {
            f.field(
                "is_discard_indication",
                &format_args!("{}", self.is_discard_indication()),
            );
        }
        if self.get_application_message_id().is_some() {
            f.field(
                "application_message_id",
                &format_args!("{}", &self.get_application_message_id().unwrap()),
            );
        }
        if self.get_user_data().is_ok_and(|v| v.is_some()) {
            if let Ok(v) = std::str::from_utf8(self.get_user_data().unwrap().unwrap()) {
                f.field("user_data", &v);
            }
        }
        if self.get_destination().is_ok_and(|v| v.is_some()) {
            f.field("destination", &self.get_destination().unwrap().unwrap());
        }
        if self.get_payload().is_ok_and(|v| v.is_some()) {
            if let Ok(v) = std::str::from_utf8(self.get_payload().unwrap().unwrap()) {
                f.field("payload", &v);
            }
        }
        f.finish()
    }
}

unsafe impl Send for InboundMessage {}

impl Drop for InboundMessage {
    fn drop(&mut self) {
        let rc = unsafe { ffi::solClient_msg_free(&mut self._msg_ptr) };

        let rc = SolClientReturnCode::from_raw(rc);
        if !rc.is_ok() {
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
        let rc = unsafe { ffi::solClient_msg_getRcvTimestamp(self.get_raw_message_ptr(), &mut ts) };

        let rc = SolClientReturnCode::from_raw(rc);
        match rc {
            SolClientReturnCode::NotFound => Ok(None),
            SolClientReturnCode::Ok => Ok(Some(
                SystemTime::UNIX_EPOCH + Duration::from_millis(ts.try_into().unwrap()),
            )),
            _ => Err(MessageError::FieldError("receive_timestamp", rc)),
        }
    }

    pub fn get_sender_id(&self) -> Result<Option<&str>> {
        let mut buffer = ptr::null();

        let rc = unsafe { ffi::solClient_msg_getSenderId(self.get_raw_message_ptr(), &mut buffer) };

        let rc = SolClientReturnCode::from_raw(rc);
        match rc {
            SolClientReturnCode::Ok => (),
            SolClientReturnCode::NotFound => return Ok(None),
            _ => return Err(MessageError::FieldError("sender_id", rc)),
        }

        let c_str = unsafe { CStr::from_ptr(buffer) };

        let str = c_str
            .to_str()
            .map_err(|_| MessageError::FieldConvertionError("sender_id"))?;

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
