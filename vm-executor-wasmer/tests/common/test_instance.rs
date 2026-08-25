use klever_chain_vm_executor::{CompilationOptions, ExecutorError, ExecutorService, Instance, VMHooksDefault};
use klever_chain_vm_executor_wasmer::BasicExecutorService;
use wasmer::wat2wasm;

pub const DUMMY_COMPILATION_OPTIONS: CompilationOptions = CompilationOptions {
    gas_limit: 0,
    unmetered_locals: 0,
    max_memory_grow: 0,
    max_memory_grow_delta: 0,
    max_declared_table_size: usize::MAX,
    opcode_trace: false,
    metering: false,
    runtime_breakpoints: false,
};

pub fn test_instance(wat: &str) -> Result<Box<dyn Instance>, ExecutorError> {
    test_instance_with_options(wat, &DUMMY_COMPILATION_OPTIONS)
}

pub fn test_instance_with_options(
    wat: &str,
    options: &CompilationOptions,
) -> Result<Box<dyn Instance>, ExecutorError> {
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
    let service = BasicExecutorService::new();
    let executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
    executor.new_instance(&wasm_bytes, options)
}
