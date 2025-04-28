use solace_rs_sys as ffi;
use std::{
    ffi::{CString, NulError},
    ptr,
};
use thiserror::Error;

use crate::util::bool_to_ptr;

type Result<T> = std::result::Result<T, EndpointPropsBuilderError>;

#[derive(Error, Debug)]
pub enum EndpointPropsBuilderError {
    #[error("builder received invalid args")]
    InvalidArgs(#[from] NulError),
    #[error("{0} arg need to be set")]
    MissingRequiredArgs(String),
    #[error("{0} size need to be less than {1} found {2}")]
    SizeErrorArgs(String, usize, usize),
}

/// Endpoint Configuration Properties
/// https://docs.solace.com/API-Developer-Online-Ref-Documentation/c/group__endpoint_props.html
pub struct EndpointPropsBuilder {
    id: Option<EndpointId<String>>,
    durable: Option<bool>,
    permission: Option<EndpointPermission>,
    access_type: Option<EndpointAccessType>,
    quota_mb: Option<u64>,
    max_msg_size: Option<u64>,
    respects_msg_ttl: Option<bool>,
    discard_behavior: Option<EndpointDiscardBehavior>,
    max_msg_redelivery: Option<u64>,
}

impl EndpointPropsBuilder {
    /// Creates a new `EndpointPropsBuilder` with default properties.
    pub fn new() -> Self {
        Self {
            id: None,
            durable: None,
            permission: None,
            access_type: None,
            quota_mb: None,
            max_msg_size: None,
            respects_msg_ttl: None,
            discard_behavior: None,
            max_msg_redelivery: None,
        }
    }

    /// Sets the type of endpoint.
    ///
    /// The valid values are SOLCLIENT_ENDPOINT_PROP_QUEUE, SOLCLIENT_ENDPOINT_PROP_TE, and SOLCLIENT_ENDPOINT_PROP_CLIENT_NAME.
    /// Default: [ffi::SOLCLIENT_ENDPOINT_PROP_TE]
    pub fn id(mut self, id: EndpointId<String>) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the durability of the endpoint.
    ///
    /// Default: [ffi::SOLCLIENT_PROP_ENABLE_VAL], which means the endpoint is durable. Only SOLCLIENT_PROP_ENABLE_VAL is supported in solClient_session_endpointProvision().
    pub fn durable(mut self, durable: bool) -> Self {
        self.durable = Some(durable);
        self
    }

    /// Sets the permissions for the created entity.
    ///
    /// Permissions can be SOLCLIENT_ENDPOINT_PERM_DELETE, SOLCLIENT_ENDPOINT_PERM_MODIFY_TOPIC, SOLCLIENT_ENDPOINT_PERM_CONSUME, SOLCLIENT_ENDPOINT_PERM_READ_ONLY, SOLCLIENT_ENDPOINT_PERM_NONE.
    pub fn permission(mut self, permission: EndpointPermission) -> Self {
        self.permission = Some(permission);
        self
    }

    /// Sets the access type for the endpoint.
    ///
    /// This applies to durable Queues only.
    pub fn access_type(mut self, access_type: EndpointAccessType) -> Self {
        self.access_type = Some(access_type);
        self
    }

    /// Sets the maximum quota (in megabytes) for the endpoint.
    ///
    /// A value of 0 configures the endpoint to act as a Last-Value-Queue (LVQ), where the broker enforces a Queue depth of one.
    pub fn quota_mb(mut self, quota_mb: u64) -> Self {
        self.quota_mb = Some(quota_mb);
        self
    }

    /// Sets the maximum size (in bytes) for any one message stored in the endpoint.
    pub fn max_msg_size(mut self, max_msg_size: u64) -> Self {
        self.max_msg_size = Some(max_msg_size);
        self
    }

    /// Configures the endpoint to observe message Time-to-Live (TTL) values and remove expired messages.
    ///
    /// Default: [ffi::SOLCLIENT_ENDPOINT_PROP_DEFAULT_RESPECTS_MSG_TTL]
    pub fn respects_msg_ttl(mut self, respects_msg_ttl: bool) -> Self {
        self.respects_msg_ttl = Some(respects_msg_ttl);
        self
    }

