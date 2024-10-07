use super::destination::MessageDestination;
use super::{ClassOfService, DeliveryMode, Message};
use crate::SolClientReturnCode;
use solace_rs_sys as ffi;
use std::ffi::{c_void, CString, NulError};
use std::ptr;
use std::time::SystemTime;
use thiserror::Error;
use tracing::warn;

#[derive(Error, Debug)]
pub enum MessageBuilderError {
    #[error("builder recieved invalid args")]
    InvalidArgs(#[from] NulError),
    #[error("{0} arg need to be set")]
    MissingRequiredArgs(String),
    #[error("{0} size need to be less than {1} found {2}")]
    SizeErrorArgs(String, usize, usize),
    #[error("timestamp needs to be greater than UNIX_EPOCH")]
    TimestampError,
    #[error("solClient message aloc failed")]
    MessageAlocFailure,
}

type Result<T> = std::result::Result<T, MessageBuilderError>;

pub struct OutboundMessage {
    _msg_ptr: ffi::solClient_opaqueMsg_pt,
}

unsafe impl Send for OutboundMessage {}

impl Drop for OutboundMessage {
    fn drop(&mut self) {
        let msg_free_result = unsafe { ffi::solClient_msg_free(&mut self._msg_ptr) };

        let rc = SolClientReturnCode::from_raw(msg_free_result);

        if !rc.is_ok() {
            warn!("message was not dropped properly");
        }
    }
}

impl<'a> Message<'a> for OutboundMessage {
    unsafe fn get_raw_message_ptr(&self) -> ffi::solClient_opaqueMsg_pt {
        self._msg_ptr
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
    user_data: Option<Vec<u8>>,
    sender_ts: Option<SystemTime>,
    eliding_eligible: Option<()>,
    is_reply: Option<()>,
}

impl OutboundMessageBuilder {
    /// Creates a new [`OutboundMessageBuilder`].
    pub fn new() -> Self {
        Self::default()
    }
    pub fn delivery_mode(mut self, mode: DeliveryMode) -> Self {
        self.delivery_mode = Some(mode);
        self
    }

    pub fn application_id<M>(mut self, application_id: M) -> Self
    where
        M: Into<Vec<u8>>,
    {
        self.application_id = Some(application_id.into());
        self
    }

    pub fn application_msg_type<M>(mut self, message_type: M) -> Self
    where
        M: Into<Vec<u8>>,
    {
        self.application_msg_type = Some(message_type.into());
        self
    }

    pub fn destination(mut self, destination: MessageDestination) -> Self {
        self.destination = Some(destination);
        self
    }

    pub fn class_of_service(mut self, cos: ClassOfService) -> Self {
        self.class_of_service = Some(cos);
        self
    }

    pub fn seq_number(mut self, seq_num: u64) -> Self {
        self.seq_number = Some(seq_num);
        self
    }

    pub fn sender_timestamp(mut self, ts: SystemTime) -> Self {
        self.sender_ts = Some(ts);
        self
    }

    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn is_reply(mut self, is_reply: bool) -> Self {
        if is_reply {
            self.is_reply = Some(());
        } else {
            self.is_reply = None
        }
        self
    }

    pub fn user_data<D>(mut self, data: D) -> Self
    where
        D: Into<Vec<u8>>,
    {
        self.user_data = Some(data.into());

        self
    }

    pub fn payload<M>(mut self, message: M) -> Self
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

