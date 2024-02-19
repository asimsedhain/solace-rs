pub mod destination;
pub mod inbound;
pub mod outbound;

use crate::SolClientReturnCode;
pub use destination::{DestinationType, MessageDestination};
use enum_primitive::*;
pub use inbound::InboundMessage;
pub use outbound::{OutboundMessage, OutboundMessageBuilder};
use solace_rs_sys as ffi;
use std::ffi::CStr;
use std::mem;
use std::mem::size_of;
use std::ptr;
use std::time::{Duration, SystemTime};
use thiserror::Error;

// the below assertions makes sure that u32 can always be converted into usize safely.
#[allow(dead_code)]
const ASSERT_USIZE_IS_AT_LEAST_U32: () = assert!(size_of::<u32>() <= size_of::<usize>());

enum_from_primitive! {
    #[derive(Debug, PartialEq, Eq)]
    #[repr(u32)]
    pub enum DeliveryMode {
        Direct=ffi::SOLCLIENT_DELIVERY_MODE_DIRECT,
        Persistent=ffi::SOLCLIENT_DELIVERY_MODE_PERSISTENT,
        NonPersistent=ffi::SOLCLIENT_DELIVERY_MODE_NONPERSISTENT
    }
}

enum_from_primitive! {
    #[derive(Debug, PartialEq, Eq)]
    #[repr(u32)]
    pub enum ClassOfService {
        One=ffi::SOLCLIENT_COS_1,
        Two=ffi::SOLCLIENT_COS_2,
        Three=ffi::SOLCLIENT_COS_3,
    }
}

impl From<ClassOfService> for u32 {
    fn from(val: ClassOfService) -> Self {
        match val {
            ClassOfService::One => ffi::SOLCLIENT_COS_1,
            ClassOfService::Two => ffi::SOLCLIENT_COS_2,
            ClassOfService::Three => ffi::SOLCLIENT_COS_3,
        }
    }
}

