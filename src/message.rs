pub mod inbound;
pub mod outbound;

use crate::solace::ffi;
use crate::{Result, SolClientReturnCode, SolaceError};
use enum_primitive::*;
pub use inbound::InboundMessage;
pub use outbound::{OutboundMessage, OutboundMessageBuilder};
use std::ffi::CStr;
use std::ptr;
use std::time::SystemTime;

enum_from_primitive! {
    #[derive(Debug, PartialEq)]
    #[repr(u32)]
    pub enum DeliveryMode {
        Direct=ffi::SOLCLIENT_DELIVERY_MODE_DIRECT,
        Persistent=ffi::SOLCLIENT_DELIVERY_MODE_PERSISTENT,
        NonPersistent=ffi::SOLCLIENT_DELIVERY_MODE_NONPERSISTENT
    }
}

pub enum ClassOfServive {
    One,
    Two,
    Three,
}

pub trait Message<'a> {
    // so the problem right now is that, there is no documentaton on who owns the data
    // for the getting the data as bytes, the document reads like we do not own the data
    // for gettnig the data as string, it seems like we own it
    // for now, it might be best to assume we do not own data from any function.
    // and copy over anything we get
    unsafe fn get_raw_message_ptr(&'a self) -> ffi::solClient_opaqueMsg_pt;

    fn get_payload_as_bytes(&'a self) -> Result<&'a [u8]> {
        let mut buffer = ptr::null_mut();
        let mut buffer_len: u32 = 0;
        println!("pointing the buffer to the binary attachment");

        let msg_ops_result = unsafe {
            ffi::solClient_msg_getBinaryAttachmentPtr(
                self.get_raw_message_ptr(),
                &mut buffer,
                &mut buffer_len as *mut u32,
            )
        };

        if SolClientReturnCode::from_i32(msg_ops_result) != Some(SolClientReturnCode::Ok) {
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
        let msg_ops_result = unsafe {
            ffi::solClient_msg_getBinaryAttachmentString(self.get_raw_message_ptr(), &mut buffer)
        };

        if SolClientReturnCode::from_i32(msg_ops_result) != Some(SolClientReturnCode::Ok) {
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
