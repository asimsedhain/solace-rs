use super::{CacheStatus, Message, MessageError, Result};
use crate::util::get_last_error_info;
use crate::{SolClientReturnCode, SolClientSubCode};
use enum_primitive::*;
use solace_rs_sys::{self as ffi, solClient_msgId_t};
use std::convert::From;
use std::ffi::CStr;
use std::time::{Duration, SystemTime};
use std::{fmt, ptr};
use tracing::warn;

pub struct InboundMessage {
    _msg_ptr: ffi::solClient_opaqueMsg_pt,
}

impl InboundMessageTrait<'_> for InboundMessage {}

impl fmt::Debug for InboundMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("InboundMessage");
        debug_inbound_message_fields(self, &mut f);
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

pub struct FlowInboundMessage {
    _msg_ptr: ffi::solClient_opaqueMsg_pt,
    _flow_ptr: ffi::solClient_opaqueFlow_pt,
}

impl InboundMessageTrait<'_> for FlowInboundMessage {}

impl fmt::Debug for FlowInboundMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("FlowInboundMessage");
        debug_inbound_message_fields(self, &mut f);
        f.finish()
    }
}

unsafe impl Send for FlowInboundMessage {}

impl Drop for FlowInboundMessage {
    fn drop(&mut self) {
        let rc = unsafe { ffi::solClient_msg_free(&mut self._msg_ptr) };

        let rc = SolClientReturnCode::from_raw(rc);
        if !rc.is_ok() {
            warn!("warning: message was not dropped properly");
        }
    }
}

impl From<(ffi::solClient_opaqueMsg_pt, ffi::solClient_opaqueFlow_pt)> for FlowInboundMessage {
    /// .
    ///
    /// # Safety
    ///
    /// From a valid owned pointer.
    /// No other alias should exist for this pointer
    /// FlowInboundMessage will try to free the ptr when it is destroyed
    ///
    /// .
    fn from(
        (_msg_ptr, _flow_ptr): (ffi::solClient_opaqueMsg_pt, ffi::solClient_opaqueFlow_pt),
    ) -> Self {
        Self {
            _msg_ptr,
            _flow_ptr,
        }
    }
}

impl<'a> Message<'a> for FlowInboundMessage {
    unsafe fn get_raw_message_ptr(&self) -> ffi::solClient_opaqueMsg_pt {
        self._msg_ptr
    }
}

#[derive(thiserror::Error, Debug)]
pub enum FlowInboundMessageAckError {
    #[error("Invalid message: subcode {0}")]
    InvalidMessage(SolClientSubCode),
    #[error("Message not found")]
    MessageNotFound,
    #[error("Ack failed: subcode {0}")]
    AckFailed(SolClientSubCode),
    #[error("Flow was freed before message ack")]
    FlowFreedBeforeAck,
}

impl FlowInboundMessage {
    pub fn try_ack(&self) -> std::result::Result<(), FlowInboundMessageAckError> {
        let mut message_id: solClient_msgId_t = 0;
        let get_message_id_return_code = unsafe {
            let get_message_id_return_code_raw =
                ffi::solClient_msg_getMsgId(self._msg_ptr, &mut message_id);
            SolClientReturnCode::from_raw(get_message_id_return_code_raw)
        };
        if let SolClientReturnCode::NotFound = get_message_id_return_code {
            return Err(FlowInboundMessageAckError::MessageNotFound);
        }
        if !get_message_id_return_code.is_ok() {
            return Err(FlowInboundMessageAckError::InvalidMessage(
                get_last_error_info(),
            ));
        }

        let send_ack_return_code = unsafe {
            if self._flow_ptr.is_null() {
                return Err(FlowInboundMessageAckError::FlowFreedBeforeAck);
            }
            let send_ack_return_code_raw = ffi::solClient_flow_sendAck(self._flow_ptr, message_id);
            SolClientReturnCode::from_raw(send_ack_return_code_raw)
        };
        if !send_ack_return_code.is_ok() {
            return Err(FlowInboundMessageAckError::AckFailed(get_last_error_info()));
        }

        Ok(())
    }
}

