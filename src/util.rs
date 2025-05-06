use ffi::solClient_getLastErrorInfo;
use num_traits::FromPrimitive;

use crate::message::InboundMessage;
use crate::session::SessionEvent;
use crate::SolClientSubCode;
use solace_rs_sys as ffi;
use std::mem;

pub(crate) fn on_message_trampoline<'s, F>(
    _closure: &'s F,
) -> ffi::solClient_session_rxMsgCallbackFunc_t
where
    F: FnMut(InboundMessage) + Send + 's,
{
    Some(static_on_message::<F>)
}

pub(crate) fn on_event_trampoline<'s, F>(
    _closure: &'s F,
) -> ffi::solClient_session_eventCallbackFunc_t
where
    F: FnMut(SessionEvent) + Send + 's,
{
    Some(static_on_event::<F>)
}

pub(crate) extern "C" fn static_no_op_on_message(
    _opaque_session_p: ffi::solClient_opaqueSession_pt,
    _msg_p: ffi::solClient_opaqueMsg_pt,
    _raw_user_closure: *mut ::std::os::raw::c_void,
) -> ffi::solClient_rxMsgCallback_returnCode_t {
    ffi::solClient_rxMsgCallback_returnCode_SOLCLIENT_CALLBACK_OK
}

extern "C" fn static_on_message<'s, F>(
    _opaque_session_p: ffi::solClient_opaqueSession_pt, // non-null
    msg_p: ffi::solClient_opaqueMsg_pt,                 // non-null
    raw_user_closure: *mut ::std::os::raw::c_void,      // can be null
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
    _opaque_session_p: ffi::solClient_opaqueSession_pt, // non-null
    _event_info_p: ffi::solClient_session_eventCallbackInfo_pt, //non-null
    _raw_user_closure: *mut ::std::os::raw::c_void,     // can be null
) {
}

extern "C" fn static_on_event<'s, F>(
    _opaque_session_p: ffi::solClient_opaqueSession_pt, // non-null
    event_info_p: ffi::solClient_session_eventCallbackInfo_pt, //non-null
    raw_user_closure: *mut ::std::os::raw::c_void,      // can be null
) where
    F: FnMut(SessionEvent) + Send + 's,
{
    let non_null_raw_user_closure = std::ptr::NonNull::new(raw_user_closure);

    let Some(raw_user_closure) = non_null_raw_user_closure else {
        return;
    };
    let raw_event = unsafe { (*event_info_p).sessionEvent };

    let Some(event) = SessionEvent::from_u32(raw_event) else {
        // TODO
        // log a warning
        return;
    };

    let user_closure: &mut Box<F> = unsafe { mem::transmute(raw_user_closure) };

    user_closure(event);
}

pub(crate) fn get_last_error_info() -> SolClientSubCode {
    // Safety: erno is never null
    unsafe {
        let erno = solClient_getLastErrorInfo();
        let subcode = (*erno).subCode;
        let repr_raw: [u8; 256] = mem::transmute((*erno).errorStr);
        let repr = std::ffi::CStr::from_bytes_until_nul(&repr_raw).unwrap();
        SolClientSubCode {
            subcode,
            error_string: repr.to_string_lossy().to_string(),
        }
    }
}

pub(crate) fn bool_to_ptr(b: bool) -> *const i8 {
    if b {
        ffi::SOLCLIENT_PROP_ENABLE_VAL.as_ptr() as *const i8
    } else {
        ffi::SOLCLIENT_PROP_DISABLE_VAL.as_ptr() as *const i8
    }
}
