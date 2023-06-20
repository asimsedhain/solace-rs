use super::Result;
use crate::solace::ffi;
use crate::SolaceError;
use enum_primitive::*;
use std::ffi::CString;

enum_from_primitive! {
    #[derive(Debug, PartialEq)]
    #[repr(i32)]
    pub enum DestinationType {
        Null=ffi::solClient_destinationType_SOLCLIENT_NULL_DESTINATION,
        Topic=ffi::solClient_destinationType_SOLCLIENT_TOPIC_DESTINATION,
        Queue=ffi::solClient_destinationType_SOLCLIENT_QUEUE_DESTINATION,
        TopicTemp=ffi::solClient_destinationType_SOLCLIENT_TOPIC_TEMP_DESTINATION,
        QueueTemp=ffi::solClient_destinationType_SOLCLIENT_QUEUE_TEMP_DESTINATION,
    }
}

impl DestinationType {
    pub fn to_i32(&self) -> i32 {
        match self {
            Self::Null => ffi::solClient_destinationType_SOLCLIENT_NULL_DESTINATION,
            Self::Topic => ffi::solClient_destinationType_SOLCLIENT_TOPIC_TEMP_DESTINATION,
            Self::Queue => ffi::solClient_destinationType_SOLCLIENT_QUEUE_DESTINATION,
            Self::TopicTemp => ffi::solClient_destinationType_SOLCLIENT_TOPIC_TEMP_DESTINATION,
            Self::QueueTemp => ffi::solClient_destinationType_SOLCLIENT_QUEUE_TEMP_DESTINATION,
        }
    }
}

// rethink about this api
// we need to be able to create a new owned MessageDestination
// then pass that as value to the message builder
// but then also be able to get a MessageDestination from a message.
// Right now, it seems the best way to do that is with by copying the meessage destination field.
pub struct MessageDestination {
    pub(super) dest_type: DestinationType,
    pub(super) dest: CString,
}

impl MessageDestination {
    pub fn new<T: Into<Vec<u8>>>(dest_type: DestinationType, destination: T) -> Result<Self> {
        let c_destination = CString::new(destination).map_err(|_| SolaceError)?;

        Ok(MessageDestination {
            dest_type,
            dest: c_destination,
        })
    }
}
