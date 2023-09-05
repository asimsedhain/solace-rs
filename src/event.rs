use solace_sys as ffi;
use core::fmt;
use enum_primitive::*;
use std::ffi::CStr;

enum_from_primitive! {
    #[derive(Debug, PartialEq, Copy, Clone)]
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

impl fmt::Display for SessionEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw_event = *self as u32 as std::os::raw::c_uint;
        let raw_c_ptr = unsafe { ffi::solClient_session_eventToString(raw_event) };
        let c_str = unsafe { CStr::from_ptr(raw_c_ptr) };
        let message = c_str.to_str().unwrap_or("Unknown Event");
        write!(f, "{}", message)
    }
}
