use std::fmt;
pub mod context;
mod message;
pub mod session;
mod solace;

use enum_primitive::*;
use solace::ffi;

#[derive(Debug, Clone)]
pub struct SolaceError;

type Result<T> = std::result::Result<T, SolaceError>;

impl fmt::Display for SolaceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Solace Error Occured!")
    }
}

enum_from_primitive! {
    #[derive(Debug, PartialEq)]
    #[repr(u32)]
    pub enum SolaceLogLevel {
        Critical = ffi::solClient_log_level_SOLCLIENT_LOG_CRITICAL,
        Error = ffi::solClient_log_level_SOLCLIENT_LOG_ERROR,
        Warning = ffi::solClient_log_level_SOLCLIENT_LOG_WARNING,
        Notice = ffi::solClient_log_level_SOLCLIENT_LOG_NOTICE,
        Info = ffi::solClient_log_level_SOLCLIENT_LOG_INFO,
        Debug = ffi::solClient_log_level_SOLCLIENT_LOG_DEBUG,
    }
}

enum_from_primitive! {
    #[derive(Debug, PartialEq)]
    #[repr(i32)]
    pub enum SolClientReturnCode {
        Ok=ffi::solClient_returnCode_SOLCLIENT_OK,
        WouldBlock=ffi::solClient_returnCode_SOLCLIENT_WOULD_BLOCK,
        InProgress=ffi::solClient_returnCode_SOLCLIENT_IN_PROGRESS,
        NotReady=ffi::solClient_returnCode_SOLCLIENT_NOT_READY,
        EndOfStream=ffi::solClient_returnCode_SOLCLIENT_EOS,
        NotFound=ffi::solClient_returnCode_SOLCLIENT_NOT_FOUND,
        NoEvent=ffi::solClient_returnCode_SOLCLIENT_NOEVENT,
        Incomplete=ffi::solClient_returnCode_SOLCLIENT_INCOMPLETE,
        Rollback=ffi::solClient_returnCode_SOLCLIENT_ROLLBACK,
        Fail=ffi::solClient_returnCode_SOLCLIENT_FAIL,
    }
}

enum_from_primitive! {
    #[derive(Debug, PartialEq)]
    #[repr(u32)]
    pub enum SessionEvent {
        UpNotice=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_UP_NOTICE,
        DownError=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_DOWN_ERROR,
        ConnectFailedError=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_CONNECT_FAILED_ERROR,
        RejectedMsgError=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_REJECTED_MSG_ERROR,
        SubscriptionError=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_SUBSCRIPTION_ERROR,
        RxMsgTooBigError=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_RX_MSG_TOO_BIG_ERROR,
        Acknowledgement=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_ACKNOWLEDGEMENT,
        AssuredPublishingUp=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_ASSURED_PUBLISHING_UP,
        AssuredDeliveryDown=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_ASSURED_DELIVERY_DOWN,
        TeUnsubscribeError=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_TE_UNSUBSCRIBE_ERROR,
        TeUnsubscribeOk=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_TE_UNSUBSCRIBE_OK,
        CanSend=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_CAN_SEND,
        ReconnectingNotice=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_RECONNECTING_NOTICE,
        ReconnectedNotice=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_RECONNECTED_NOTICE,
        ProvisionError=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_PROVISION_ERROR,
        ProvisionOk=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_PROVISION_OK,
        SubscriptionOk=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_SUBSCRIPTION_OK,
        VirtualRouterNameChanged=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_VIRTUAL_ROUTER_NAME_CHANGED,
        ModifypropOk=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_MODIFYPROP_OK,
        ModifypropFail=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_MODIFYPROP_FAIL,
        RepublishUnackedMessages=ffi::solClient_session_event_SOLCLIENT_SESSION_EVENT_REPUBLISH_UNACKED_MESSAGES,

    }
}
