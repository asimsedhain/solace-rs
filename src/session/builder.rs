use solace_rs_sys as ffi;
use std::{
    ffi::{CString, NulError},
    marker::PhantomData,
    mem, ptr,
};

use crate::{
    message::InboundMessage,
    session::SessionEvent,
    util::{
        get_last_error_info, on_event_trampoline, on_message_trampoline, static_no_op_on_event,
        static_no_op_on_message,
    },
    Context, Session, SolClientReturnCode, SolClientSubCode,
};

#[derive(thiserror::Error, Debug)]
pub enum SessionBuilderError {
    #[error("session failed to initialize. SolClient return code: {0} subcode: {1}")]
    InitializationFailure(SolClientReturnCode, SolClientSubCode),
    #[error("session failed to connect. SolClient return code: {0} subcode: {1}")]
    ConnectionFailure(SolClientReturnCode, SolClientSubCode),
    #[error("arg contains interior nul byte")]
    InvalidArgs(#[from] NulError),
    #[error("{0} arg need to be set")]
    MissingRequiredArgs(String),
    #[error("{0} valid range is {1} foound {2}")]
    InvalidRange(String, String, String),
}

type Result<T> = std::result::Result<T, SessionBuilderError>;

fn bool_to_ptr(b: bool) -> *const i8 {
    if b {
        ffi::SOLCLIENT_PROP_ENABLE_VAL.as_ptr() as *const i8
    } else {
        ffi::SOLCLIENT_PROP_DISABLE_VAL.as_ptr() as *const i8
    }
}

struct UncheckedSessionProps<Host, Vpn, Username, Password> {
    // Note: required params
    // In the future we can use type state pattern to always force clients to provide these params
    host_name: Option<Host>,
    vpn_name: Option<Vpn>,
    username: Option<Username>,
    password: Option<Password>,

    // Note: optional params
    buffer_size_bytes: Option<u64>,
    block_write_timeout_ms: Option<u64>,
    connect_timeout_ms: Option<u64>,
    subconfirm_timeout_ms: Option<u64>,
    ignore_dup_subscription_error: Option<bool>,
    tcp_nodelay: Option<bool>,
    socket_send_buf_size_bytes: Option<u64>,
    socket_rcv_buf_size_bytes: Option<u64>,
    keep_alive_interval_ms: Option<u64>,
    keep_alive_limit: Option<u64>,
    application_description: Option<Vec<u8>>,
    client_name: Option<Vec<u8>>,
    compression_level: Option<u8>,
    generate_rcv_timestamps: Option<bool>,
    generate_send_timestamp: Option<bool>,
    generate_sender_id: Option<bool>,
    generate_sender_sequence_number: Option<bool>,
    connect_retries_per_host: Option<i64>,
    connect_retries: Option<i64>,
    reconnect_retries: Option<i64>,
    reconnect_retry_wait_ms: Option<u64>,
    reapply_subscriptions: Option<bool>,
    provision_timeout_ms: Option<u64>,
    calculate_message_expiration: Option<bool>,
    no_local: Option<bool>,
    modifyprop_timeout_ms: Option<u64>,
    ssl_trust_store_dir: Option<Vec<u8>>,

    // TODO: need to check if some of these params will break other assumptions
    // ex: we might check for ok status on send but if send_blocking is set to false
    // it will return can_block which will be assumed as an error

    // Note: below params has not exposed
    // TODO: check if we should even expose this
    #[allow(dead_code)]
    send_blocking: Option<bool>,
    #[allow(dead_code)]
    subscribe_blocking: Option<bool>,
    #[allow(dead_code)]
    block_while_connecting: Option<bool>,