    /// Sets the discard behavior for the endpoint.
    ///
    /// When a message cannot be added to an endpoint (e.g., maximum quota exceeded), this property controls the action the broker will perform towards the publisher.
    pub fn discard_behavior(mut self, discard_behavior: EndpointDiscardBehavior) -> Self {
        self.discard_behavior = Some(discard_behavior);
        self
    }

    /// Defines how many message redelivery retries before discarding or moving the message to the DMQ.
    ///
    /// The valid range is {0..255}, where 0 means retry forever. Default: 0
    pub fn max_msg_redelivery(mut self, max_msg_redelivery: u64) -> Self {
        self.max_msg_redelivery = Some(max_msg_redelivery);
        self
    }

    pub fn build(self) -> Result<EndpointProps> {
        let id = self.id.map(|i| i.try_into()).transpose()?;
        let durable = self.durable;
        let permission = self.permission;
        let access_type = self.access_type;
        let quota_mb = self
            .quota_mb
            .map(|q| CString::new(q.to_string()))
            .transpose()?;
        let max_msg_size = self
            .max_msg_size
            .map(|m| CString::new(m.to_string()))
            .transpose()?;
        let respects_msg_ttl = self.respects_msg_ttl;
        let discard_behavior = self.discard_behavior;
        let max_msg_redelivery = self
            .max_msg_redelivery
            .map(|m| CString::new(m.to_string()))
            .transpose()?;

        Ok(EndpointProps {
            id,
            durable,
            permission,
            access_type,
            quota_mb,
            max_msg_size,
            respects_msg_ttl,
            discard_behavior,
            max_msg_redelivery,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EndpointProps {
    id: Option<EndpointId<CString>>,
    durable: Option<bool>,
    permission: Option<EndpointPermission>,
    access_type: Option<EndpointAccessType>,
    quota_mb: Option<CString>,
    max_msg_size: Option<CString>,
    respects_msg_ttl: Option<bool>,
    discard_behavior: Option<EndpointDiscardBehavior>,
    max_msg_redelivery: Option<CString>,
}

impl EndpointProps {
    pub fn to_raw(&self) -> Vec<*const i8> {
        let mut props = vec![];

        if let Some(id) = &self.id {
            props.push(ffi::SOLCLIENT_ENDPOINT_PROP_ID.as_ptr() as *const i8);
            match id {
                EndpointId::Queue { name } => {
                    props.push(ffi::SOLCLIENT_ENDPOINT_PROP_QUEUE.as_ptr() as *const i8);
                    props.push(ffi::SOLCLIENT_ENDPOINT_PROP_NAME.as_ptr() as *const i8);
                    props.push(name.as_ptr());
                }
                EndpointId::Te { name } => {
                    props.push(ffi::SOLCLIENT_ENDPOINT_PROP_TE.as_ptr() as *const i8);
                    props.push(ffi::SOLCLIENT_ENDPOINT_PROP_NAME.as_ptr() as *const i8);
                    props.push(name.as_ptr());
                }
                EndpointId::ClientName { name } => {
                    props.push(ffi::SOLCLIENT_ENDPOINT_PROP_CLIENT_NAME.as_ptr() as *const i8);
                    props.push(ffi::SOLCLIENT_ENDPOINT_PROP_NAME.as_ptr() as *const i8);
                    props.push(name.as_ptr());
                }
            }
        }

        if let Some(durable) = self.durable {
            props.push(ffi::SOLCLIENT_ENDPOINT_PROP_DURABLE.as_ptr() as *const i8);
            props.push(bool_to_ptr(durable));
        }

        if let Some(permission) = &self.permission {
            props.push(ffi::SOLCLIENT_ENDPOINT_PROP_PERMISSION.as_ptr() as *const i8);
            props.push(permission.as_ptr());
        }

        if let Some(access_type) = &self.access_type {
            props.push(ffi::SOLCLIENT_ENDPOINT_PROP_ACCESSTYPE.as_ptr() as *const i8);
            props.push(access_type.as_ptr());
        }

        if let Some(quota_mb) = &self.quota_mb {
            props.push(ffi::SOLCLIENT_ENDPOINT_PROP_QUOTA_MB.as_ptr() as *const i8);
            props.push(quota_mb.as_ptr() as *const i8);
        }

        if let Some(max_msg_size) = &self.max_msg_size {
            props.push(ffi::SOLCLIENT_ENDPOINT_PROP_MAXMSG_SIZE.as_ptr() as *const i8);
            props.push(max_msg_size.as_ptr() as *const i8);
        }

        if let Some(respects_msg_ttl) = self.respects_msg_ttl {
            props.push(ffi::SOLCLIENT_ENDPOINT_PROP_RESPECTS_MSG_TTL.as_ptr() as *const i8);
            props.push(bool_to_ptr(respects_msg_ttl));
        }

        if let Some(discard_behavior) = &self.discard_behavior {
            props.push(ffi::SOLCLIENT_ENDPOINT_PROP_DISCARD_BEHAVIOR.as_ptr() as *const i8);
            props.push(discard_behavior.as_ptr());
        }

        if let Some(max_msg_redelivery) = &self.max_msg_redelivery {
            props.push(ffi::SOLCLIENT_ENDPOINT_PROP_MAXMSG_REDELIVERY.as_ptr() as *const i8);
            props.push(max_msg_redelivery.as_ptr() as *const i8);
        }

        props.push(ptr::null());

        props
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EndpointId<T> {
    Queue { name: T },
    Te { name: T },
    ClientName { name: T },
}
impl TryFrom<EndpointId<String>> for EndpointId<CString> {
    type Error = EndpointPropsBuilderError;

    fn try_from(value: EndpointId<String>) -> Result<Self> {
        match value {
            EndpointId::Queue { name } => Ok(EndpointId::Queue {
                name: CString::new(name)?,
            }),
            EndpointId::Te { name } => Ok(EndpointId::Te {
                name: CString::new(name)?,
            }),
            EndpointId::ClientName { name } => Ok(EndpointId::ClientName {
                name: CString::new(name)?,
            }),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EndpointPermission {
    Delete,
    ModifyTopic,
    Consume,
    ReadOnly,
    None,
}
impl EndpointPermission {
    pub fn as_ptr(&self) -> *const i8 {
        match self {
            Self::Delete => ffi::SOLCLIENT_ENDPOINT_PERM_DELETE,
            Self::ModifyTopic => ffi::SOLCLIENT_ENDPOINT_PERM_MODIFY_TOPIC,
            Self::Consume => ffi::SOLCLIENT_ENDPOINT_PERM_CONSUME,
            Self::ReadOnly => ffi::SOLCLIENT_ENDPOINT_PERM_READ_ONLY,
            Self::None => ffi::SOLCLIENT_ENDPOINT_PERM_NONE,
        }
        .as_ptr() as *const i8
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EndpointAccessType {
    Exclusive,
    NonExclusive,
}
impl EndpointAccessType {
    pub fn as_ptr(&self) -> *const i8 {
        match self {
            Self::Exclusive => ffi::SOLCLIENT_ENDPOINT_PROP_ACCESSTYPE_EXCLUSIVE,
            Self::NonExclusive => ffi::SOLCLIENT_ENDPOINT_PROP_ACCESSTYPE_NONEXCLUSIVE,
        }
        .as_ptr() as *const i8
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EndpointDiscardBehavior {
    DiscardNotifySenderOff,
    DiscardNotifySenderOn,
}
impl EndpointDiscardBehavior {
    pub fn as_ptr(&self) -> *const i8 {
        match self {
            Self::DiscardNotifySenderOff => ffi::SOLCLIENT_ENDPOINT_PROP_DISCARD_NOTIFY_SENDER_OFF,
            Self::DiscardNotifySenderOn => ffi::SOLCLIENT_ENDPOINT_PROP_DISCARD_NOTIFY_SENDER_ON,
        }
        .as_ptr() as *const i8
    }
}
