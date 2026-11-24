use crate::wasmer_opcode_trace::OpcodeTracer;
use crate::wasmer_protected_globals::ProtectedGlobals;
use crate::wasmer_stack_depth::{
    enter_stack_depth_call, has_stack_depth_counters, local_function_operand_heights,
    record_stack_depth, StackDepth,
};
use crate::{
    wasmer_breakpoints::*, wasmer_imports::generate_import_object, wasmer_metering::*,
    wasmer_opcode_control::OpcodeControl, wasmer_vm_hooks::VMHooksWrapper, WasmerExecutorData,
    REPLAY_RECORD_MODE,
};
use log::trace;
use klever_chain_vm_executor::{
    BreakpointValue, CompilationOptions, ExecutorError, Instance, ServiceError,
};
use klever_chain_vm_executor::{MemLength, MemPtr};

use std::cell::RefCell;
use std::{rc::Rc, sync::Arc};
use std::panic::AssertUnwindSafe;
use wasmer::Universal;
use wasmer::{CompilerConfig, Extern, Module, Store, Target};
use wasmer::{Pages, Singlepass};

const MAX_MEMORY_PAGES_ALLOWED: Pages = Pages(20);

pub struct WasmerInstance {
    wasmer_instance: wasmer::Instance,
    memory_name: String,
}

impl WasmerInstance {
    pub(crate) fn try_new_instance(
        executor_data: Rc<RefCell<WasmerExecutorData>>,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        // Use Singlepass compiler with the default settings
        let mut compiler = Singlepass::default();

        // KLC-2789: the stack depth middleware weighs every frame by its operand stack
        // height, which only a validation pass over the whole function body yields. Run it
        // with the features the engine below compiles with (its default for the target).
        // A module that is not valid fails here with the error Module::new would return.
        // The module is validated twice on purpose, here and again in Module::new: the
        // pre-pass mirrors wasmer's validation exactly, so both agree on every module.
        // Determinism over speed. The pre-pass also applies the reserved export name rule
        // (see `wasmer_stack_depth`).
        trace!("Measuring operand stack heights ...");
        let features = compiler.default_features_for_target(&Target::default());
        let operand_heights = std::panic::catch_unwind(AssertUnwindSafe(|| {
            match wasmer::wat2wasm(wasm_bytes) {
                Ok(wasm) => local_function_operand_heights(&wasm, &features),
                // Module::new reports the same conversion error before compiling anything.
                Err(_) => Ok(Vec::new()),
            }
        }));
        let operand_heights = match operand_heights {
            Ok(Ok(operand_heights)) => operand_heights,
            Ok(Err(err)) => return Err(Box::new(err)),
            Err(_) => return Err(Box::new(ServiceError::new("module compilation panicked"))),
        };

        // Push middlewares
        push_middlewares(
            &mut compiler,
            compilation_options,
            executor_data.clone(),
            operand_heights,
        );

        // Create the store
        let store = Store::new(&Universal::new(compiler).engine());

        trace!("Compiling module ...");
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            Module::new(&store, wasm_bytes)
        }));

        let module = match result {
            Ok(Ok(module)) => module,
            Ok(Err(err)) => return Err(Box::new(err)),
            Err(_) => return Err(Box::new(ServiceError::new("module compilation panicked"))),
        };

        validate_tables(&module, compilation_options.max_declared_table_size)?;

        // Create an empty import object.
        trace!("Generating imports ...");
        let vm_hooks_wrapper = VMHooksWrapper {
            vm_hooks: executor_data.borrow().get_vm_hooks(),
        };
        let import_object = generate_import_object(&store, &vm_hooks_wrapper);

        trace!("Instantiating WasmerInstance ...");
        let wasmer_instance = wasmer::Instance::new(&module, &import_object)?;
        set_points_limit(&wasmer_instance, compilation_options.gas_limit)?;

        // Check that there is exactly one memory in the smart contract, no more, no less
        let memories = get_memories(&wasmer_instance);
        validate_memories(&memories)?;

        // At this point we know that there is exactly one memory
        let memory = memories[0].1;
        // Checks that the memory size is not greater than the maximum allowed
        validate_memory(memory)?;

        trace!("WasmerMemory size: {:#?}", memory.size());
        let memory_name = memories[0].0.clone();

        Ok(Box::new(WasmerInstance {
            wasmer_instance,
            memory_name,
        }))
    }

    pub(crate) fn try_new_instance_from_cache(
        executor_data: Rc<RefCell<WasmerExecutorData>>,
        cache_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        // Use Singlepass compiler with the default settings
        let mut compiler = Singlepass::default();

        // Push middlewares. Nothing is compiled, so no operand stack heights are needed.
        push_middlewares(
            &mut compiler,
            compilation_options,
            executor_data.clone(),
            Vec::new(),
        );

        // Create the store
        let store = Store::new(&Universal::new(compiler).engine());

        trace!("Deserializing module ...");
        let module;
        unsafe {
            module = Module::deserialize(&store, cache_bytes)?;
        };

        // KLC-2789: a module cached before the stack depth middleware existed would run
        // without a stack budget. The release build refuses it before instantiation runs
        // any of its code; record mode loads it as before and skips the counters.
        if !REPLAY_RECORD_MODE && !has_stack_depth_counters(&module) {
            return Err(Box::new(ServiceError::new(
                "cached module has no stack depth counters",
            )));
        }

        validate_tables(&module, compilation_options.max_declared_table_size)?;

        // Create an empty import object.
        trace!("Generating imports ...");
        let vm_hooks_wrapper = VMHooksWrapper {
            vm_hooks: executor_data.borrow().get_vm_hooks(),
        };
        let import_object = generate_import_object(&store, &vm_hooks_wrapper);

        trace!("Instantiating WasmerInstance ...");
        let wasmer_instance = wasmer::Instance::new(&module, &import_object)?;
        set_points_limit(&wasmer_instance, compilation_options.gas_limit)?;

        // Check that there is exactly one memory in the smart contract, no more, no less
        let memories = get_memories(&wasmer_instance);
        validate_memories(&memories)?;

        // At this point we know that there is exactly one memory
        let memory = memories[0].1;
        // Checks that the memory size is not greater than the maximum allowed
        validate_memory(memory)?;

        trace!("WasmerMemory size: {:#?}", memory.size());
        let memory_name = memories[0].0.clone();

        Ok(Box::new(WasmerInstance {
            wasmer_instance,
            memory_name,
        }))
    }

    fn get_memory_ref(&self) -> Result<&wasmer::Memory, String> {
        let result = self.wasmer_instance.exports.get_memory(&self.memory_name);
        match result {
            Ok(memory) => Ok(memory),
            Err(err) => Err(err.to_string()),
        }
    }
}

