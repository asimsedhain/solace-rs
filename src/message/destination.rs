use super::Result;
use crate::SolaceError;
use enum_primitive::*;
use solace_rs_sys as ffi;
use std::convert::From;
use std::ffi::{CStr, CString};

enum_from_primitive! {
    #[derive(Debug, PartialEq, Default)]
    #[repr(i32)]
    pub enum DestinationType {
        #[default]
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
#[derive(Debug)]
pub struct MessageDestination {
    pub dest_type: DestinationType,
    pub dest: CString,
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

impl From<ffi::solClient_destination> for MessageDestination {
    fn from(raw_dest: ffi::solClient_destination) -> Self {
        let dest_type = DestinationType::from_i32(raw_dest.destType).unwrap_or_default();

        let dest_cstr = unsafe { CStr::from_ptr(raw_dest.dest) };
        let dest: CString = dest_cstr.into();

        MessageDestination { dest_type, dest }
    }
}
