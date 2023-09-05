use num_traits::FromPrimitive;

use crate::event::SessionEvent;
use crate::message::InboundMessage;
use solace_rs_sys as ffi;
use std::{mem, ptr};

pub(super) fn on_message_trampoline<F>(_closure: &F) -> ffi::solClient_session_rxMsgCallbackFunc_t
where
    F: FnMut(InboundMessage) + Send + 'static,
{
    Some(static_on_message::<F>)
}

pub(super) fn on_event_trampoline<F>(_closure: &F) -> ffi::solClient_session_eventCallbackFunc_t
where
    F: FnMut(SessionEvent) + Send + 'static,
{
    Some(static_on_event::<F>)
}

extern "C" fn static_on_message<F>(
    _opaque_session_p: ffi::solClient_opaqueSession_pt, // non-null
    msg_p: ffi::solClient_opaqueMsg_pt,                 // non-null
    raw_user_closure: *mut ::std::os::raw::c_void,      // can be null
) -> ffi::solClient_rxMsgCallback_returnCode_t
where
    // not completely sure if this is supposed to be FnMut or FnOnce
    // threading takes in FnOnce - that is why I suspect it might be FnOnce.
    // But not enough knowledge to make sure it is FnOnce.
    F: FnMut(InboundMessage) + Send + 'static,
{
    // this function is glue code to allow users to pass in closures
    // we duplicate the message pointer (which does not copy over the binary data)
    // also this function will only be called from the context thread, so it should be thread safe
    // as well

    let non_null_raw_user_closure = std::ptr::NonNull::new(raw_user_closure);

    let Some(raw_user_closure) =  non_null_raw_user_closure else{
        return ffi::solClient_rxMsgCallback_returnCode_SOLCLIENT_CALLBACK_OK;
    };

    let mut dup_msg_ptr = ptr::null_mut();
    unsafe { ffi::solClient_msg_dup(msg_p, &mut dup_msg_ptr) };

    let message = InboundMessage::from(dup_msg_ptr);
    let user_closure: &mut Box<F> = unsafe { mem::transmute(raw_user_closure) };
    user_closure(message);

    ffi::solClient_rxMsgCallback_returnCode_SOLCLIENT_CALLBACK_OK
}

extern "C" fn static_on_event<F>(
    _opaque_session_p: ffi::solClient_opaqueSession_pt, // non-null
    event_info_p: ffi::solClient_session_eventCallbackInfo_pt, //non-null
    raw_user_closure: *mut ::std::os::raw::c_void,      // can be null
) where
    F: FnMut(SessionEvent),
{
    let non_null_raw_user_closure = std::ptr::NonNull::new(raw_user_closure);

    let Some(raw_user_closure) =  non_null_raw_user_closure else{
        return
    };
    let raw_event = unsafe { (*event_info_p).sessionEvent };

    let Some(event) = SessionEvent::from_u32(raw_event) else{
        // TODO
        // log a warning
        return
    };

    let user_closure: &mut Box<F> = unsafe { mem::transmute(raw_user_closure) };

    user_closure(event);
}