fn get_memories(wasmer_instance: &wasmer::Instance) -> Vec<(&String, &wasmer::Memory)> {
    let memories = wasmer_instance
        .exports
        .iter()
        .memories()
        .collect::<Vec<_>>();
    memories
}

fn validate_memories(memories: &[(&String, &wasmer::Memory)]) -> Result<(), ExecutorError> {
    if memories.is_empty() {
        return Err(Box::new(ServiceError::new(
            "no memory declared in smart contract",
        )));
    }
    if memories.len() > 1 {
        return Err(Box::new(ServiceError::new(
            "more than one memory declared in smart contract",
        )));
    }

    Ok(())
}

fn validate_memory(memory: &wasmer::Memory) -> Result<(), ExecutorError> {
    let memory_type = memory.ty();
    let max_memory_pages = memory_type.maximum.unwrap_or(memory_type.minimum);

    if max_memory_pages > MAX_MEMORY_PAGES_ALLOWED {
        trace!(
            "Memory size exceeds maximum allowed: {:#?} > {:#?}",
            max_memory_pages,
            MAX_MEMORY_PAGES_ALLOWED
        );
        return Err(Box::new(ServiceError::new(
            "memory size exceeds maximum allowed",
        )));
    }

    Ok(())
}

fn validate_tables(module: &Module, max_declared_table_size: usize) -> Result<(), ExecutorError> {
    if max_declared_table_size == usize::MAX {
        return Ok(());
    }

    for table_type in module.info().tables.values() {
        let exceeds_cap = match table_type.maximum {
            None => true,
            Some(declared_max) => (declared_max as usize) > max_declared_table_size,
        };
        if exceeds_cap {
            trace!(
                "Declared table size exceeds maximum allowed: {:#?} > {:#?}",
                table_type.maximum,
                max_declared_table_size
            );
            return Err(Box::new(ServiceError::new(
                "declared table size exceeds maximum allowed",
            )));
        }
    }

    Ok(())
}

