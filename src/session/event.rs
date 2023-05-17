use crate::solace::ffi;
use enum_primitive::*;

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

pub struct Event{}