pub trait InboundMessageTrait<'a>: Message<'a> {
    fn get_receive_timestamp(&'a self) -> Result<Option<SystemTime>> {
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

    fn get_sender_id(&'a self) -> Result<Option<&'a str>> {
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

    fn is_discard_indication(&'a self) -> bool {
        let discard_indication =
            unsafe { ffi::solClient_msg_isDiscardIndication(self.get_raw_message_ptr()) };

        if discard_indication == 0 {
            return false;
        }

        true
    }

    fn get_cache_request_id(&'a self) -> Result<Option<u64>> {
        let mut id: u64 = 0;

        let rc =
            unsafe { ffi::solClient_msg_getCacheRequestId(self.get_raw_message_ptr(), &mut id) };

        let rc = SolClientReturnCode::from_raw(rc);
        match rc {
            SolClientReturnCode::Ok => Ok(Some(id)),
            SolClientReturnCode::NotFound => Ok(None),
            _ => Err(MessageError::FieldError("cache_request_id", rc)),
        }
    }

    fn is_cache_msg(&'a self) -> CacheStatus {
        let raw = unsafe { ffi::solClient_msg_isCacheMsg(self.get_raw_message_ptr()) };
        CacheStatus::from_i32(raw).unwrap_or(CacheStatus::InvalidMessage)
    }
}

pub fn debug_inbound_message_fields<'a, M: InboundMessageTrait<'a>>(
    message: &'a M,
    f: &mut fmt::DebugStruct<'_, '_>,
) {
    if message.get_receive_timestamp().is_ok_and(|v| v.is_some()) {
        f.field(
            "receive_timestamp",
            &format_args!("{:?}", message.get_receive_timestamp().unwrap().unwrap()),
        );
    }
    if message.get_sender_id().is_ok_and(|v| v.is_some()) {
        f.field(
            "sender_id",
            &format_args!("{}", message.get_sender_id().unwrap().unwrap()),
        );
    }
    if message.get_sender_timestamp().is_ok_and(|v| v.is_some()) {
        f.field(
            "sender_timestamp",
            &format_args!("{:?}", message.get_sender_timestamp().unwrap().unwrap()),
        );
    }
    if message.get_sequence_number().is_ok_and(|v| v.is_some()) {
        f.field(
            "sequence_number",
            &format_args!("{}", message.get_sequence_number().unwrap().unwrap()),
        );
    }
    if message.get_correlation_id().is_ok_and(|v| v.is_some()) {
        f.field(
            "correlation_id",
            &format_args!("{}", message.get_correlation_id().unwrap().unwrap()),
        );
    }
    if message.get_priority().is_ok_and(|v| v.is_some()) {
        f.field(
            "priority",
            &format_args!("{}", message.get_priority().unwrap().unwrap()),
        );
    }
    if message.is_discard_indication() {
        f.field(
            "is_discard_indication",
            &format_args!("{}", message.is_discard_indication()),
        );
    }
    if message.get_application_message_id().is_some() {
        f.field(
            "application_message_id",
            &format_args!("{}", &message.get_application_message_id().unwrap()),
        );
    }
    if message.get_user_data().is_ok_and(|v| v.is_some()) {
        if let Ok(v) = std::str::from_utf8(message.get_user_data().unwrap().unwrap()) {
            f.field("user_data", &v);
        }
    }
    if message.get_destination().is_ok_and(|v| v.is_some()) {
        f.field("destination", &message.get_destination().unwrap().unwrap());
    }

    f.field("is_reply", &message.is_reply());

    if message.get_reply_to().is_ok_and(|v| v.is_some()) {
        f.field("reply_to", &message.get_reply_to().unwrap().unwrap());
    }

    f.field("is_cache_msg", &message.is_cache_msg());

    if message.get_cache_request_id().is_ok_and(|v| v.is_some()) {
        f.field(
            "cache_request_id",
            &message.get_cache_request_id().unwrap().unwrap(),
        );
    }

    if message.get_payload().is_ok_and(|v| v.is_some()) {
        if let Ok(v) = std::str::from_utf8(message.get_payload().unwrap().unwrap()) {
            f.field("payload", &v);
        }
    }
}
