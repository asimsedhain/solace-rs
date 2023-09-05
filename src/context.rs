use crate::{ContextError, SolClientReturnCode, SolaceLogLevel};
use num_traits::FromPrimitive;
use solace_rs_sys as ffi;
use std::mem;
use std::ptr;

type Result<T> = std::result::Result<T, ContextError>;

pub struct RawContext {
    // This pointer must never be allowed to leave the struct
    pub(crate) ctx: ffi::solClient_opaqueContext_pt,
}

impl RawContext {
    /// .
    /// Raw solace context that wraps around the c context
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    ///
    /// # Safety
    /// Context initializes global variables so it is not safe to have multiple solace contexts.
    /// .
    pub unsafe fn new(log_level: SolaceLogLevel) -> Result<Self> {
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

        // enable context thread
        let mut conext_props: [*const i8; 3] = [
            solace_rs_sys::SOLCLIENT_CONTEXT_PROP_CREATE_THREAD.as_ptr() as *const i8,
            solace_rs_sys::SOLCLIENT_PROP_ENABLE_VAL.as_ptr() as *const i8,
            ptr::null(),
        ];

        let solace_context_result = unsafe {
            ffi::solClient_context_create(
                conext_props.as_mut_ptr() as *mut *const i8,
                &mut ctx,
                &mut context_func,
                mem::size_of::<ffi::solClient_context_createRegisterFdFuncInfo>(),
            )
        };

        if SolClientReturnCode::from_i32(solace_context_result) != Some(SolClientReturnCode::Ok) {
            return Err(ContextError::InitializationFailed);
        }
        Ok(Self { ctx })
    }
}

impl Drop for RawContext {
    fn drop(&mut self) {
        let return_code = unsafe { ffi::solClient_cleanup() };
        if return_code != ffi::solClient_returnCode_SOLCLIENT_OK {
            println!("WARNING!! Solace context did not drop properly");
        }
    }
}
