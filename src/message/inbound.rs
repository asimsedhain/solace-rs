use super::Message;
use crate::SolClientReturnCode;
use enum_primitive::*;
use solace_sys as ffi;
use std::convert::From;
use std::time::SystemTime;

pub struct InboundMessage {
    msg_ptr: ffi::solClient_opaqueMsg_pt,
}

impl Drop for InboundMessage {
    fn drop(&mut self) {
        let msg_free_result = unsafe { ffi::solClient_msg_free(&mut self.msg_ptr) };
        if SolClientReturnCode::from_i32(msg_free_result) != Some(SolClientReturnCode::Ok) {
            println!("warning: message was not dropped properly");
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
        Self { msg_ptr: ptr }
    }
}

impl<'a> Message<'a> for InboundMessage {
    unsafe fn get_raw_message_ptr(&self) -> ffi::solClient_opaqueMsg_pt {
        self.msg_ptr
    }
}

impl InboundMessage {
    pub fn get_receive_timestamp(&self) -> SystemTime {
        todo!()
    }

    pub fn get_sender_timestamp(&self) -> SystemTime {
        todo!()
    }

    pub fn get_sender_id(&self) -> String {
        todo!()
    }

    pub fn is_discard_indication(&self) -> bool {
        todo!()
    }
}
