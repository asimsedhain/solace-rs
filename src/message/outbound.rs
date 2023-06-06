use super::{DeliveryMode, Message};
use crate::solace::ffi;
use crate::SolClientReturnCode;
use num_traits::FromPrimitive;
use std::ffi::{CString, NulError};
use std::ptr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageBuilderError {
    #[error("builder recieved invalid args")]
    InvalidArgs(#[from] NulError),
}

type Result<T> = std::result::Result<T, MessageBuilderError>;

pub struct OutboundMessage {
    msg_ptr: ffi::solClient_opaqueMsg_pt,
}

impl Drop for OutboundMessage {
    fn drop(&mut self) {
        let msg_free_result = unsafe { ffi::solClient_msg_free(&mut self.msg_ptr) };
        if SolClientReturnCode::from_i32(msg_free_result) != Some(SolClientReturnCode::Ok) {
            println!("warning: message was not dropped properly");
        }
    }
}

impl<'a> Message<'a> for OutboundMessage {
    unsafe fn get_raw_message_ptr(&self) -> ffi::solClient_opaqueMsg_pt {
        self.msg_ptr
    }
}

pub struct OutboundMessageBuilder {
    delivery_mode: Option<DeliveryMode>,
    destination: Option<CString>,
    message: Option<CString>,
}

impl OutboundMessageBuilder {
    pub fn new() -> Self {
        Self {
            delivery_mode: None,
            destination: None,
            message: None,
        }
    }
    pub fn set_delivery_mode(&mut self, mode: DeliveryMode) {
        self.delivery_mode = Some(mode);
    }

    pub fn set_destination<T>(&mut self, destination: T) -> Result<()>
    where
        T: Into<Vec<u8>>,
    {
        self.destination = Some(CString::new(destination)?);
        Ok(())
    }

    pub fn set_binary_string<M>(&mut self, message: M) -> Result<()>
    where
        M: Into<Vec<u8>>,
    {
        // for attaching the message to the ptr, we have a couple of options
        // based on those options, we can create a couple of interfaces
        //
        // solClient_msg_setBinaryAttachmentPtr (solClient_opaqueMsg_pt msg_p, void *buf_p, solClient_uint32_t size)
        // Given a msg_p, set the contents of a Binary Attachment Part to the given pointer and size.
        //
        // solClient_msg_setBinaryAttachment (solClient_opaqueMsg_pt msg_p, const void *buf_p, solClient_uint32_t size)
        // Given a msg_p, set the contents of the binary attachment part by copying in from the given pointer and size.
        //
        // solClient_msg_setBinaryAttachmentString (solClient_opaqueMsg_pt msg_p, const char *buf_p)
        // Given a msg_p, set the contents of the binary attachment part to a UTF-8 or ASCII string by copying in from the given pointer until null-terminated.
        //
        self.message = Some(CString::new(message)?);
        Ok(())
    }

    pub fn build(self) -> Result<OutboundMessage> {
        let mut msg_ptr: ffi::solClient_opaqueMsg_pt = ptr::null_mut();

        let msg_alloc_result = unsafe { ffi::solClient_msg_alloc(&mut msg_ptr) };
        assert_eq!(
            SolClientReturnCode::from_i32(msg_alloc_result),
            Some(SolClientReturnCode::Ok)
        );

        let Some(delivery_mode) = self.delivery_mode else{
            panic!();
        };

        let set_delivery_result =
            unsafe { ffi::solClient_msg_setDeliveryMode(msg_ptr, delivery_mode as u32) };
        assert_eq!(
            SolClientReturnCode::from_i32(set_delivery_result),
            Some(SolClientReturnCode::Ok)
        );

        let Some(destination) = self.destination else{
            panic!();
        };

        let mut destination: ffi::solClient_destination = ffi::solClient_destination {
            destType: ffi::solClient_destinationType_SOLCLIENT_TOPIC_DESTINATION,
            dest: destination.as_ptr(),
        };

        let set_destination_result = unsafe {
            ffi::solClient_msg_setDestination(
                msg_ptr,
                &mut destination,
                std::mem::size_of::<ffi::solClient_destination>(),
            )
        };

        assert_eq!(
            SolClientReturnCode::from_i32(set_destination_result),
            Some(SolClientReturnCode::Ok)
        );

        // I thought we would have passed ownership to the c function
        // but we are passing a reference to the c function instead
        let Some(message) = self.message else{
            panic!();
        };
        let set_attachment_result =
            unsafe { ffi::solClient_msg_setBinaryAttachmentString(msg_ptr, message.as_ptr()) };
        assert_eq!(
            SolClientReturnCode::from_i32(set_attachment_result),
            Some(SolClientReturnCode::Ok)
        );

        Ok(OutboundMessage { msg_ptr })
    }
}
