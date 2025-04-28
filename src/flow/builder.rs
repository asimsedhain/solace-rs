use solace_rs_sys as ffi;
use std::{
    ffi::{CString, NulError},
    marker::PhantomData,
    ptr,
};

use crate::{
    endpoint_props::EndpointProps,
    message::{inbound::FlowInboundMessage, InboundMessage},
    session::SessionEvent,
    util::{bool_to_ptr, get_last_error_info},
    Session, SolClientReturnCode, SolClientSubCode,
};

use super::{
    callback::{
        on_event_trampoline, on_message_trampoline, static_no_op_on_event, static_no_op_on_message,
    },
    event::FlowEvent,
    Flow,
};

#[derive(thiserror::Error, Debug)]
pub enum FlowBuilderError {
    #[error("flow failed to initialize. SolClient return code: {0} subcode: {1}")]
    InitializationFailure(SolClientReturnCode, SolClientSubCode),
    #[error("arg contains interior nul byte")]
    InvalidArgs(#[from] NulError),
}

type Result<T> = std::result::Result<T, FlowBuilderError>;

/// Flow Configuration Properties
/// https://docs.solace.com/API-Developer-Online-Ref-Documentation/c/group__flow_props.html
#[derive(Debug, PartialEq, Eq, Clone, Default)]
struct UncheckedFlowProps {
    bind_timeout_ms: Option<u32>,
    bind_entity_id: Option<FlowBindEntityId<String>>,
    bind_entity_durable: Option<FlowBindEntityDurable>,
    window_size: Option<u32>,
    ack_mode: Option<FlowAckMode>,
    topic: Option<String>,
    max_bind_tries: Option<u32>,
    ack_timer_ms: Option<u32>,
    ack_threshold: Option<u8>,
    start_state: Option<bool>,
    selector: Option<String>,
    no_local: Option<bool>,
    max_unacked_messages: Option<u32>,
    browser: Option<bool>,
    active_flow_ind: Option<bool>,
    replay_start_location: Option<String>,
    max_reconnect_tries: Option<u32>,
    reconnect_retry_interval_ms: Option<u32>,
    required_outcome_failed: Option<bool>,
    required_outcome_rejected: Option<bool>,
    // Note: Blocking only supported for now
    // bind_blocking: Option<bool>,
}

pub struct FlowBuilder<'builder, 'session, SM, SE, OnMessage, OnEvent>
where
    SM: FnMut(InboundMessage) + Send + 'session,
    SE: FnMut(SessionEvent) + Send + 'session,
{
    session: &'builder Session<'session, SM, SE>,
    props: UncheckedFlowProps,

    // callbacks
    on_message: Option<OnMessage>,
    on_event: Option<OnEvent>,
}

impl<'builder, 'session, SM, SE, OnMessage, OnEvent>
    FlowBuilder<'builder, 'session, SM, SE, OnMessage, OnEvent>
where
    SM: FnMut(InboundMessage) + Send + 'session,
    SE: FnMut(SessionEvent) + Send + 'session,
{
    pub(crate) fn new(session: &'builder Session<'session, SM, SE>) -> Self {
        Self {
            session,
            props: UncheckedFlowProps::default(),
            on_message: None,
            on_event: None,
        }
    }
}

impl<'builder, 'flow, 'session, SM, SE, FM, FE> FlowBuilder<'builder, 'session, SM, SE, FM, FE>
where
    SM: FnMut(InboundMessage) + Send + 'session,
    SE: FnMut(SessionEvent) + Send + 'session,
    FM: FnMut(FlowInboundMessage) + Send + 'flow,
    FE: FnMut(FlowEvent) + Send + 'flow,
    'builder: 'flow,
{
    pub fn build(self) -> Result<Flow<'flow, 'session, SM, SE, FM, FE>> {
        let checked_props = CheckedFlowProps::try_from(self.props)?;

        let mut flow_ptr: ffi::solClient_opaqueFlow_pt = ptr::null_mut();

        let (static_on_message_callback, user_on_message, msg_func_ptr) = match self.on_message {
            Some(f) => {
                let tramp = on_message_trampoline(&f);
                let mut func = Box::new(Box::new(f));
                (tramp, func.as_mut() as *const _ as *mut _, Some(func))
            }
            _ => (
                Some(static_no_op_on_message as unsafe extern "C" fn(_, _, _) -> u32),
                std::ptr::null_mut(),
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
                std::ptr::null_mut(),
                None,
            ),
        };

        let mut flow_func_info: ffi::solClient_flow_createFuncInfo_t =
            ffi::solClient_flow_createFuncInfo_t {
                rxInfo: ffi::solClient_flow_createRxCallbackFuncInfo {
                    callback_p: ptr::null_mut(),
                    user_p: ptr::null_mut(),
                },
                eventInfo: ffi::solClient_flow_createEventCallbackFuncInfo {
                    callback_p: static_on_event_callback,
                    user_p: user_on_event,
                },
                rxMsgInfo: ffi::solClient_flow_createRxMsgCallbackFuncInfo {
                    callback_p: static_on_message_callback,
                    user_p: user_on_message,
                },
            };

        let flow_create_raw_rc = unsafe {
            ffi::solClient_session_createFlow(
                checked_props.to_raw().as_mut_ptr(),
                self.session._session_ptr,
                &mut flow_ptr,
                &mut flow_func_info,
                std::mem::size_of::<ffi::solClient_session_createFuncInfo_t>(),
            )
        };

        let rc = SolClientReturnCode::from_raw(flow_create_raw_rc);
        if rc.is_ok() {
            Ok(Flow {
                lifetime: PhantomData,
                _flow_ptr: flow_ptr,
                session: &self.session,
                _msg_fn_ptr: msg_func_ptr,
                _event_fn_ptr: event_func_ptr,
            })
        } else {
            let subcode = get_last_error_info();
            Err(FlowBuilderError::InitializationFailure(rc, subcode))
        }
    }

    /// Sets the timeout (in milliseconds) used when creating a Flow in blocking mode.
    ///
    /// The valid range is > 0. Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_BIND_TIMEOUT_MS]
    pub fn bind_timeout_ms(mut self, timeout: u32) -> Self {
        self.props.bind_timeout_ms = Some(timeout);
        self
    }

    /// Sets the type of object to which this Flow is bound.
    ///
    /// The valid values are SOLCLIENT_FLOW_PROP_BIND_ENTITY_SUB, SOLCLIENT_FLOW_PROP_BIND_ENTITY_QUEUE, and SOLCLIENT_FLOW_PROP_BIND_ENTITY_TE.
    /// Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_BIND_ENTITY_ID]
    pub fn bind_entity_id(mut self, entity_id: FlowBindEntityId<String>) -> Self {
        self.props.bind_entity_id = Some(entity_id);
        self
    }

    /// Sets the durability of the object to which this Flow is bound.
    ///
    /// Default: [ffi::SOLCLIENT_PROP_ENABLE_VAL], which means the endpoint is durable. When set to [ffi::SOLCLIENT_PROP_DISABLE_VAL], a temporary endpoint is created.
    pub fn bind_entity_durable(mut self, durable: FlowBindEntityDurable) -> Self {
        self.props.bind_entity_durable = Some(durable);
        self
    }

    /// Sets the Guaranteed message window size for the Flow.
    ///
    /// This sets the maximum number of messages that can be in transit (that is, the messages are sent from the broker but are not yet delivered to the application).
    /// The valid range is 1..255. Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_WINDOWSIZE]
    pub fn window_size(mut self, size: u32) -> Self {
        self.props.window_size = Some(size);
        self
    }

    /// Sets the acknowledgment mode for the Flow.
    ///
    /// Possible values are SOLCLIENT_FLOW_PROP_ACKMODE_AUTO and SOLCLIENT_FLOW_PROP_ACKMODE_CLIENT. Default: SOLCLIENT_FLOW_PROP_ACKMODE_AUTO
    pub fn ack_mode(mut self, mode: FlowAckMode) -> Self {
        self.props.ack_mode = Some(mode);
        self
    }

    /// Sets the topic to which the Flow is bound.
    ///
    /// When binding to a Topic endpoint, the Topic may be set in the bind. This parameter is ignored for Queue or subscriber binding.
    /// The maximum length (not including NULL terminator) is SOLCLIENT_BUFINFO_MAX_TOPIC_SIZE. Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_TOPIC]
    pub fn topic(mut self, topic: String) -> Self {
        self.props.topic = Some(topic);
        self
    }

    /// Sets the maximum number of attempts to bind the Flow.
    ///
    /// The valid range is >= 1. Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_MAX_BIND_TRIES]
    pub fn max_bind_tries(mut self, tries: u32) -> Self {
        self.props.max_bind_tries = Some(tries);
        self
    }

    /// Sets the timer (in milliseconds) for sending acknowledgments.
    ///
    /// The valid range is 20..1500. Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_ACK_TIMER_MS]
    pub fn ack_timer_ms(mut self, timer: u32) -> Self {
        self.props.ack_timer_ms = Some(timer);
        self
    }

    /// Sets the threshold for sending an acknowledgment, configured as a percentage.
    ///
    /// The valid range is 1..75. Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_ACK_THRESHOLD]
    pub fn ack_threshold(mut self, threshold: u8) -> Self {
        self.props.ack_threshold = Some(threshold);
        self
    }

    /// Controls whether the Flow should be created in a start or stop state with respect to receiving messages.
    ///
    /// Flow start/stop state can be changed later through solClient_flow_start() or solClient_flow_stop(). Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_START_STATE]
    pub fn start_state(mut self, state: bool) -> Self {
        self.props.start_state = Some(state);
        self
    }

    /// Sets the selector for filtering messages on the Flow.
    ///
    /// A Java Message System (JMS) defined selector.
    pub fn selector(mut self, selector: String) -> Self {
        self.props.selector = Some(selector);
        self
    }

    /// Controls whether the Flow should exclude messages published by the same client.
    ///
    /// When a Flow has the No Local property enabled, messages published on the Session cannot appear in a Flow created in the same Session, even if the endpoint contains a subscription that matches the published message.
    pub fn no_local(mut self, no_local: bool) -> Self {
        self.props.no_local = Some(no_local);
        self
    }

    /// Sets the maximum number of unacknowledged messages allowed on the Flow.
    ///
    /// This property may only be set when the Flow property SOLCLIENT_FLOW_PROP_ACKMODE is set to SOLCLIENT_FLOW_PROP_ACKMODE_CLIENT.
    /// Valid values are -1 and >0. Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_MAX_UNACKED_MESSAGES]
    pub fn max_unacked_messages(mut self, max: u32) -> Self {
        self.props.max_unacked_messages = Some(max);
        self
    }

    /// Indicates whether the Flow operates in browser mode.
    ///
    /// A browser flow allows client applications to look at messages spooled on Endpoints without removing them. Messages are browsed from oldest to newest.
    /// Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_BROWSER]
    pub fn browser(mut self, browser: bool) -> Self {
        self.props.browser = Some(browser);
        self
    }

    /// Enables Active Flow Indication, which sends events when the Flow becomes active or inactive.
    ///
    /// If the underlying session capabilities indicate that the broker does not support active flow indications, then solClient_session_createFlow() will fail immediately.
    /// Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_ACTIVE_FLOW_IND]
    pub fn active_flow_ind(mut self, active: bool) -> Self {
        self.props.active_flow_ind = Some(active);
        self
    }

    /// Sets the starting location for replaying messages on the Flow.
    ///
    /// The replay start location may be SOLCLIENT_FLOW_PROP_REPLAY_START_LOCATION_BEGINNING to indicate that all messages available should be replayed.
    /// Examples: ex/messageReplay.c, and ex/simpleFlowToQueue.c.
    pub fn replay_start_location(mut self, location: String) -> Self {
        self.props.replay_start_location = Some(location);
        self
    }

    /// Sets the maximum number of attempts to reconnect the Flow.
    ///
    /// If this property is -1, it will retry forever. Otherwise it tries the configured maximum number of times. Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_MAX_RECONNECT_TRIES]
    pub fn max_reconnect_tries(mut self, tries: u32) -> Self {
        self.props.max_reconnect_tries = Some(tries);
        self
    }

    /// Sets the interval (in milliseconds) between reconnect attempts for the Flow.
    ///
    /// Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_RECONNECT_RETRY_INTERVAL_MS]
    pub fn reconnect_retry_interval_ms(mut self, interval: u32) -> Self {
        self.props.reconnect_retry_interval_ms = Some(interval);
        self
    }

    /// Indicates whether the Flow requires an outcome of "failed" for messages.
    ///
    /// Ignored on transacted sessions. Requires [ffi::SOLCLIENT_SESSION_CAPABILITY_AD_APP_ACK_FAILED]. Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_REQUIRED_OUTCOME_FAILED]
    pub fn required_outcome_failed(mut self, failed: bool) -> Self {
        self.props.required_outcome_failed = Some(failed);
        self
    }

    /// Indicates whether the Flow requires an outcome of "rejected" for messages.
    ///
    /// Ignored on transacted sessions. Requires [ffi::SOLCLIENT_SESSION_CAPABILITY_AD_APP_ACK_FAILED]. Default: [ffi::SOLCLIENT_FLOW_PROP_DEFAULT_REQUIRED_OUTCOME_REJECTED]
    pub fn required_outcome_rejected(mut self, rejected: bool) -> Self {
        self.props.required_outcome_rejected = Some(rejected);
        self
    }

    /// Sets the callback for handling inbound messages on the Flow.
    pub fn on_message(mut self, on_message: FM) -> Self {
        self.on_message = Some(on_message);
        self
    }

    /// Sets the callback for handling events on the Flow.
    pub fn on_event(mut self, on_event: FE) -> Self {
        self.on_event = Some(on_event);
        self
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct CheckedFlowProps {
    bind_timeout_ms: Option<CString>,
    bind_entity_id: Option<FlowBindEntityId<CString>>,
    bind_entity_durable: Option<FlowBindEntityDurable>,
    window_size: Option<CString>,
    ack_mode: Option<FlowAckMode>,
    topic: Option<CString>,
    max_bind_tries: Option<CString>,
    ack_timer_ms: Option<CString>,
    ack_threshold: Option<CString>,
    start_state: Option<bool>,
    selector: Option<CString>,
    no_local: Option<bool>,
    max_unacked_messages: Option<CString>,
    browser: Option<bool>,
    active_flow_ind: Option<bool>,
    replay_start_location: Option<CString>,
    max_reconnect_tries: Option<CString>,
    reconnect_retry_interval_ms: Option<CString>,
    required_outcome_failed: Option<bool>,
    required_outcome_rejected: Option<bool>,
}

impl CheckedFlowProps {
    fn to_raw(&self) -> Vec<*const i8> {
        let mut props = vec![];

        if let Some(bind_timeout_ms) = &self.bind_timeout_ms {
            props.push(ffi::SOLCLIENT_FLOW_PROP_BIND_TIMEOUT_MS.as_ptr() as *const i8);
            props.push(bind_timeout_ms.as_ptr());
        }

        if let Some(bind_entity_id) = &self.bind_entity_id {
            props.push(ffi::SOLCLIENT_FLOW_PROP_BIND_ENTITY_ID.as_ptr() as *const i8);
            match bind_entity_id {
                FlowBindEntityId::Sub => {
                    props.push(ffi::SOLCLIENT_FLOW_PROP_BIND_ENTITY_SUB.as_ptr() as *const i8);
                }
                FlowBindEntityId::Queue { queue_name } => {
                    props.push(ffi::SOLCLIENT_FLOW_PROP_BIND_ENTITY_QUEUE.as_ptr() as *const i8);
                    props.push(ffi::SOLCLIENT_FLOW_PROP_BIND_NAME.as_ptr() as *const i8);
                    props.push(queue_name.as_ptr());
                }
                FlowBindEntityId::Te {
                    topic_endpoint_name,
                } => {
                    props.push(ffi::SOLCLIENT_FLOW_PROP_BIND_ENTITY_TE.as_ptr() as *const i8);
                    props.push(ffi::SOLCLIENT_FLOW_PROP_BIND_NAME.as_ptr() as *const i8);
                    props.push(topic_endpoint_name.as_ptr());
                }
            }
        }

        if let Some(bind_entity_durable) = &self.bind_entity_durable {
            props.push(ffi::SOLCLIENT_FLOW_PROP_BIND_ENTITY_DURABLE.as_ptr() as *const i8);

            match bind_entity_durable {
                FlowBindEntityDurable::Durable => {
                    props.push(bool_to_ptr(true));
                }
                FlowBindEntityDurable::NonDurable { endpoint_props } => {
                    props.push(bool_to_ptr(false));
                    let mut endpoint_props = endpoint_props.to_raw();
                    // Remove null character on the end
                    endpoint_props
                        .pop()
                        .expect("null character should be removed");
                    props.extend_from_slice(&endpoint_props);
                }
            }
        }

        if let Some(window_size) = &self.window_size {
            props.push(ffi::SOLCLIENT_FLOW_PROP_WINDOWSIZE.as_ptr() as *const i8);
            props.push(window_size.as_ptr());
        }

        if let Some(ack_mode) = &self.ack_mode {
            props.push(ffi::SOLCLIENT_FLOW_PROP_ACKMODE.as_ptr() as *const i8);
            match ack_mode {
                FlowAckMode::Auto => {
                    props.push(ffi::SOLCLIENT_FLOW_PROP_ACKMODE_AUTO.as_ptr() as *const i8);
                }
                FlowAckMode::Client => {
                    props.push(ffi::SOLCLIENT_FLOW_PROP_ACKMODE_CLIENT.as_ptr() as *const i8);
                }
            }
        }

        if let Some(topic) = &self.topic {
            props.push(ffi::SOLCLIENT_FLOW_PROP_TOPIC.as_ptr() as *const i8);
            props.push(topic.as_ptr());
        }

        if let Some(max_bind_tries) = &self.max_bind_tries {
            props.push(ffi::SOLCLIENT_FLOW_PROP_MAX_BIND_TRIES.as_ptr() as *const i8);
            props.push(max_bind_tries.as_ptr());
        }

        if let Some(ack_timer_ms) = &self.ack_timer_ms {
            props.push(ffi::SOLCLIENT_FLOW_PROP_ACK_TIMER_MS.as_ptr() as *const i8);
            props.push(ack_timer_ms.as_ptr());
        }

        if let Some(ack_threshold) = &self.ack_threshold {
            props.push(ffi::SOLCLIENT_FLOW_PROP_ACK_THRESHOLD.as_ptr() as *const i8);
            props.push(ack_threshold.as_ptr());
        }

        if let Some(start_state) = &self.start_state {
            props.push(ffi::SOLCLIENT_FLOW_PROP_START_STATE.as_ptr() as *const i8);
            props.push(bool_to_ptr(*start_state));
        }

        if let Some(selector) = &self.selector {
            props.push(ffi::SOLCLIENT_FLOW_PROP_SELECTOR.as_ptr() as *const i8);
            props.push(selector.as_ptr());
        }

        if let Some(no_local) = &self.no_local {
            props.push(ffi::SOLCLIENT_FLOW_PROP_NO_LOCAL.as_ptr() as *const i8);
            props.push(bool_to_ptr(*no_local));
        }

        if let Some(max_unacked_messages) = &self.max_unacked_messages {
            props.push(ffi::SOLCLIENT_FLOW_PROP_MAX_UNACKED_MESSAGES.as_ptr() as *const i8);
            props.push(max_unacked_messages.as_ptr());
        }

        if let Some(browser) = &self.browser {
            props.push(ffi::SOLCLIENT_FLOW_PROP_BROWSER.as_ptr() as *const i8);
            props.push(bool_to_ptr(*browser));
        }

        if let Some(active_flow_ind) = &self.active_flow_ind {
            props.push(ffi::SOLCLIENT_FLOW_PROP_ACTIVE_FLOW_IND.as_ptr() as *const i8);
            props.push(bool_to_ptr(*active_flow_ind));
        }

        if let Some(replay_start_location) = &self.replay_start_location {
            props.push(ffi::SOLCLIENT_FLOW_PROP_REPLAY_START_LOCATION.as_ptr() as *const i8);
            props.push(replay_start_location.as_ptr());
        }

        if let Some(max_reconnect_tries) = &self.max_reconnect_tries {
            props.push(ffi::SOLCLIENT_FLOW_PROP_MAX_RECONNECT_TRIES.as_ptr() as *const i8);
            props.push(max_reconnect_tries.as_ptr());
        }

        if let Some(reconnect_retry_interval_ms) = &self.reconnect_retry_interval_ms {
            props.push(ffi::SOLCLIENT_FLOW_PROP_RECONNECT_RETRY_INTERVAL_MS.as_ptr() as *const i8);
            props.push(reconnect_retry_interval_ms.as_ptr());
        }

        if let Some(required_outcome_failed) = &self.required_outcome_failed {
            props.push(ffi::SOLCLIENT_FLOW_PROP_REQUIRED_OUTCOME_FAILED.as_ptr() as *const i8);
            props.push(bool_to_ptr(*required_outcome_failed));
        }

        if let Some(required_outcome_rejected) = &self.required_outcome_rejected {
            props.push(ffi::SOLCLIENT_FLOW_PROP_REQUIRED_OUTCOME_REJECTED.as_ptr() as *const i8);
            props.push(bool_to_ptr(*required_outcome_rejected));
        }

        props.push(std::ptr::null());

        props
    }
}

impl TryFrom<UncheckedFlowProps> for CheckedFlowProps {
    type Error = FlowBuilderError;

    fn try_from(props: UncheckedFlowProps) -> Result<Self> {
        let bind_timeout_ms = match props.bind_timeout_ms {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let bind_entity_id = match props.bind_entity_id {
            Some(x) => Some(x.try_into()?),
            None => None,
        };

        let bind_entity_durable = props.bind_entity_durable;

        let window_size = match props.window_size {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let ack_mode = props.ack_mode;

        let topic = match props.topic {
            Some(x) => Some(CString::new(x)?),
            None => None,
        };

        let max_bind_tries = match props.max_bind_tries {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let ack_timer_ms = match props.ack_timer_ms {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let ack_threshold = match props.ack_threshold {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let start_state = props.start_state;

        let selector = match props.selector {
            Some(x) => Some(CString::new(x)?),
            None => None,
        };

        let no_local = props.no_local;

        let max_unacked_messages = match props.max_unacked_messages {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let browser = props.browser;

        let active_flow_ind = props.active_flow_ind;

        let replay_start_location = match props.replay_start_location {
            Some(x) => Some(CString::new(x)?),
            None => None,
        };

        let max_reconnect_tries = match props.max_reconnect_tries {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let reconnect_retry_interval_ms = match props.reconnect_retry_interval_ms {
            Some(x) => Some(CString::new(x.to_string())?),
            None => None,
        };

        let required_outcome_failed = props.required_outcome_failed;

        let required_outcome_rejected = props.required_outcome_rejected;

        Ok(Self {
            bind_timeout_ms,
            bind_entity_id,
            bind_entity_durable,
            window_size,
            ack_mode,
            topic,
            max_bind_tries,
            ack_timer_ms,
            ack_threshold,
            start_state,
            selector,
            no_local,
            max_unacked_messages,
            browser,
            active_flow_ind,
            replay_start_location,
            max_reconnect_tries,
            reconnect_retry_interval_ms,
            required_outcome_failed,
            required_outcome_rejected,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FlowBindEntityId<T> {
    Sub,
    Queue { queue_name: T },
    Te { topic_endpoint_name: T },
}
impl TryFrom<FlowBindEntityId<String>> for FlowBindEntityId<CString> {
    type Error = FlowBuilderError;

    fn try_from(value: FlowBindEntityId<String>) -> Result<Self> {
        match value {
            FlowBindEntityId::Sub => Ok(FlowBindEntityId::Sub),
            FlowBindEntityId::Queue { queue_name } => Ok(FlowBindEntityId::Queue {
                queue_name: CString::new(queue_name)?,
            }),
            FlowBindEntityId::Te {
                topic_endpoint_name,
            } => Ok(FlowBindEntityId::Te {
                topic_endpoint_name: CString::new(topic_endpoint_name)?,
            }),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FlowAckMode {
    Auto,
    Client,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FlowBindEntityDurable {
    Durable,
    NonDurable { endpoint_props: EndpointProps },
}