    pub fn correlation_id<M>(mut self, id: M) -> Self
    where
        M: Into<Vec<u8>>,
    {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn eliding_eligible(mut self, eliding_eligible: bool) -> Self {
        if eliding_eligible {
            self.eliding_eligible = Some(());
        } else {
            self.eliding_eligible = None;
        }
        self
    }

    pub fn build(self) -> Result<OutboundMessage> {
        // message allocation
        let mut msg_ptr: ffi::solClient_opaqueMsg_pt = ptr::null_mut();
        let rc = unsafe { ffi::solClient_msg_alloc(&mut msg_ptr) };


        let rc = SolClientReturnCode::from_raw(rc);

        if !rc.is_ok() {
            return Err(MessageBuilderError::MessageAlocFailure);
        };

        // OutboundMessage is responsible for dropping the message in-case of any errors
        let msg = OutboundMessage { _msg_ptr: msg_ptr };

        // We do not check the return code for many of the setter functions since they only fail
        // on invalid msg_ptr. We validated the message ptr above, so no need to double check.

        // delivery_mode
        let Some(delivery_mode) = self.delivery_mode else {
            return Err(MessageBuilderError::MissingRequiredArgs(
                "delivery_mode".to_owned(),
            ));
        };
        unsafe { ffi::solClient_msg_setDeliveryMode(msg_ptr, delivery_mode as u32) };

        // destination
        let Some(destination) = self.destination else {
            return Err(MessageBuilderError::MissingRequiredArgs(
                "destination".to_owned(),
            ));
        };
        // destination is being copied by solClient_msg_setDestination
        // so it is fine to create a ptr for the destination.dest
        let mut destination: ffi::solClient_destination = ffi::solClient_destination {
            destType: destination.dest_type.to_i32(),
            dest: destination.dest.as_ptr(),
        };
        unsafe {
            ffi::solClient_msg_setDestination(
                msg_ptr,
                &mut destination,
                std::mem::size_of::<ffi::solClient_destination>(),
            )
        };

        if let Some(user_data) = self.user_data {
            if user_data.len()
                > ffi::SOLCLIENT_BUFINFO_MAX_USER_DATA_SIZE
                    .try_into()
                    .unwrap()
            {
                return Err(MessageBuilderError::SizeErrorArgs(
                    "user_data".to_owned(),
                    user_data.len(),
                    ffi::SOLCLIENT_BUFINFO_MAX_USER_DATA_SIZE
                        .try_into()
                        .unwrap(),
                ));
            }
            // We pass the ptr which is then copied over
            unsafe {
                ffi::solClient_msg_setUserData(
                    msg_ptr,
                    user_data.as_ptr() as *const c_void,
                    user_data.len() as u32,
                )
            };
        }

        // binary attachment
        // We pass the ptr which is then copied over
        let Some(message) = self.message else {
            return Err(MessageBuilderError::MissingRequiredArgs(
                "message".to_owned(),
            ));
        };
        unsafe {
            ffi::solClient_msg_setBinaryAttachment(
                msg_ptr,
                message.as_ptr() as *const c_void,
                message.len() as u32,
            )
        };

        // correlation_id
        if let Some(id) = self.correlation_id {
            // correlation_id is copied over
            unsafe { ffi::solClient_msg_setCorrelationId(msg_ptr, CString::new(id)?.as_ptr()) };
        }

        // Class of Service
        if let Some(cos) = self.class_of_service {
            unsafe { ffi::solClient_msg_setClassOfService(msg_ptr, cos.into()) };
        }

        // Sequence Number
        if let Some(seq_number) = self.seq_number {
            unsafe { ffi::solClient_msg_setSequenceNumber(msg_ptr, seq_number) };
        }

        // Priority
        if let Some(priority) = self.priority {
            unsafe { ffi::solClient_msg_setPriority(msg_ptr, priority.into()) };
        }

        // Sender timestamp
        if let Some(ts) = self.sender_ts {
            let ts = ts
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|_| MessageBuilderError::TimestampError)?;
            let ts: i64 = ts
                .as_millis()
                .try_into()
                .map_err(|_| MessageBuilderError::TimestampError)?;

            unsafe { ffi::solClient_msg_setSenderTimestamp(msg_ptr, ts) };
        }

        // Application ID
        if let Some(id) = self.application_id {
            // application id is copied over
            unsafe {
                ffi::solClient_msg_setApplicationMessageId(msg_ptr, CString::new(id)?.as_ptr())
            };
        }

        // Application Message Type
        if let Some(message_type) = self.application_msg_type {
            // application msg type is copied over
            unsafe {
                ffi::solClient_msg_setApplicationMsgType(
                    msg_ptr,
                    CString::new(message_type)?.as_ptr(),
                )
            };
        }

        if self.eliding_eligible.is_some() {
            unsafe { ffi::solClient_msg_setElidingEligible(msg_ptr, true.into()) };
        }

        if self.is_reply.is_some() {
            unsafe { ffi::solClient_msg_setAsReplyMsg(msg_ptr, true.into()) };
        }

        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{DestinationType, MessageDestination};

    #[test]
    fn it_should_build_message() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let _ = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .build()
            .unwrap();
    }

