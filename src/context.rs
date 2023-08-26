use crate::solace::ffi;
use crate::{ContextError, SolClientReturnCode, SolaceLogLevel};
use num_traits::FromPrimitive;
use std::mem;
use std::ptr;

type Result<T> = std::result::Result<T, ContextError>;

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

        if SolClientReturnCode::from_i32(solace_initailization_result)
            != Some(SolClientReturnCode::Ok)
        {
            return Err(ContextError::InitializationFailed);
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

        if SolClientReturnCode::from_i32(solace_context_result) != Some(SolClientReturnCode::Ok) {
            return Err(ContextError::InitializationFailed);
        }
        Ok(Self { _ctx: ctx })
    }
}

impl Drop for SolContext {
    fn drop(&mut self) {
        let return_code = unsafe { ffi::solClient_cleanup() };
        if return_code != ffi::solClient_returnCode_SOLCLIENT_OK {
            println!("WARNING!! Solace context did not drop properly");
        }
    }
}
