use crate::solace::ffi;
use crate::{Result, SolaceError, SolClientReturnCode};
use num_traits::FromPrimitive;
use std::convert::From;
use std::ffi::CStr;
use std::ops::Drop;
use std::ptr;
use std::time::SystemTime;

pub enum ClassOfServive {
    One,
    Two,
    Three,
}

pub trait Message<'a> {
    fn get_payload_as_bytes(&'a self) -> Result<&'a [u8]>;
    fn get_payload_as_str(&'a self) -> Result<&'a str>;
    fn get_application_message_id(&'a self) -> Result<&'a str>;
    fn get_application_message_type(&'a self) -> Result<&'a str>;
    fn get_class_of_service(&'a self) -> Result<ClassOfServive>;
    fn get_correlation_id(&'a self) -> Result<&'a str>;
    fn get_expiration(&'a self) -> Result<SystemTime>;
    fn get_priority(&'a self) -> Result<u8>;
    fn get_sequence_number(&'a self) -> Result<i64>;
}

pub struct InboundMessage {
    msg_ptr: ffi::solClient_opaqueMsg_pt,
}

impl From<ffi::solClient_opaqueMsg_pt> for InboundMessage {
    // From owned pointer
    // InboundMessage will try to free the ptr when it is destroyed
    fn from(ptr: ffi::solClient_opaqueMsg_pt) -> Self {
        InboundMessage { msg_ptr: ptr }
    }
}

impl Drop for InboundMessage {
    fn drop(&mut self) {
        let msg_free_result = unsafe { ffi::solClient_msg_free(&mut self.msg_ptr) };
        if SolClientReturnCode::from_i32(msg_free_result) != Some(SolClientReturnCode::OK) {
            println!("warning: message was not dropped properly");
        }
    }
}

impl<'a> Message<'a> for InboundMessage {
    // so the problem right now is that, there is no documentaton on who owns the data
    // for the getting the data as bytes, the document reads like we do not own the data
    // for gettnig the data as string, it seems like we own it
    // for now, it might be best to assume we do not own data from any function.
    // and copy over anything we get

    fn get_payload_as_bytes(&'a self) -> Result<&'a [u8]> {
        let mut buffer = ptr::null_mut();
        let mut buffer_len: u32 = 0;
        println!("pointing the buffer to the binary attachment");

        let msg_ops_result = unsafe {
            ffi::solClient_msg_getBinaryAttachmentPtr(
                self.msg_ptr,
                &mut buffer,
                &mut buffer_len as *mut u32,
            )
        };

        if SolClientReturnCode::from_i32(msg_ops_result) != Some(SolClientReturnCode::OK) {
            println!("solace did not return ok; code: {}", msg_ops_result);
            return Err(SolaceError);
        }
        let buf_len = buffer_len.try_into().unwrap();

        let safe_slice = unsafe { std::slice::from_raw_parts(buffer as *const u8, buf_len) };

        Ok(safe_slice)
    }

    fn get_payload_as_str(&'a self) -> Result<&'a str> {
        // TODO
        // this method might be broken
        // fix and test

        let mut buffer = ptr::null();

        println!("pointing the buffer to the binary attachment");
        let msg_ops_result =
            unsafe { ffi::solClient_msg_getBinaryAttachmentString(self.msg_ptr, &mut buffer) };

        if SolClientReturnCode::from_i32(msg_ops_result) != Some(SolClientReturnCode::OK) {
            println!("solace did not return ok");
            return Err(SolaceError);
        }

        println!("successfully pointed the buffer to the binary attachment");

        let c_str = unsafe { CStr::from_ptr(buffer) };
        return c_str.to_str().map_err(|_| SolaceError);
    }

    fn get_application_message_id(&'a self) -> Result<&'a str> {
        todo!()
    }
    fn get_application_message_type(&'a self) -> Result<&'a str> {
        todo!()
    }
    fn get_class_of_service(&'a self) -> Result<ClassOfServive> {
        todo!()
    }
    fn get_correlation_id(&'a self) -> Result<&'a str> {
        todo!()
    }
    fn get_expiration(&'a self) -> Result<SystemTime> {
        todo!()
    }
    fn get_priority(&'a self) -> Result<u8> {
        todo!()
    }
    fn get_sequence_number(&'a self) -> Result<i64> {
        todo!()
    }
}