    // TODO: probably should expose this through some other way
    // maybe a feature flag for the library
    #[allow(dead_code)]
    topic_dispatch: Option<bool>,
}

impl<Host, Vpn, Username, Password> Default
    for UncheckedSessionProps<Host, Vpn, Username, Password>
{
    fn default() -> Self {
        Self {
            host_name: None,
            vpn_name: None,
            username: None,
            password: None,
            buffer_size_bytes: None,
            block_write_timeout_ms: None,
            connect_timeout_ms: None,
            subconfirm_timeout_ms: None,
            ignore_dup_subscription_error: None,
            tcp_nodelay: None,
            socket_send_buf_size_bytes: None,
            socket_rcv_buf_size_bytes: None,
            keep_alive_interval_ms: None,
            keep_alive_limit: None,
            application_description: None,
            client_name: None,
            compression_level: None,
            generate_rcv_timestamps: None,
            generate_send_timestamp: None,
            generate_sender_id: None,
            generate_sender_sequence_number: None,
            connect_retries_per_host: None,
            connect_retries: None,
            reconnect_retries: None,
            reconnect_retry_wait_ms: None,
            reapply_subscriptions: None,
            provision_timeout_ms: None,
            calculate_message_expiration: None,
            no_local: None,
            modifyprop_timeout_ms: None,
            send_blocking: None,
            subscribe_blocking: None,
            block_while_connecting: None,
            topic_dispatch: None,
            ssl_trust_store_dir: None,
        }
    }
}

/// `SessionBuilder` allows setting up a session with customizable options that are not exposed by
/// the `session` function such as buffer size, timeouts, and more.
///
/// For more detailed documentation on all the configuration field, refer to [the official library documentation](https://docs.solace.com/API-Developer-Online-Ref-Documentation/c/group___session_props.html).
pub struct SessionBuilder<Host, Vpn, Username, Password, OnMessage, OnEvent> {
    context: Context,
    props: UncheckedSessionProps<Host, Vpn, Username, Password>,

    // callbacks
    on_message: Option<OnMessage>,
    on_event: Option<OnEvent>,
}

impl<Host, Vpn, Username, Password, OnMessage, OnEvent>
    SessionBuilder<Host, Vpn, Username, Password, OnMessage, OnEvent>
{
    pub(crate) fn new(context: Context) -> Self {
        Self {
            context,
            props: UncheckedSessionProps::default(),
            on_message: None,
            on_event: None,
        }
    }
}

impl<'session, Host, Vpn, Username, Password, OnMessage, OnEvent>
    SessionBuilder<Host, Vpn, Username, Password, OnMessage, OnEvent>
where
    Host: Into<Vec<u8>>,
    Vpn: Into<Vec<u8>>,
    Username: Into<Vec<u8>>,
    Password: Into<Vec<u8>>,
    OnMessage: FnMut(InboundMessage) + Send + 'session,
    OnEvent: FnMut(SessionEvent) + Send + 'session,
{
    pub fn build(mut self) -> Result<Session<'session, OnMessage, OnEvent>> {
        let config = CheckedSessionProps::try_from(mem::take(&mut self.props))?;

        // Session props is a **char in C
        // it takes in an array of key and values
        // first we specify the key, then the value
        // Session also copies over the props and maintains a copy internally.
        // Note: Needs to live long enough for the values to be copied
        let mut session_pt: ffi::solClient_opaqueSession_pt = ptr::null_mut();

        // Box::into_raw(Box::new(Box::new(f))) as *mut _
        // need to box it twice
        // first box will result in a fat pointer
        // causing a seg fault when dereffing in C land.
        // leaking is also fine since the lifetime of the closure is set to be the lifetime of the
        // session
        let (static_on_message_callback, user_on_message, msg_func_ptr) = match self.on_message {
            Some(f) => {
                let tramp = on_message_trampoline(&f);
                let mut func = Box::new(Box::new(f));
                (tramp, func.as_mut() as *const _ as *mut _, Some(func))
            }
            _ => (
                Some(static_no_op_on_message as unsafe extern "C" fn(_, _, _) -> u32),
                ptr::null_mut(),
                None,
            ),
        };

        let (static_on_event_callback, user_on_event, event_func_ptr) = match self.on_event {
            Some(f) => {
                let tramp = on_event_trampoline(&f);
                let mut func = Box::new(Box::new(f));
                (tramp, func.as_mut() as *const _ as *mut _, Some(func))
            }
            _ => (
                Some(static_no_op_on_event as unsafe extern "C" fn(_, _, _)),
                ptr::null_mut(),
                None,
            ),
        };

        // Function information for Session creation.
        // The application must set the eventInfo callback information. All Sessions must have an event callback registered.
        let mut session_func_info: ffi::solClient_session_createFuncInfo_t =
            ffi::solClient_session_createFuncInfo {
                rxInfo: ffi::solClient_session_createRxCallbackFuncInfo {
                    callback_p: ptr::null_mut(),
                    user_p: ptr::null_mut(),
                },
                eventInfo: ffi::solClient_session_createEventCallbackFuncInfo {
                    callback_p: static_on_event_callback,
                    user_p: user_on_event,
                },
                rxMsgInfo: ffi::solClient_session_createRxMsgCallbackFuncInfo {
                    callback_p: static_on_message_callback,
                    user_p: user_on_message,
                },
            };

        let mut raw = config.to_raw();
        let context_ptr = self.context.raw.lock().unwrap();
        let session_create_raw_rc = unsafe {
            ffi::solClient_session_create(
                raw.as_mut_ptr(),
                context_ptr.ctx,
                &mut session_pt,
                &mut session_func_info,
                std::mem::size_of::<ffi::solClient_session_createFuncInfo_t>(),
            )
        };
        drop(context_ptr);

        let rc = SolClientReturnCode::from_raw(session_create_raw_rc);

        if !rc.is_ok() {
            let subcode = get_last_error_info();
            return Err(SessionBuilderError::InitializationFailure(rc, subcode));
        }

        let connection_raw_rc = unsafe { ffi::solClient_session_connect(session_pt) };

        let rc = SolClientReturnCode::from_raw(connection_raw_rc);
        if rc.is_ok() {
            Ok(Session {
                _msg_fn_ptr: msg_func_ptr,
                _event_fn_ptr: event_func_ptr,
                _session_ptr: session_pt,
                context: self.context,
                lifetime: PhantomData,
            })
        } else {
            let subcode = get_last_error_info();
            Err(SessionBuilderError::ConnectionFailure(rc, subcode))
        }
    }

