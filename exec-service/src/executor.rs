use crate::{CompilationOptions, ExecutorError, ServiceInstance};

use std::ffi::c_void;

pub trait Executor {
    fn set_context_ptr(&mut self, context_ptr: *mut c_void) -> Result<(), ExecutorError>;

    fn new_instance(
        &self,
        bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn ServiceInstance>, ExecutorError>;
}
