pub mod builder;
pub(crate) mod callback;
pub mod event;

use event::FlowEvent;
use solace_rs_sys as ffi;
use std::marker::PhantomData;
use tracing::warn;

use crate::{
    message::{inbound::FlowInboundMessage, InboundMessage},
    session::SessionEvent,
    Session, SolClientReturnCode,
};

pub struct Flow<
    'flow,
    'session,
    SM: FnMut(InboundMessage) + Send + 'session,
    SE: FnMut(SessionEvent) + Send + 'session,
    FM: FnMut(FlowInboundMessage) + Send + 'flow,
    FE: FnMut(FlowEvent) + Send + 'flow,
> {
    pub(crate) lifetime: PhantomData<&'flow ()>,

    // Pointer to flow
    // This pointer must never be allowed to leave the struct
    pub(crate) _flow_ptr: ffi::solClient_opaqueFlow_pt,

    #[allow(dead_code)]
    pub(crate) session: &'flow Session<'session, SM, SE>,

    // These fields are used to store the fn callback. The mutable reference to this fn is passed to the FFI library,
    #[allow(dead_code, clippy::redundant_allocation)]
    _msg_fn_ptr: Option<Box<Box<FM>>>,
    #[allow(dead_code, clippy::redundant_allocation)]
    _event_fn_ptr: Option<Box<Box<FE>>>,
}

unsafe impl<
        SM: FnMut(InboundMessage) + Send,
        SE: FnMut(SessionEvent) + Send,
        FM: FnMut(FlowInboundMessage) + Send,
        FE: FnMut(FlowEvent) + Send,
    > Send for Flow<'_, '_, SM, SE, FM, FE>
{
}

impl<
        SM: FnMut(InboundMessage) + Send,
        SE: FnMut(SessionEvent) + Send,
        FM: FnMut(FlowInboundMessage) + Send,
        FE: FnMut(FlowEvent) + Send,
    > Drop for Flow<'_, '_, SM, SE, FM, FE>
{
    fn drop(&mut self) {
        let session_free_result = unsafe { ffi::solClient_flow_destroy(&mut self._flow_ptr) };
        let rc = SolClientReturnCode::from_raw(session_free_result);

        if !rc.is_ok() {
            warn!("flow was not dropped properly. {rc}");
        }
    }
}