    pub fn host_name(mut self, host_name: Host) -> Self {
        self.props.host_name = Some(host_name);
        self
    }

    pub fn vpn_name(mut self, vpn_name: Vpn) -> Self {
        self.props.vpn_name = Some(vpn_name);
        self
    }
    pub fn username(mut self, username: Username) -> Self {
        self.props.username = Some(username);
        self
    }
    pub fn password(mut self, password: Password) -> Self {
        self.props.password = Some(password);
        self
    }

    pub fn on_message(mut self, on_message: OnMessage) -> Self {
        self.on_message = Some(on_message);
        self
    }

    pub fn on_event(mut self, on_event: OnEvent) -> Self {
        self.on_event = Some(on_event);
        self
    }

    pub fn buffer_size_bytes(mut self, buffer_size_bytes: u64) -> Self {
        self.props.buffer_size_bytes = Some(buffer_size_bytes);
        self
    }
    pub fn block_write_timeout_ms(mut self, write_timeout_ms: u64) -> Self {
        self.props.block_write_timeout_ms = Some(write_timeout_ms);
        self
    }
    pub fn connect_timeout_ms(mut self, connect_timeout_ms: u64) -> Self {
        self.props.connect_timeout_ms = Some(connect_timeout_ms);
        self
    }
    pub fn subconfirm_timeout_ms(mut self, subconfirm_timeout_ms: u64) -> Self {
        self.props.subconfirm_timeout_ms = Some(subconfirm_timeout_ms);
        self
    }
    pub fn ignore_dup_subscription_error(mut self, ignore_dup_subscription_error: bool) -> Self {
        self.props.ignore_dup_subscription_error = Some(ignore_dup_subscription_error);
        self
    }
    pub fn tcp_nodelay(mut self, tcp_nodelay: bool) -> Self {
        self.props.tcp_nodelay = Some(tcp_nodelay);
        self
    }
    pub fn socket_send_buf_size_bytes(mut self, socket_send_buf_size_bytes: u64) -> Self {
        self.props.socket_send_buf_size_bytes = Some(socket_send_buf_size_bytes);
        self
    }
    pub fn socket_rcv_buf_size_bytes(mut self, socket_rcv_buf_size_bytes: u64) -> Self {
        self.props.socket_rcv_buf_size_bytes = Some(socket_rcv_buf_size_bytes);
        self
    }
    pub fn keep_alive_interval_ms(mut self, keep_alive_interval_ms: u64) -> Self {
        self.props.keep_alive_interval_ms = Some(keep_alive_interval_ms);
        self
    }
    pub fn keep_alive_limit(mut self, keep_alive_limit: u64) -> Self {
        self.props.keep_alive_limit = Some(keep_alive_limit);
        self
    }
    pub fn application_description<AppDescription: Into<Vec<u8>>>(
        mut self,
        application_description: AppDescription,
    ) -> Self {
        self.props.application_description = Some(application_description.into());
        self
    }
    pub fn client_name<ClientName: Into<Vec<u8>>>(mut self, client_name: ClientName) -> Self {
        self.props.client_name = Some(client_name.into());
        self
    }
    pub fn compression_level(mut self, compression_level: u8) -> Self {
        self.props.compression_level = Some(compression_level);
        self
    }
    pub fn generate_rcv_timestamps(mut self, generate_rcv_timestamps: bool) -> Self {
        self.props.generate_rcv_timestamps = Some(generate_rcv_timestamps);
        self
    }
    pub fn generate_send_timestamp(mut self, generate_send_timestamp: bool) -> Self {
        self.props.generate_send_timestamp = Some(generate_send_timestamp);
        self
    }
    pub fn generate_sender_id(mut self, generate_sender_id: bool) -> Self {
        self.props.generate_sender_id = Some(generate_sender_id);
        self
    }
    pub fn generate_sender_sequence_number(
        mut self,
        generate_sender_sequence_number: bool,
    ) -> Self {
        self.props.generate_sender_sequence_number = Some(generate_sender_sequence_number);
        self
    }
    pub fn connect_retries_per_host(mut self, connect_retries_per_host: i64) -> Self {
        self.props.connect_retries_per_host = Some(connect_retries_per_host);
        self
    }
    pub fn connect_retries(mut self, connect_retries: i64) -> Self {
        self.props.connect_retries = Some(connect_retries);
        self
    }
    pub fn reconnect_retries(mut self, reconnect_retries: i64) -> Self {
        self.props.reconnect_retries = Some(reconnect_retries);
        self
    }
    pub fn reconnect_retry_wait_ms(mut self, reconnect_retry_wait_ms: u64) -> Self {
        self.props.reconnect_retry_wait_ms = Some(reconnect_retry_wait_ms);
        self
    }
    pub fn reapply_subscriptions(mut self, reapply_subscriptions: bool) -> Self {
        self.props.reapply_subscriptions = Some(reapply_subscriptions);
        self
    }
    pub fn provision_timeout_ms(mut self, provision_timeout_ms: u64) -> Self {
        self.props.provision_timeout_ms = Some(provision_timeout_ms);
        self
    }
    pub fn calculate_message_expiration(mut self, calculate_message_expiration: bool) -> Self {
        self.props.calculate_message_expiration = Some(calculate_message_expiration);
        self
    }
    pub fn no_local(mut self, no_local: bool) -> Self {
        self.props.no_local = Some(no_local);
        self
    }
    pub fn modifyprop_timeout_ms(mut self, modifyprop_timeout_ms: u64) -> Self {
        self.props.modifyprop_timeout_ms = Some(modifyprop_timeout_ms);
        self
    }
    pub fn ssl_trust_store_dir<ClientName: Into<Vec<u8>>>(mut self, ssl_trust_store_dir: ClientName) -> Self {
        self.props.ssl_trust_store_dir = Some(ssl_trust_store_dir.into());
        self
    }
}

