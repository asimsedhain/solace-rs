use crate::solace::ffi;
use crate::{Result, SolaceLogLevel};
use std::mem;
use std::ptr;

pub struct SolContext {
    // This pointer must never be allowed to leave the struct
    pub(crate) _ctx: ffi::solClient_opaqueContext_pt,
}

// Solace initializes global variables
// as such it is not safe to have multiple solaces libraries
// in the same project
impl SolContext {
    pub fn new(log_level: SolaceLogLevel) -> Result<Self> {
        let solace_initailization_result =
            unsafe { ffi::solClient_initialize(log_level as u32, ptr::null_mut()) };

        if solace_initailization_result != ffi::solClient_returnCode_SOLCLIENT_OK {
            panic!("Could not initialize solace client");
            //return Err(SolaceError);
        }
        let mut ctx: ffi::solClient_opaqueContext_pt = ptr::null_mut();
        let mut context_func: ffi::solClient_context_createFuncInfo_t =
            ffi::solClient_context_createFuncInfo {
                regFdInfo: ffi::solClient_context_createRegisterFdFuncInfo {
                    regFdFunc_p: None,
                    unregFdFunc_p: None,
                    user_p: ptr::null_mut(),
                },
            };

        let solace_context_result = unsafe {
            ffi::solClient_context_create(
                (&mut ffi::_solClient_contextPropsDefaultWithCreateThread) as *mut *const i8,
                &mut ctx,
                &mut context_func,
                mem::size_of::<ffi::solClient_context_createRegisterFdFuncInfo>(),
            )
        };
        if solace_context_result != ffi::solClient_returnCode_SOLCLIENT_OK {
            panic!("Could not initialize solace context");
            //return Err(SolaceError);
        }
        Ok(Self { _ctx: ctx })
    }
}

impl Drop for SolContext {
    fn drop(&mut self) {
        let return_code = unsafe { ffi::solClient_cleanup() };
        if return_code != ffi::solClient_returnCode_SOLCLIENT_OK {
            // TODO
            // remove
            // undefined behavior to panic in drop
            panic!("Solace context did not drop properly");
        }
    }
}
