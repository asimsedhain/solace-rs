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
    #[error("builder recieved invalid args")]
    InvalidArgs(#[from] NulError),
    #[error("{0} arg need to be set")]
    MissingRequiredArgs(String),
    #[error("{0} size need to be less than {1} found {2}")]
    SizeErrorArgs(String, usize, usize),
}

/// Endpoint Configuration Properties
/// https://docs.solace.com/API-Developer-Online-Ref-Documentation/c/group__endpoint_props.html
pub struct EndpointPropsBuilder<Id, Name> {
    // Note: required params
    // In the future we can use type state pattern to always force clients to provide these params
    id: Option<Id>,
    name: Option<Name>,

    durable: Option<bool>,
    permission: Option<EndpointPermission>,
    access_type: Option<EndpointAccessType>,
    quota_mb: Option<u64>,
    max_msg_size: Option<u64>,
    respects_msg_ttl: Option<bool>,
    discard_behavior: Option<EndpointDiscardBehavior>,
    max_msg_redelivery: Option<u64>,
}

impl<Id, Name> EndpointPropsBuilder<Id, Name>
where
    Id: Into<EndpointId>,
    Name: Into<String>,
{
    /// Creates a new `EndpointPropsBuilder` with default properties.
    pub fn new() -> Self {
        Self {
            id: None,
            name: None,
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

    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    pub fn name(mut self, name: Name) -> Self {
        self.name = Some(name);
        self
    }

    pub fn durable(mut self, durable: bool) -> Self {
        self.durable = Some(durable);
        self
    }
    pub fn permission(mut self, permission: EndpointPermission) -> Self {
        self.permission = Some(permission);
        self
    }
    pub fn access_type(mut self, access_type: EndpointAccessType) -> Self {
        self.access_type = Some(access_type);
        self
    }
    pub fn quota_mb(mut self, quota_mb: u64) -> Self {
        self.quota_mb = Some(quota_mb);
        self
    }
    pub fn max_msg_size(mut self, max_msg_size: u64) -> Self {
        self.max_msg_size = Some(max_msg_size);
        self
    }
    pub fn respects_msg_ttl(mut self, respects_msg_ttl: bool) -> Self {
        self.respects_msg_ttl = Some(respects_msg_ttl);
        self
    }
    pub fn discard_behavior(mut self, discard_behavior: EndpointDiscardBehavior) -> Self {
        self.discard_behavior = Some(discard_behavior);
        self
    }
    pub fn max_msg_redelivery(mut self, max_msg_redelivery: u64) -> Self {
        self.max_msg_redelivery = Some(max_msg_redelivery);
        self
    }

    pub fn build(self) -> Result<EndpointProps> {
        println!("Building EndpointProps");

        let id = match self.id {
            Some(id) => id.into(),
            None => {
                return Err(EndpointPropsBuilderError::MissingRequiredArgs(
                    "id".to_string(),
                ))
            }
        };
        let name = match self.name {
            Some(name) => CString::new(name.into())?,
            None => {
                return Err(EndpointPropsBuilderError::MissingRequiredArgs(
                    "name".to_string(),
                ))
            }
        };
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
            name,
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

#[derive(Debug, Clone)]
pub struct EndpointProps {
    id: EndpointId,
    name: CString,

    // Note: optional params
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
        let mut props = vec![
            ffi::SOLCLIENT_ENDPOINT_PROP_ID.as_ptr() as *const i8,
            self.id.as_ptr(),
            ffi::SOLCLIENT_ENDPOINT_PROP_NAME.as_ptr() as *const i8,
            self.name.as_ptr() as *const i8,
        ];

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

#[derive(Debug, PartialEq, Eq, Default, Clone)]
pub enum EndpointId {
    Queue,
    #[default]
    Te,
    ClientName,
}
impl EndpointId {
    fn as_ptr(&self) -> *const i8 {
        match self {
            Self::Queue => ffi::SOLCLIENT_ENDPOINT_PROP_QUEUE,
            Self::Te => ffi::SOLCLIENT_ENDPOINT_PROP_TE,
            Self::ClientName => ffi::SOLCLIENT_ENDPOINT_PROP_CLIENT_NAME,
        }
        .as_ptr() as *const i8
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