struct CheckedSessionProps {
    host_name: CString,
    vpn_name: CString,
    username: CString,
    password: CString,

    // Note: optional params
    buffer_size_bytes: Option<CString>,
    block_write_timeout_ms: Option<CString>,
    connect_timeout_ms: Option<CString>,
    subconfirm_timeout_ms: Option<CString>,
    ignore_dup_subscription_error: Option<bool>,
    tcp_nodelay: Option<bool>,
    socket_send_buf_size_bytes: Option<CString>,
    socket_rcv_buf_size_bytes: Option<CString>,
    keep_alive_interval_ms: Option<CString>,
    keep_alive_limit: Option<CString>,
    application_description: Option<CString>,
    client_name: Option<CString>,
    compression_level: Option<CString>,
    generate_rcv_timestamps: Option<bool>,
    generate_send_timestamp: Option<bool>,
    generate_sender_id: Option<bool>,
    generate_sender_sequence_number: Option<bool>,
    connect_retries_per_host: Option<CString>,
    connect_retries: Option<CString>,
    reconnect_retries: Option<CString>,
    reconnect_retry_wait_ms: Option<CString>,
    reapply_subscriptions: Option<bool>,
    provision_timeout_ms: Option<CString>,
    calculate_message_expiration: Option<bool>,
    no_local: Option<bool>,
    modifyprop_timeout_ms: Option<CString>,
    ssl_trust_store_dir: Option<CString>,
}

