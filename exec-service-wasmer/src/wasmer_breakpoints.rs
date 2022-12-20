use std::mem;
use std::sync::Mutex;

use loupe::{MemoryUsage, MemoryUsageTracker};
use wasmer::wasmparser::{Operator, Type as WpType, TypeOrFuncType as WpTypeOrFuncType};
use wasmer::{
    FunctionMiddleware, Instance, LocalFunctionIndex, MiddlewareError, MiddlewareReaderState,
    ModuleMiddleware,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

use crate::wasmer_helpers::create_global_index;

const BREAKPOINT_VALUE: &str = "breakpoint_value";

pub(crate) const BREAKPOINT_VALUE_NO_BREAKPOINT: u64 = 0;
pub(crate) const BREAKPOINT_VALUE_OUT_OF_GAS: u64 = 4;
pub(crate) const BREAKPOINT_VALUE_MEMORY_LIMIT: u64 = 5;

#[derive(Clone, Debug, MemoryUsage)]
struct BreakpointsGlobalIndex {
    breakpoint_value_global_index: GlobalIndex,
}

#[derive(Debug)]
pub(crate) struct Breakpoints {
    global_index: Mutex<Option<BreakpointsGlobalIndex>>,
}

impl Breakpoints {
    pub(crate) fn new() -> Self {
        Self {
            global_index: Mutex::new(None),
        }
    }

    pub(crate) fn inject_breakpoint_condition<'b>(
        &self,
        state: &mut MiddlewareReaderState<'b>,
        breakpoint_value: u64,
    ) {
        state.extend(&[
            Operator::If {
                ty: WpTypeOrFuncType::Type(WpType::EmptyBlockType),
            },
            Operator::I64Const {
                value: breakpoint_value as i64,
            },
            Operator::GlobalSet {
                global_index: self.get_breakpoint_value_global_index().as_u32(),
            },
            Operator::End,
        ]);
    }

    fn get_breakpoint_value_global_index(&self) -> GlobalIndex {
        self.global_index
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .breakpoint_value_global_index
    }
}

unsafe impl Send for Breakpoints {}
unsafe impl Sync for Breakpoints {}

impl MemoryUsage for Breakpoints {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.global_index.size_of_val(tracker)
            - mem::size_of_val(&self.global_index)
    }
}

impl ModuleMiddleware for Breakpoints {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionBreakpoints {
            global_index: self.global_index.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_index = self.global_index.lock().unwrap();

        *global_index = Some(BreakpointsGlobalIndex {
            breakpoint_value_global_index: create_global_index(
                module_info,
                BREAKPOINT_VALUE,
                BREAKPOINT_VALUE_NO_BREAKPOINT as i64,
            ),
        });
    }
}

#[derive(Debug)]
struct FunctionBreakpoints {
    global_index: BreakpointsGlobalIndex,
}

impl FunctionBreakpoints {
    fn inject_breakpoint_condition_check<'b>(&self, state: &mut MiddlewareReaderState<'b>) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_index.breakpoint_value_global_index.as_u32(),
            },
            Operator::I64Const {
                value: BREAKPOINT_VALUE_NO_BREAKPOINT as i64,
            },
            Operator::I64Ne,
            Operator::If {
                ty: WpTypeOrFuncType::Type(WpType::EmptyBlockType),
            },
            Operator::Unreachable,
            Operator::End,
        ]);
    }

    fn check_invalid_global_set<'b>(&self, operator: &Operator<'b>) -> Result<(), MiddlewareError> {
        if let Operator::GlobalSet { global_index } = *operator {
            if global_index == self.global_index.breakpoint_value_global_index.as_u32() {
                return Err(MiddlewareError::new(
                    "breakpoints_middleware",
                    "invalid global set",
                ));
            }
        }

        Ok(())
    }
}

impl FunctionMiddleware for FunctionBreakpoints {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        // Check for invalid access of breakpoints global
        self.check_invalid_global_set(&operator)?;

        let must_add_breakpoint = matches!(
            operator,
            Operator::Call { .. } | Operator::CallIndirect { .. }
        );

        state.push_operator(operator);

        if must_add_breakpoint {
            self.inject_breakpoint_condition_check(state)
        }

        Ok(())
    }
}

pub(crate) fn set_breakpoint_value(instance: &Instance, value: u64) -> Result<(), String> {
    let result = instance.exports.get_global(BREAKPOINT_VALUE);
    match result {
        Ok(global) => {
            let result = global.set(value.into());
            match result {
                Ok(_) => Ok(()),
                Err(err) => Err(err.message()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

pub(crate) fn get_breakpoint_value(instance: &Instance) -> Result<u64, String> {
    let result = instance.exports.get_global(BREAKPOINT_VALUE);
    match result {
        Ok(global) => {
            let result = global.get().try_into();
            match result {
                Ok(value) => Ok(value),
                Err(err) => Err(err.to_string()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
}