fn push_middlewares(
    compiler: &mut Singlepass,
    compilation_options: &CompilationOptions,
    executor_data: Rc<RefCell<WasmerExecutorData>>,
    local_function_operand_heights: Vec<u32>,
) {
    // Create breakpoints middleware
    let breakpoints_middleware = Arc::new(Breakpoints::new());

    // Create opcode_control middleware
    let opcode_control_middleware = Arc::new(OpcodeControl::new(
        compilation_options.max_memory_grow,
        compilation_options.max_memory_grow_delta,
        breakpoints_middleware.clone(),
    ));

    // Create metering middleware
    let metering_middleware = Arc::new(Metering::new(
        compilation_options.gas_limit,
        compilation_options.unmetered_locals,
        executor_data.borrow().get_opcode_cost(),
        breakpoints_middleware.clone(),
    ));

    // Create stack_depth middleware
    let stack_depth_middleware = Arc::new(StackDepth::new(local_function_operand_heights));

    // Create protected_globals middleware
    let protected_globals_middleware = Arc::new(ProtectedGlobals::new(vec![
        breakpoints_middleware.clone(),
        metering_middleware.clone(),
        stack_depth_middleware.clone(),
    ]));

    trace!("Adding protected_globals middleware ...");
    compiler.push_middleware(protected_globals_middleware);
    trace!("Adding metering middleware ...");
    compiler.push_middleware(metering_middleware);
    trace!("Adding opcode_control middleware ...");
    compiler.push_middleware(opcode_control_middleware);
    trace!("Adding breakpoints middleware ...");
    compiler.push_middleware(breakpoints_middleware);
    // Keep stack_depth after metering and protected_globals: its injected instructions
    // must not be metered nor rejected as writes to protected globals.
    trace!("Adding stack_depth middleware ...");
    compiler.push_middleware(stack_depth_middleware);

    if compilation_options.opcode_trace {
        // Create opcode_tracer middleware
        let opcode_tracer_middleware = Arc::new(OpcodeTracer::new());
        trace!("Adding opcode_tracer middleware ...");
        compiler.push_middleware(opcode_tracer_middleware);
    }
}

impl Instance for WasmerInstance {
    fn call(&self, func_name: &str) -> Result<(), String> {
        trace!("Rust instance call: {func_name}");

        let func = self
            .wasmer_instance
            .exports
            .get_function(func_name)
            .map_err(|_| "function not found".to_string())?;

        let stack_depth_call = enter_stack_depth_call(&self.wasmer_instance);
        let result = func.call(&[]);
        record_stack_depth(&self.wasmer_instance, func_name, result.is_err());
        drop(stack_depth_call);

        match result {
            Ok(_) => {
                trace!("Call succeeded: {func_name}");
                Ok(())
            }
            Err(err) => {
                trace!("Call failed: {func_name} - {err}");
                Err(err.to_string())
            }
        }
    }

    fn check_signatures(&self) -> bool {
        for (_, export) in self.wasmer_instance.exports.iter() {
            if let Extern::Function(endpoint) = export {
                if endpoint.param_arity() > 0 {
                    return false;
                }
                if endpoint.result_arity() > 0 {
                    return false;
                }
            }
        }

        true
    }

    fn has_function(&self, func_name: &str) -> bool {
        self.wasmer_instance.exports.get_function(func_name).is_ok()
    }

    fn get_exported_function_names(&self) -> Vec<String> {
        self.wasmer_instance
            .exports
            .iter()
            .filter_map(|(name, export)| match export {
                Extern::Function(_) => Some(name),
                _ => None,
            })
            .cloned()
            .collect()
    }

    fn set_points_limit(&self, limit: u64) -> Result<(), String> {
        set_points_limit(&self.wasmer_instance, limit)
    }

    fn set_points_used(&self, points: u64) -> Result<(), String> {
        set_points_used(&self.wasmer_instance, points)
    }