impl CheckedSessionProps {
    fn to_raw(&self) -> Vec<*const i8> {
        let mut props = vec![
            ffi::SOLCLIENT_SESSION_PROP_HOST.as_ptr() as *const i8,
            self.host_name.as_ptr(),
            ffi::SOLCLIENT_SESSION_PROP_VPN_NAME.as_ptr() as *const i8,
            self.vpn_name.as_ptr(),
            ffi::SOLCLIENT_SESSION_PROP_USERNAME.as_ptr() as *const i8,
            self.username.as_ptr(),
            ffi::SOLCLIENT_SESSION_PROP_PASSWORD.as_ptr() as *const i8,
            self.password.as_ptr(),
            ffi::SOLCLIENT_SESSION_PROP_CONNECT_BLOCKING.as_ptr() as *const i8,
            ffi::SOLCLIENT_PROP_ENABLE_VAL.as_ptr() as *const i8,
        ];

        if let Some(x) = &self.buffer_size_bytes {
            props.push(ffi::SOLCLIENT_SESSION_PROP_BUFFER_SIZE.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }

        if let Some(x) = &self.block_write_timeout_ms {
            props.push(ffi::SOLCLIENT_SESSION_PROP_BLOCKING_WRITE_TIMEOUT_MS.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.connect_timeout_ms {
            props.push(ffi::SOLCLIENT_SESSION_PROP_CONNECT_TIMEOUT_MS.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }

        if let Some(x) = &self.subconfirm_timeout_ms {
            props.push(ffi::SOLCLIENT_SESSION_PROP_SUBCONFIRM_TIMEOUT_MS.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.ignore_dup_subscription_error {
            props.push(
                ffi::SOLCLIENT_SESSION_PROP_IGNORE_DUP_SUBSCRIPTION_ERROR.as_ptr() as *const i8,
            );
            props.push(bool_to_ptr(*x));
        }

        if let Some(x) = &self.tcp_nodelay {
            props.push(ffi::SOLCLIENT_SESSION_PROP_TCP_NODELAY.as_ptr() as *const i8);
            props.push(bool_to_ptr(*x));
        }
        if let Some(x) = &self.socket_send_buf_size_bytes {
            props.push(ffi::SOLCLIENT_SESSION_PROP_SOCKET_SEND_BUF_SIZE.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }

        if let Some(x) = &self.socket_rcv_buf_size_bytes {
            props.push(ffi::SOLCLIENT_SESSION_PROP_SOCKET_RCV_BUF_SIZE.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.keep_alive_interval_ms {
            props.push(ffi::SOLCLIENT_SESSION_PROP_KEEP_ALIVE_INT_MS.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.keep_alive_limit {
            props.push(ffi::SOLCLIENT_SESSION_PROP_KEEP_ALIVE_LIMIT.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.application_description {
            props.push(ffi::SOLCLIENT_SESSION_PROP_APPLICATION_DESCRIPTION.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.client_name {
            props.push(ffi::SOLCLIENT_SESSION_PROP_CLIENT_NAME.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }

        if let Some(x) = &self.compression_level {
            props.push(ffi::SOLCLIENT_SESSION_PROP_COMPRESSION_LEVEL.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.generate_rcv_timestamps {
            props.push(ffi::SOLCLIENT_SESSION_PROP_GENERATE_RCV_TIMESTAMPS.as_ptr() as *const i8);
            props.push(bool_to_ptr(*x));
        }
        if let Some(x) = &self.generate_send_timestamp {
            props.push(ffi::SOLCLIENT_SESSION_PROP_GENERATE_SEND_TIMESTAMPS.as_ptr() as *const i8);
            props.push(bool_to_ptr(*x));
        }
        if let Some(x) = &self.generate_sender_id {
            props.push(ffi::SOLCLIENT_SESSION_PROP_GENERATE_SENDER_ID.as_ptr() as *const i8);
            props.push(bool_to_ptr(*x));
        }
        if let Some(x) = &self.generate_sender_sequence_number {
            props.push(ffi::SOLCLIENT_SESSION_PROP_GENERATE_SEQUENCE_NUMBER.as_ptr() as *const i8);
            props.push(bool_to_ptr(*x));
        }
        if let Some(x) = &self.connect_retries_per_host {
            props.push(ffi::SOLCLIENT_SESSION_PROP_CONNECT_RETRIES_PER_HOST.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.connect_retries {
            props.push(ffi::SOLCLIENT_SESSION_PROP_CONNECT_RETRIES.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.reconnect_retries {
            props.push(ffi::SOLCLIENT_SESSION_PROP_RECONNECT_RETRIES.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.reconnect_retry_wait_ms {
            props.push(ffi::SOLCLIENT_SESSION_PROP_RECONNECT_RETRY_WAIT_MS.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.reapply_subscriptions {
            props.push(ffi::SOLCLIENT_SESSION_PROP_REAPPLY_SUBSCRIPTIONS.as_ptr() as *const i8);
            props.push(bool_to_ptr(*x));
        }
        if let Some(x) = &self.provision_timeout_ms {
            props.push(ffi::SOLCLIENT_SESSION_PROP_PROVISION_TIMEOUT_MS.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.calculate_message_expiration {
            props.push(
                ffi::SOLCLIENT_SESSION_PROP_CALCULATE_MESSAGE_EXPIRATION.as_ptr() as *const i8,
            );
            props.push(bool_to_ptr(*x));
        }
        if let Some(x) = &self.no_local {
            props.push(ffi::SOLCLIENT_SESSION_PROP_NO_LOCAL.as_ptr() as *const i8);
            props.push(bool_to_ptr(*x));
        }
        if let Some(x) = &self.modifyprop_timeout_ms {
            props.push(ffi::SOLCLIENT_SESSION_PROP_MODIFYPROP_TIMEOUT_MS.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }
        if let Some(x) = &self.ssl_trust_store_dir {
            props.push(ffi::SOLCLIENT_SESSION_PROP_SSL_TRUST_STORE_DIR.as_ptr() as *const i8);
            props.push(x.as_ptr());
        }

        props.push(ptr::null());

        props
    }
}

impl<Host, Vpn, Username, Password> TryFrom<UncheckedSessionProps<Host, Vpn, Username, Password>>
    for CheckedSessionProps
where
    Host: Into<Vec<u8>>,
    Vpn: Into<Vec<u8>>,
    Username: Into<Vec<u8>>,
    Password: Into<Vec<u8>>,
{
    type Error = SessionBuilderError;

    fn try_from(
        value: UncheckedSessionProps<Host, Vpn, Username, Password>,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        let host_name = match value.host_name {
            Some(x) => CString::new(x)?,
            None => {
                return Err(SessionBuilderError::MissingRequiredArgs(
                    "host_name".to_owned(),
                ));
            }
        };

        let vpn_name = match value.vpn_name {
            Some(x) => CString::new(x)?,
            None => {
                return Err(SessionBuilderError::MissingRequiredArgs(
                    "vpn_name".to_owned(),
                ));
            }
        };

        let username = match value.username {
            Some(x) => CString::new(x)?,
            None => {
                return Err(SessionBuilderError::MissingRequiredArgs(
                    "username".to_owned(),
                ));
            }
        };

        let password = match value.password {
            Some(x) => CString::new(x)?,
            None => {
                return Err(SessionBuilderError::MissingRequiredArgs(
                    "password".to_owned(),
                ));
            }
        };

        let client_name = match value.client_name {
            Some(x) => Some(CString::new(x)?),
            None => None,
        };

        let application_description = match value.application_description {
            Some(x) => Some(CString::new(x)?),
            None => None,
        };

        let buffer_size_bytes = match value.buffer_size_bytes {
            Some(x) if x < 1 => {
                return Err(SessionBuilderError::InvalidRange(
                    "buffer_size_bytes".to_owned(),
                    ">= 1".to_owned(),
                    x.to_string(),
                ));
            }
            Some(b) => Some(CString::new(b.to_string())?),
            None => None,
        };

        let block_write_timeout_ms = match value.block_write_timeout_ms {
            Some(x) if x < 1 => {
                return Err(SessionBuilderError::InvalidRange(
                    "block_write_timeout_ms".to_owned(),
                    ">= 1".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let connect_timeout_ms = match value.connect_timeout_ms {
            Some(x) if x < 1 => {
                return Err(SessionBuilderError::InvalidRange(
                    "connect_timeout_ms".to_owned(),
                    ">= 1".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let subconfirm_timeout_ms = match value.subconfirm_timeout_ms {
            Some(x) if x < 1000 => {
                return Err(SessionBuilderError::InvalidRange(
                    "subconfirm_timeout_ms".to_owned(),
                    ">= 1000".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let socket_send_buf_size_bytes = match value.socket_send_buf_size_bytes {
            Some(x) if x != 0 && x < 1024 => {
                return Err(SessionBuilderError::InvalidRange(
                    "socket_send_buf_size_bytes".to_owned(),
                    "0 or >= 1024".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let socket_rcv_buf_size_bytes = match value.socket_rcv_buf_size_bytes {
            Some(x) if x != 0 && x < 1024 => {
                return Err(SessionBuilderError::InvalidRange(
                    "socket_rcv_buf_size_bytes".to_owned(),
                    "0 or >= 1024".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let keep_alive_interval_ms = match value.keep_alive_interval_ms {
            Some(x) if x != 0 && x < 50 => {
                return Err(SessionBuilderError::InvalidRange(
                    "keep_alive_interval_ms".to_owned(),
                    "0 or >= 50".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let keep_alive_limit = match value.keep_alive_limit {
            Some(x) if x < 3 => {
                return Err(SessionBuilderError::InvalidRange(
                    "keep_alive_limit".to_owned(),
                    ">= 3".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let compression_level = match value.compression_level {
            Some(x) if x > 9 => {
                return Err(SessionBuilderError::InvalidRange(
                    "compression_level".to_owned(),
                    "<= 9".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let connect_retries_per_host = match value.connect_retries_per_host {
            Some(x) if x < -1 => {
                return Err(SessionBuilderError::InvalidRange(
                    "connect_retries_per_host".to_owned(),
                    ">= -1".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let connect_retries = match value.connect_retries {
            Some(x) if x < -1 => {
                return Err(SessionBuilderError::InvalidRange(
                    "connect_retries ".to_owned(),
                    ">= -1".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let reconnect_retries = match value.reconnect_retries {
            Some(x) if x < -1 => {
                return Err(SessionBuilderError::InvalidRange(
                    "reconnect_retries ".to_owned(),
                    ">= -1".to_owned(),
                    x.to_string(),
                ));
            }
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let reconnect_retry_wait_ms = match value.reconnect_retry_wait_ms {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let provision_timeout_ms = match value.provision_timeout_ms {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };
        let modifyprop_timeout_ms = match value.modifyprop_timeout_ms {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };
        let ssl_trust_store_dir = match value.ssl_trust_store_dir {
            Some(x) => Some(CString::new(x)?),
            None => None,
        };
        

        Ok(Self {
            host_name,
            vpn_name,
            username,
            password,
            buffer_size_bytes,
            block_write_timeout_ms,
            connect_timeout_ms,
            subconfirm_timeout_ms,
            ignore_dup_subscription_error: value.ignore_dup_subscription_error,
            tcp_nodelay: value.tcp_nodelay,
            socket_send_buf_size_bytes,
            socket_rcv_buf_size_bytes,
            keep_alive_interval_ms,
            keep_alive_limit,
            application_description,
            client_name,
            compression_level,
            generate_rcv_timestamps: value.generate_rcv_timestamps,
            generate_send_timestamp: value.generate_send_timestamp,
            generate_sender_id: value.generate_sender_id,
            generate_sender_sequence_number: value.generate_sender_sequence_number,
            connect_retries_per_host,
            connect_retries,
            reconnect_retries,
            reconnect_retry_wait_ms,
            reapply_subscriptions: value.reapply_subscriptions,
            provision_timeout_ms,
            calculate_message_expiration: value.calculate_message_expiration,
            no_local: value.no_local,
            modifyprop_timeout_ms,
            ssl_trust_store_dir
        })
    }
}
