use num_traits::FromPrimitive;
use solace_rs_sys as ffi;
use std::mem;

use crate::message::InboundMessage;

use super::event::FlowEvent;

pub(crate) fn on_message_trampoline<'s, F>(
    _closure: &'s F,
) -> ffi::solClient_flow_rxMsgCallbackFunc_t
where
    F: FnMut(InboundMessage) + Send + 's,
{
    Some(static_on_message::<F>)
}

pub(crate) fn on_event_trampoline<'s, F>(_closure: &'s F) -> ffi::solClient_flow_eventCallbackFunc_t
where
    F: FnMut(FlowEvent) + Send + 's,
{
    Some(static_on_event::<F>)
}

pub(crate) extern "C" fn static_no_op_on_message(
    _opaque_flow_p: ffi::solClient_opaqueFlow_pt,
    _msg_p: ffi::solClient_opaqueMsg_pt,
    _raw_user_closure: *mut ::std::os::raw::c_void,
) -> ffi::solClient_rxMsgCallback_returnCode_t {
    ffi::solClient_rxMsgCallback_returnCode_SOLCLIENT_CALLBACK_OK
}

extern "C" fn static_on_message<'s, F>(
    _opaque_flow_p: ffi::solClient_opaqueFlow_pt, // non-null
    msg_p: ffi::solClient_opaqueMsg_pt,           // non-null
    raw_user_closure: *mut ::std::os::raw::c_void, // can be null
) -> ffi::solClient_rxMsgCallback_returnCode_t
where
    // not completely sure if this is supposed to be FnMut or FnOnce
    // threading takes in FnOnce - that is why I suspect it might be FnOnce.
    // But not enough knowledge to make sure it is FnOnce.
    F: FnMut(InboundMessage) + Send + 's,
{
    // this function is glue code to allow users to pass in closures
    // we duplicate the message pointer (which does not copy over the binary data)
    // also this function will only be called from the context thread, so it should be thread safe
    // as well

    let non_null_raw_user_closure = std::ptr::NonNull::new(raw_user_closure);

    let Some(raw_user_closure) = non_null_raw_user_closure else {
        return ffi::solClient_rxMsgCallback_returnCode_SOLCLIENT_CALLBACK_OK;
    };

    let message = InboundMessage::from(msg_p);
    let user_closure: &mut Box<F> = unsafe { mem::transmute(raw_user_closure) };
    user_closure(message);

    ffi::solClient_rxMsgCallback_returnCode_SOLCLIENT_CALLBACK_TAKE_MSG
}

pub(crate) extern "C" fn static_no_op_on_event(
    _opaque_flow_p: ffi::solClient_opaqueFlow_pt, // non-null
    _event_info_p: ffi::solClient_flow_eventCallbackInfo_pt, //non-null
    _raw_user_closure: *mut ::std::os::raw::c_void, // can be null
) {
}

extern "C" fn static_on_event<'s, F>(
    _opaque_flow_p: ffi::solClient_opaqueFlow_pt, // non-null
    event_info_p: ffi::solClient_flow_eventCallbackInfo_pt, //non-null
    raw_user_closure: *mut ::std::os::raw::c_void, // can be null
) where
    F: FnMut(FlowEvent) + Send + 's,
{
    let non_null_raw_user_closure = std::ptr::NonNull::new(raw_user_closure);

    let Some(raw_user_closure) = non_null_raw_user_closure else {
        return;
    };
    let raw_event = unsafe { (*event_info_p).flowEvent };

    let Some(event) = FlowEvent::from_u32(raw_event) else {
        // TODO
        // log a warning
        return;
    };

    let user_closure: &mut Box<F> = unsafe { mem::transmute(raw_user_closure) };

    user_closure(event);
}