    fn get_points_used(&self) -> Result<u64, String> {
        get_points_used(&self.wasmer_instance)
    }

    fn memory_length(&self) -> Result<u64, String> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => Ok(memory.data_size()),
            Err(err) => Err(err),
        }
    }

    fn memory_ptr(&self) -> Result<*mut u8, String> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => Ok(memory.data_ptr()),
            Err(err) => Err(err),
        }
    }

    fn memory_load(&self, mem_ptr: MemPtr, mem_length: MemLength) -> Result<&[u8], ExecutorError> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => unsafe {
                let mem_data = memory.data_unchecked();
                Ok(&mem_data[mem_ptr as usize..=(mem_ptr + mem_length) as usize])
            },
            Err(err) => Err(err.into()),
        }
    }

    fn memory_store(&self, mem_ptr: MemPtr, data: &[u8]) -> Result<(), ExecutorError> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => unsafe {
                let mem_data = memory.data_unchecked_mut();
                mem_data[mem_ptr as usize..mem_ptr as usize + data.len()].copy_from_slice(data);
                Ok(())
            },
            Err(err) => Err(err.into()),
        }
    }

    fn memory_grow(&self, by_num_pages: u32) -> Result<u32, ExecutorError> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => {
                let pages = memory.grow(wasmer::Pages(by_num_pages))?;
                Ok(pages.0)
            }
            Err(err) => Err(err.into()),
        }
    }

    fn set_breakpoint_value(&self, value: BreakpointValue) -> Result<(), String> {
        set_breakpoint_value(&self.wasmer_instance, value.as_u64())
    }

    fn get_breakpoint_value(&self) -> Result<BreakpointValue, String> {
        get_breakpoint_value(&self.wasmer_instance)?.try_into()
    }

    fn reset(&self) -> Result<(), String> {
        self.wasmer_instance.reset()
    }

    fn cache(&self) -> Result<Vec<u8>, String> {
        let module = self.wasmer_instance.module();
        match module.serialize() {
            Ok(bytes) => Ok(bytes),
            Err(err) => Err(err.to_string()),
        }
    }

    fn max_declared_table_size(&self) -> u32 {
        self.wasmer_instance
            .module()
            .info()
            .tables
            .values()
            .map(|table_type| table_type.maximum.unwrap_or(u32::MAX))
            .max()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasmer_stack_depth::{LEGACY_INJECTED_GLOBAL_NAMES, RESERVED_EXPORT_NAMES};
    use klever_chain_vm_executor::VMHooksDefault;
    use wasmer::ExternType;

    /// Every global the middlewares inject must be in one of the two lists the pre-pass
    /// checks contract exports against.
    #[test]
    fn injected_global_names_are_all_listed() {
        let wasm =
            wasmer::wat2wasm(br#"(module (memory 1) (export "memory" (memory 0)))"#).unwrap();
        let mut compiler = Singlepass::default();
        let features = compiler.default_features_for_target(&Target::default());
        let operand_heights = local_function_operand_heights(&wasm, &features).unwrap();
        let options = CompilationOptions {
            gas_limit: 0,
            unmetered_locals: 0,
            max_memory_grow: 0,
            max_memory_grow_delta: 0,
            max_declared_table_size: usize::MAX,
            opcode_trace: false,
            metering: false,
            runtime_breakpoints: false,
        };
        let executor_data = Rc::new(RefCell::new(WasmerExecutorData::new(Box::new(
            VMHooksDefault,
        ))));
        push_middlewares(&mut compiler, &options, executor_data, operand_heights);
        let store = Store::new(&Universal::new(compiler).engine());
        let module = Module::new(&store, &wasm).unwrap();
        assert!(has_stack_depth_counters(&module));

        let mut exported = module
            .exports()
            .filter(|export| matches!(export.ty(), ExternType::Global(_)))
            .map(|export| export.name().to_string())
            .collect::<Vec<_>>();
        exported.sort();
        let mut listed = RESERVED_EXPORT_NAMES
            .iter()
            .chain(LEGACY_INJECTED_GLOBAL_NAMES.iter())
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        listed.sort();
        assert_eq!(exported, listed);
    }
}
