use super::Message;
use crate::solace::ffi;
use crate::SolClientReturnCode;
use enum_primitive::*;
use std::convert::From;

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
    // From owned pointer
    // InboundMessage will try to free the ptr when it is destroyed
    fn from(ptr: ffi::solClient_opaqueMsg_pt) -> Self {
        Self { msg_ptr: ptr }
    }
}

impl<'a> Message<'a> for InboundMessage {
    unsafe fn get_raw_message_ptr(&self) -> ffi::solClient_opaqueMsg_pt {
        self.msg_ptr
    }
}
