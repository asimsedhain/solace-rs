pub mod destination;
pub mod inbound;
pub mod outbound;

use crate::{Result, SolClientReturnCode, SolaceError};
pub use destination::{DestinationType, MessageDestination};
use enum_primitive::*;
pub use inbound::InboundMessage;
pub use outbound::{OutboundMessage, OutboundMessageBuilder};
use solace_rs_sys as ffi;
use std::ffi::CStr;
use std::mem;
use std::mem::size_of;
use std::ptr;

// the below assertions makes sure that u32 can always be converted into usize safely.
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

        let msg_ops_result = unsafe {
            ffi::solClient_msg_getBinaryAttachmentPtr(
                self.get_raw_message_ptr(),
                &mut buffer,
                &mut buffer_len,
            )
        };

        match SolClientReturnCode::from_i32(msg_ops_result) {
            Some(SolClientReturnCode::Ok) => (),
            Some(SolClientReturnCode::NotFound) => return Ok(None),
            _ => return Err(SolaceError),
        }

        // the compile time check ASSERT_USIZE_IS_AT_LEAST_U32 guarantees that this conversion is
        // possible
        let buf_len = buffer_len.try_into().unwrap();

        let safe_slice = unsafe { std::slice::from_raw_parts(buffer as *const u8, buf_len) };

        Ok(Some(safe_slice))
    }

    fn get_application_message_id(&'a self) -> Option<&'a str> {
        let mut buffer = ptr::null();

        let op_result = unsafe {
            ffi::solClient_msg_getApplicationMessageId(self.get_raw_message_ptr(), &mut buffer)
        };

        if SolClientReturnCode::from_i32(op_result) != Some(SolClientReturnCode::Ok) {
            return None;
        }

        let c_str = unsafe { CStr::from_ptr(buffer) };

        c_str.to_str().ok()
    }

    fn get_application_msg_type(&'a self) -> Option<&'a str> {
        let mut buffer = ptr::null();

        let op_result = unsafe {
            ffi::solClient_msg_getApplicationMsgType(self.get_raw_message_ptr(), &mut buffer)
        };

        if SolClientReturnCode::from_i32(op_result) != Some(SolClientReturnCode::Ok) {
            return None;
        }

        let c_str = unsafe { CStr::from_ptr(buffer) };

        c_str.to_str().ok()
    }

    fn get_class_of_service(&'a self) -> Result<ClassOfService> {
        let mut cos: u32 = 0;
        let cos_result =
            unsafe { ffi::solClient_msg_getClassOfService(self.get_raw_message_ptr(), &mut cos) };

        if SolClientReturnCode::from_i32(cos_result) != Some(SolClientReturnCode::Ok) {
            return Err(SolaceError);
        }

        let Some(cos) = ClassOfService::from_u32(cos) else {
            return Err(SolaceError);
        };

        Ok(cos)
    }

    fn get_correlation_id(&'a self) -> Result<&'a str> {
        let mut buffer = ptr::null();

        let msg_ops_result =
            unsafe { ffi::solClient_msg_getCorrelationId(self.get_raw_message_ptr(), &mut buffer) };

        if SolClientReturnCode::from_i32(msg_ops_result) != Some(SolClientReturnCode::Ok) {
            return Err(SolaceError);
        }

        let c_str = unsafe { CStr::from_ptr(buffer) };
        return c_str.to_str().map_err(|_| SolaceError);
    }

    fn get_expiration(&'a self) -> i64 {
        let mut exp: i64 = 0;
        unsafe { ffi::solClient_msg_getExpiration(self.get_raw_message_ptr(), &mut exp) };

        exp
    }

    fn get_priority(&'a self) -> Result<Option<u8>> {
        let mut priority: i32 = 0;
        let op_result =
            unsafe { ffi::solClient_msg_getPriority(self.get_raw_message_ptr(), &mut priority) };

        if Some(SolClientReturnCode::Ok) != SolClientReturnCode::from_i32(op_result) {
            return Err(SolaceError);
        }

        if priority == -1 {
            return Ok(None);
        }

        Ok(Some(priority as u8))
    }

    fn get_sequence_number(&'a self) -> Result<Option<i64>> {
        let mut seq_num: i64 = 0;
        let op_result = unsafe {
            ffi::solClient_msg_getSequenceNumber(self.get_raw_message_ptr(), &mut seq_num)
        };
        match SolClientReturnCode::from_i32(op_result) {
            Some(SolClientReturnCode::Ok) => Ok(Some(seq_num)),
            Some(SolClientReturnCode::NotFound) => Ok(None),
            _ => Err(SolaceError),
        }
    }

    fn get_destination(&'a self) -> Result<Option<MessageDestination>> {
        let mut dest_struct: ffi::solClient_destination = ffi::solClient_destination {
            destType: ffi::solClient_destinationType_SOLCLIENT_NULL_DESTINATION,
            dest: ptr::null_mut(),
        };

        let msg_ops_result = unsafe {
            ffi::solClient_msg_getDestination(
                self.get_raw_message_ptr(),
                &mut dest_struct,
                mem::size_of::<ffi::solClient_destination>(),
            )
        };
        if SolClientReturnCode::from_i32(msg_ops_result) == Some(SolClientReturnCode::NotFound) {
            return Ok(None);
        }

        if SolClientReturnCode::from_i32(msg_ops_result) == Some(SolClientReturnCode::Fail) {
            return Err(SolaceError);
        }

        Ok(Some(MessageDestination::from(dest_struct)))
    }

    fn get_user_data(&'a self) -> Result<Option<&'a [u8]>> {
        let mut buffer = ptr::null_mut();
        let mut buffer_len: u32 = 0;

        let msg_ops_result = unsafe {
            ffi::solClient_msg_getUserDataPtr(
                self.get_raw_message_ptr(),
                &mut buffer,
                &mut buffer_len,
            )
        };

        match SolClientReturnCode::from_i32(msg_ops_result) {
            Some(SolClientReturnCode::Ok) => (),
            Some(SolClientReturnCode::NotFound) => return Ok(None),
            _ => return Err(SolaceError),
        }

        // the compile time check ASSERT_USIZE_IS_AT_LEAST_U32 guarantees that this conversion is
        // possible
        let buf_len = buffer_len.try_into().unwrap();

        let safe_slice = unsafe { std::slice::from_raw_parts(buffer as *const u8, buf_len) };

        Ok(Some(safe_slice))
    }
}