#[derive(Error, Debug)]
pub enum MessageError {
    #[error("failed to get field. SolClient return code: {0}")]
    FieldError(&'static str, SolClientReturnCode),
    #[error("failed to convert field from solace")]
    FieldConvertionError(&'static str),
}

type Result<T> = std::result::Result<T, MessageError>;

pub trait Message<'a> {
    /// .
    ///
    /// # Safety
    ///
    /// Should return ptr to a owned valid message.
    /// No other alias for the ptr should exists.
    /// Other methods will not check if the message is valid or not
    ///
    /// .
    unsafe fn get_raw_message_ptr(&'a self) -> ffi::solClient_opaqueMsg_pt;

    fn get_payload(&'a self) -> Result<Option<&'a [u8]>> {
        let mut buffer = ptr::null_mut();
        let mut buffer_len: u32 = 0;

        let msg_ops_rc = unsafe {
            ffi::solClient_msg_getBinaryAttachmentPtr(
                self.get_raw_message_ptr(),
                &mut buffer,
                &mut buffer_len,
            )
        };

        let rc = SolClientReturnCode::from_raw(msg_ops_rc);
        match rc {
            SolClientReturnCode::Ok => (),
            SolClientReturnCode::NotFound => return Ok(None),
            _ => return Err(MessageError::FieldError("payload", rc)),
        }

        // the compile time check ASSERT_USIZE_IS_AT_LEAST_U32 guarantees that this conversion is
        // possible
        let buf_len = buffer_len.try_into().unwrap();

        let safe_slice = unsafe { std::slice::from_raw_parts(buffer as *const u8, buf_len) };

        Ok(Some(safe_slice))
    }

    fn get_application_message_id(&'a self) -> Option<&'a str> {
        let mut buffer = ptr::null();

        let rc = unsafe {
            ffi::solClient_msg_getApplicationMessageId(self.get_raw_message_ptr(), &mut buffer)
        };

        let rc = SolClientReturnCode::from_raw(rc);

        if !rc.is_ok() {
            return None;
        }

        let c_str = unsafe { CStr::from_ptr(buffer) };

        c_str.to_str().ok()
    }

    fn get_application_msg_type(&'a self) -> Option<&'a str> {
        let mut buffer = ptr::null();

        let rc = unsafe {
            ffi::solClient_msg_getApplicationMsgType(self.get_raw_message_ptr(), &mut buffer)
        };

        let rc = SolClientReturnCode::from_raw(rc);

        if !rc.is_ok() {
            return None;
        }

        let c_str = unsafe { CStr::from_ptr(buffer) };

        c_str.to_str().ok()
    }

    fn get_class_of_service(&'a self) -> Result<ClassOfService> {
        let mut cos: u32 = 0;
        let rc =
            unsafe { ffi::solClient_msg_getClassOfService(self.get_raw_message_ptr(), &mut cos) };

        let rc = SolClientReturnCode::from_raw(rc);
        if !rc.is_ok() {
            return Err(MessageError::FieldError("ClassOfService", rc));
        }

        let Some(cos) = ClassOfService::from_u32(cos) else {
            return Err(MessageError::FieldConvertionError("ClassOfService"));
        };

        Ok(cos)
    }

    fn get_correlation_id(&'a self) -> Result<Option<&'a str>> {
        let mut buffer = ptr::null();

        let rc =
            unsafe { ffi::solClient_msg_getCorrelationId(self.get_raw_message_ptr(), &mut buffer) };

        let rc = SolClientReturnCode::from_raw(rc);
        match rc {
            SolClientReturnCode::Ok => (),
            SolClientReturnCode::NotFound => return Ok(None),
            _ => return Err(MessageError::FieldError("correlation_id", rc)),
        }

        let c_str = unsafe { CStr::from_ptr(buffer) };

        let str = c_str
            .to_str()
            .map_err(|_| MessageError::FieldConvertionError("correlation_id"))?;

        Ok(Some(str))
    }

    fn get_expiration(&'a self) -> i64 {
        let mut exp: i64 = 0;
        unsafe { ffi::solClient_msg_getExpiration(self.get_raw_message_ptr(), &mut exp) };

        exp
    }

    fn get_priority(&'a self) -> Result<Option<u8>> {
        let mut priority: i32 = 0;
        let rc =
            unsafe { ffi::solClient_msg_getPriority(self.get_raw_message_ptr(), &mut priority) };

        let rc = SolClientReturnCode::from_raw(rc);
        if !rc.is_ok() {
            return Err(MessageError::FieldError("priority", rc));
        }

        if priority == -1 {
            return Ok(None);
        }

        Ok(Some(priority as u8))
    }

    fn get_sequence_number(&'a self) -> Result<Option<i64>> {
        let mut seq_num: i64 = 0;
        let rc = unsafe {
            ffi::solClient_msg_getSequenceNumber(self.get_raw_message_ptr(), &mut seq_num)
        };
        let rc = SolClientReturnCode::from_raw(rc);

        match rc {
            SolClientReturnCode::Ok => Ok(Some(seq_num)),
            SolClientReturnCode::NotFound => Ok(None),
            _ => Err(MessageError::FieldError("sequence_number", rc)),
        }
    }

    fn get_destination(&'a self) -> Result<Option<MessageDestination>> {
        let mut dest_struct: ffi::solClient_destination = ffi::solClient_destination {
            destType: ffi::solClient_destinationType_SOLCLIENT_NULL_DESTINATION,
            dest: ptr::null_mut(),
        };

        let rc = unsafe {
            ffi::solClient_msg_getDestination(
                self.get_raw_message_ptr(),
                &mut dest_struct,
                mem::size_of::<ffi::solClient_destination>(),
            )
        };

        let rc = SolClientReturnCode::from_raw(rc);

        match rc {
            SolClientReturnCode::NotFound => Ok(None),
            SolClientReturnCode::Fail => Err(MessageError::FieldError("destination", rc)),
            _ => Ok(Some(MessageDestination::from(dest_struct))),
        }
    }

    fn get_sender_timestamp(&'a self) -> Result<Option<SystemTime>> {
        let mut ts: i64 = 0;
        let rc =
            unsafe { ffi::solClient_msg_getSenderTimestamp(self.get_raw_message_ptr(), &mut ts) };

        let rc = SolClientReturnCode::from_raw(rc);

        match rc {
            SolClientReturnCode::NotFound => Ok(None),
            SolClientReturnCode::Ok => Ok(Some(
                SystemTime::UNIX_EPOCH + Duration::from_millis(ts.try_into().unwrap()),
            )),
            _ => Err(MessageError::FieldError("sender_timestamp", rc)),
        }
    }

    fn get_user_data(&'a self) -> Result<Option<&'a [u8]>> {
        let mut buffer = ptr::null_mut();
        let mut buffer_len: u32 = 0;

        let rc = unsafe {
            ffi::solClient_msg_getUserDataPtr(
                self.get_raw_message_ptr(),
                &mut buffer,
                &mut buffer_len,
            )
        };

        let rc = SolClientReturnCode::from_raw(rc);
        match rc {
            SolClientReturnCode::Ok => (),
            SolClientReturnCode::NotFound => return Ok(None),
            _ => return Err(MessageError::FieldError("user_data", rc)),
        }

        // the compile time check ASSERT_USIZE_IS_AT_LEAST_U32 guarantees that this conversion is
        // possible
        let buf_len = buffer_len.try_into().unwrap();

        let safe_slice = unsafe { std::slice::from_raw_parts(buffer as *const u8, buf_len) };

        Ok(Some(safe_slice))
    }
}
