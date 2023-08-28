use super::destination::MessageDestination;
use super::{ClassOfService, DeliveryMode, Message};
use crate::solace::ffi;
use crate::SolClientReturnCode;
use num_traits::FromPrimitive;
use std::ffi::{c_void, CString, NulError};
use std::ptr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageBuilderError {
    #[error("builder recieved invalid args")]
    InvalidArgs(#[from] NulError),
    #[error("{0} arg need to be set")]
    MissingArgs(String),
    #[error("solClient returned not ok code")]
    SolClientError,
    #[error("solClient message aloc failed")]
    MessageAlocFailure,
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

#[derive(Default)]
pub struct OutboundMessageBuilder {
    delivery_mode: Option<DeliveryMode>,
    destination: Option<MessageDestination>,
    message: Option<Vec<u8>>,
    correlation_id: Option<Vec<u8>>,
    class_of_service: Option<ClassOfService>,
    seq_number: Option<u64>,
    priority: Option<u8>,
    application_id: Option<Vec<u8>>,
    application_msg_type: Option<Vec<u8>>,
}

impl OutboundMessageBuilder {
    /// Creates a new [`OutboundMessageBuilder`].
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set_delivery_mode(mut self, mode: DeliveryMode) -> Self {
        self.delivery_mode = Some(mode);
        self
    }

    pub fn set_application_id<M>(mut self, application_id: M) -> Self
    where
        M: Into<Vec<u8>>,
    {
        self.application_id = Some(application_id.into());
        self
    }

    pub fn set_application_msg_type<M>(mut self, message_type: M) -> Self
    where
        M: Into<Vec<u8>>,
    {
        self.application_msg_type = Some(message_type.into());
        self
    }

    pub fn set_destination(mut self, destination: MessageDestination) -> Self {
        self.destination = Some(destination);
        self
    }

    pub fn set_class_of_service(mut self, cos: ClassOfService) -> Self {
        self.class_of_service = Some(cos);
        self
    }

    pub fn set_seq_number(mut self, seq_num: u64) -> Self {
        self.seq_number = Some(seq_num);
        self
    }

    pub fn set_priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn set_payload<M>(mut self, message: M) -> Self
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
        // we will only use the binary ptr methods
        self.message = Some(message.into());

        self
    }

    pub fn set_correlation_id<M>(mut self, id: M) -> Self
    where
        M: Into<Vec<u8>>,
    {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn build(self) -> Result<OutboundMessage> {
        // message allocation
        let mut msg_ptr: ffi::solClient_opaqueMsg_pt = ptr::null_mut();
        let msg_alloc_result = unsafe { ffi::solClient_msg_alloc(&mut msg_ptr) };
        if Some(SolClientReturnCode::Ok) != SolClientReturnCode::from_i32(msg_alloc_result) {
            return Err(MessageBuilderError::MessageAlocFailure);
        };

        // delivery_mode
        let Some(delivery_mode) = self.delivery_mode else{
            return Err(MessageBuilderError::MissingArgs("delivery_mode".to_owned()));
        };
        let set_delivery_result =
            unsafe { ffi::solClient_msg_setDeliveryMode(msg_ptr, delivery_mode as u32) };
        if Some(SolClientReturnCode::Ok) != SolClientReturnCode::from_i32(set_delivery_result) {
            return Err(MessageBuilderError::SolClientError);
        };

        // destination
        let Some(destination) = self.destination else{
            return Err(MessageBuilderError::MissingArgs("destination".to_owned()));
        };
        // destination is being copied by solClient_msg_setDestination
        // so it is fine to create a ptr for the destination.dest
        let mut destination: ffi::solClient_destination = ffi::solClient_destination {
            destType: destination.dest_type.to_i32(),
            dest: destination.dest.as_ptr(),
        };
        let set_destination_result = unsafe {
            ffi::solClient_msg_setDestination(
                msg_ptr,
                &mut destination,
                std::mem::size_of::<ffi::solClient_destination>(),
            )
        };
        if Some(SolClientReturnCode::Ok) != SolClientReturnCode::from_i32(set_destination_result) {
            return Err(MessageBuilderError::SolClientError);
        };

        // binary attachment string
        // We pass the ptr which is then copied over
        let Some(message) = self.message else{
            return Err(MessageBuilderError::MissingArgs("message".to_owned()));
        };

        let set_attachment_result = unsafe {
            ffi::solClient_msg_setBinaryAttachment(
                msg_ptr,
                message.as_ptr() as *const c_void,
                message.len() as u32,
            )
        };
        if Some(SolClientReturnCode::Ok) != SolClientReturnCode::from_i32(set_attachment_result) {
            return Err(MessageBuilderError::SolClientError);
        };

        // correlation_id
        if let Some(id) = self.correlation_id {
            let set_correlation_id_result =
                unsafe { ffi::solClient_msg_setCorrelationId(msg_ptr, CString::new(id)?.as_ptr()) };

            if Some(SolClientReturnCode::Ok)
                != SolClientReturnCode::from_i32(set_correlation_id_result)
            {
                return Err(MessageBuilderError::SolClientError);
            };
        }

        // Class of Service
        if let Some(cos) = self.class_of_service {
            let set_cos_result =
                unsafe { ffi::solClient_msg_setClassOfService(msg_ptr, cos.into()) };

            if Some(SolClientReturnCode::Ok) != SolClientReturnCode::from_i32(set_cos_result) {
                return Err(MessageBuilderError::SolClientError);
            };
        }

        // Sequence Number
        if let Some(seq_number) = self.seq_number {
            unsafe { ffi::solClient_msg_setSequenceNumber(msg_ptr, seq_number) };
        }

        // Priority
        if let Some(priority) = self.priority {
            unsafe { ffi::solClient_msg_setPriority(msg_ptr, priority.into()) };
        }

        // Application ID
        if let Some(id) = self.application_id {
            // application id is copied over
            unsafe {
                ffi::solClient_msg_setApplicationMessageId(msg_ptr, id.as_ptr() as *const i8)
            };
        }

        // Application Message Type
        if let Some(message_type) = self.application_msg_type {
            // application msg type is copied over
            unsafe {
                ffi::solClient_msg_setApplicationMsgType(
                    msg_ptr,
                    message_type.as_ptr() as *const i8,
                )
            };
        }

        Ok(OutboundMessage { msg_ptr })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{DestinationType, MessageDestination};

    #[test]
    fn it_should_build_message() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let _builder = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_payload("Hello");
    }

    #[test]
    fn it_should_build_with_same_topic() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_payload("Hello")
            .build()
            .unwrap();
        let message_destination = message.get_destination().unwrap().unwrap();

        assert!("test_topic" == message_destination.dest.to_string_lossy());
    }

    #[test]
    fn it_should_build_with_same_corralation_id() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_correlation_id("test_correlation")
            .set_payload("Hello")
            .build()
            .unwrap();

        let correlation_id = message.get_correlation_id().unwrap();

        assert!("test_correlation" == correlation_id);
    }

    #[test]
    fn it_should_build_have_valid_exp() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_payload("Hello")
            .build()
            .unwrap();

        assert!(0 == message.get_expiration());
    }

    #[test]
    fn it_should_build_with_same_cos() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_class_of_service(ClassOfService::Two)
            .set_payload("Hello")
            .build()
            .unwrap();

        assert!(ClassOfService::Two == message.get_class_of_service().unwrap());
    }

    #[test]
    fn it_should_build_with_same_seq_num() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_seq_number(45)
            .set_payload("Hello")
            .build()
            .unwrap();

        assert!(45 == message.get_sequence_number().unwrap().unwrap());
    }

    #[test]
    fn it_should_build_with_same_priority() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_priority(3)
            .set_payload("Hello")
            .build()
            .unwrap();

        assert!(3 == message.get_priority().unwrap().unwrap());

        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_payload("Hello")
            .build()
            .unwrap();

        assert!(message.get_priority().unwrap().is_none());
    }

    #[test]
    fn it_should_build_with_same_application_id() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_application_id("test_id")
            .set_payload("Hello")
            .build()
            .unwrap();

        assert!(Some("test_id") == message.get_application_message_id());

        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_payload("Hello")
            .build()
            .unwrap();

        assert!(message.get_application_message_id().is_none());
    }

    #[test]
    fn it_should_build_with_same_application_msg_type() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_application_msg_type("test_id")
            .set_payload("Hello")
            .build()
            .unwrap();

        assert!(Some("test_id") == message.get_application_msg_type());

        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_payload("Hello")
            .build()
            .unwrap();

        assert!(message.get_application_msg_type().is_none());
    }

    #[test]
    fn it_should_build_with_same_string_payload() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .set_delivery_mode(DeliveryMode::Direct)
            .set_destination(dest)
            .set_application_msg_type("test_id")
            .set_payload("Hello")
            .build()
            .unwrap();

        let raw_payload = message.get_payload().unwrap();

        assert!(b"Hello" == raw_payload);
    }
}