    #[test]
    fn it_should_build_with_eliding_eligible() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let non_elided_msg = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .eliding_eligible(false)
            .build()
            .unwrap();

        assert!(!non_elided_msg.is_eliding_eligible());

        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let elided_msg = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .eliding_eligible(true)
            .build()
            .unwrap();

        assert!(elided_msg.is_eliding_eligible());
    }

    #[test]
    fn it_should_build_with_is_reply() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let non_reply_msg = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .is_reply(false)
            .build()
            .unwrap();

        assert!(!non_reply_msg.is_reply());

        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let reply_msg = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .is_reply(true)
            .build()
            .unwrap();

        assert!(reply_msg.is_reply());
    }

    #[test]
    fn it_should_build_with_same_topic() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .build()
            .unwrap();
        let message_destination = message.get_destination().unwrap().unwrap();

        assert!("test_topic" == message_destination.dest.to_string_lossy());
    }

    #[test]
    fn it_should_build_with_same_corralation_id() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .correlation_id("test_correlation")
            .payload("Hello")
            .build()
            .unwrap();

        let correlation_id = message.get_correlation_id().unwrap().unwrap();

        assert!("test_correlation" == correlation_id);
    }

    #[test]
    fn it_should_build_have_valid_exp() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .build()
            .unwrap();

        assert!(0 == message.get_expiration());
    }

    #[test]
    fn it_should_build_with_same_cos() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .class_of_service(ClassOfService::Two)
            .payload("Hello")
            .build()
            .unwrap();

        assert!(ClassOfService::Two == message.get_class_of_service().unwrap());
    }

    #[test]
    fn it_should_build_with_same_seq_num() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .seq_number(45)
            .payload("Hello")
            .build()
            .unwrap();

        assert!(45 == message.get_sequence_number().unwrap().unwrap());
    }

    #[test]
    fn it_should_build_with_same_priority() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .priority(3)
            .payload("Hello")
            .build()
            .unwrap();

        assert!(3 == message.get_priority().unwrap().unwrap());

        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .build()
            .unwrap();

        assert!(message.get_priority().unwrap().is_none());
    }

    #[test]
    fn it_should_build_with_same_application_id() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .application_id("test_id")
            .payload("Hello")
            .build()
            .unwrap();

        assert!(Some("test_id") == message.get_application_message_id());

        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .build()
            .unwrap();

        assert!(message.get_application_message_id().is_none());
    }

    #[test]
    fn it_should_build_with_same_application_msg_type() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .application_msg_type("test_id")
            .payload("Hello")
            .build()
            .unwrap();

        assert!(Some("test_id") == message.get_application_msg_type());

        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .build()
            .unwrap();

        assert!(message.get_application_msg_type().is_none());
    }

    #[test]
    fn it_should_build_with_same_string_payload() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .build()
            .unwrap();

        let raw_payload = message.get_payload().unwrap().unwrap();

        assert!(b"Hello" == raw_payload);
    }

    #[test]
    fn it_should_build_with_same_user_data() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .user_data(32_u32.to_be_bytes())
            .build()
            .unwrap();

        let raw_user_data = message.get_user_data().unwrap().unwrap();

        assert!(32_u32.to_be_bytes() == raw_user_data);
    }

    #[test]
    fn it_should_build_with_same_sender_timestamp() {
        let dest = MessageDestination::new(DestinationType::Topic, "test_topic").unwrap();
        let now = SystemTime::now();
        let message = OutboundMessageBuilder::new()
            .delivery_mode(DeliveryMode::Direct)
            .destination(dest)
            .payload("Hello")
            .sender_timestamp(now)
            .build()
            .unwrap();

        let ts = message.get_sender_timestamp().unwrap().unwrap();

        let now = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let ts = ts
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        assert!(now == ts);
    }
}
