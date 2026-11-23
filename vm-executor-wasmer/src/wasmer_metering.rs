use crate::wasmer_breakpoints::{Breakpoints, BREAKPOINT_VALUE_OUT_OF_GAS};
use crate::wasmer_helpers::{
    create_global_index, is_control_flow_operator, MiddlewareWithProtectedGlobals,
};
use crate::{get_local_cost, get_opcode_cost, REPLAY_RECORD_MODE};
use loupe::{MemoryUsage, MemoryUsageTracker};
use klever_chain_vm_executor::OpcodeCost;
use std::io::Write;
use std::mem;
use std::sync::{Arc, Mutex};
use wasmer::wasmparser::Operator;
use wasmer::{
    FunctionMiddleware, Instance, LocalFunctionIndex, MiddlewareError, MiddlewareReaderState,
    ModuleMiddleware,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

pub(crate) const METERING_POINTS_LIMIT: &str = "metering_points_limit";
pub(crate) const METERING_POINTS_USED: &str = "metering_points_used";
pub(crate) const METERING_TABLE_INIT_PER_ELEMENT: &str = "metering_table_init_per_element";
pub(crate) const METERING_TABLE_INIT_ELEMENT_COUNT: &str = "metering_table_init_element_count";
const MAX_LOCAL_COUNT: u32 = 4000;

// KLC-2861: wasmer calls `feed_local_count` once per local declaration group, so the
// MAX_LOCAL_COUNT check alone can be bypassed by splitting a function's locals into several
// declarations, and the `unmetered_locals` allowance is granted once per declaration. In the
// release build (`REPLAY_RECORD_MODE` false) the limit is also enforced on the function's
// total locals, and locals are charged once per function: the locals beyond
// `unmetered_locals` in total. In the replay build the total is only recorded (stdout lines
// `KLC2861_LOCALS` / `KLC2861_GAS`), never enforced, and locals are still charged per
// declaration, so a replay cannot diverge because of it. The per-declaration limit check is
// the same in both modes.

#[derive(Clone, Debug, MemoryUsage)]
struct MeteringGlobalIndexes {
    points_limit_global_index: GlobalIndex,
    points_used_global_index: GlobalIndex,
    table_init_per_element_global_index: GlobalIndex,
    table_init_element_count_global_index: GlobalIndex,
}

#[derive(Debug)]
pub(crate) struct Metering {
    points_limit: u64,
    unmetered_locals: usize,
    table_init_per_element_cost: u64,
    opcode_cost: Arc<Mutex<OpcodeCost>>,
    breakpoints_middleware: Arc<Breakpoints>,
    global_indexes: Mutex<Option<MeteringGlobalIndexes>>,
}

impl Metering {
    pub(crate) fn new(
        points_limit: u64,
        unmetered_locals: usize,
        table_init_per_element_cost: u64,
        opcode_cost: Arc<Mutex<OpcodeCost>>,
        breakpoints_middleware: Arc<Breakpoints>,
    ) -> Self {
        Self {
            points_limit,
            unmetered_locals,
            table_init_per_element_cost,
            opcode_cost,
            breakpoints_middleware,
            global_indexes: Mutex::new(None),
        }
    }

    fn get_points_limit_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .points_limit_global_index
    }

    fn get_points_used_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .points_used_global_index
    }

    fn get_table_init_per_element_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .table_init_per_element_global_index
    }

    fn get_table_init_element_count_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .table_init_element_count_global_index
    }
}

unsafe impl Send for Metering {}
unsafe impl Sync for Metering {}

impl MemoryUsage for Metering {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.global_indexes.size_of_val(tracker)
            - mem::size_of_val(&self.global_indexes)
    }
}

impl ModuleMiddleware for Metering {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionMetering {
            accumulated_cost: Default::default(),
            unmetered_locals: self.unmetered_locals,
            total_locals: 0,
            local_declarations: 0,
            per_declaration_locals_cost: 0,
            locals_recorded: false,
            opcode_cost: self.opcode_cost.clone(),
            breakpoints_middleware: self.breakpoints_middleware.clone(),
            global_indexes: self.global_indexes.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_indexes = self.global_indexes.lock().unwrap();

        let points_limit = self.points_limit as i64;

        *global_indexes = Some(MeteringGlobalIndexes {
            points_limit_global_index: create_global_index(
                module_info,
                METERING_POINTS_LIMIT,
                points_limit,
            ),
            points_used_global_index: create_global_index(module_info, METERING_POINTS_USED, 0),
            table_init_per_element_global_index: create_global_index(
                module_info,
                METERING_TABLE_INIT_PER_ELEMENT,
                self.table_init_per_element_cost as i64,
            ),
            table_init_element_count_global_index: create_global_index(
                module_info,
                METERING_TABLE_INIT_ELEMENT_COUNT,
                0,
            ),
        });
    }
}

impl MiddlewareWithProtectedGlobals for Metering {
    fn protected_globals(&self) -> Vec<u32> {
        vec![
            self.get_points_limit_global_index().as_u32(),
            self.get_points_used_global_index().as_u32(),
            self.get_table_init_per_element_global_index().as_u32(),
            self.get_table_init_element_count_global_index().as_u32(),
        ]
    }
}

#[derive(Debug)]
struct FunctionMetering {
    accumulated_cost: u64,
    unmetered_locals: usize,
    // Locals declared by the function so far, summed over all its declaration groups.
    total_locals: u32,
    local_declarations: u32,
    // Gas charged for locals by the per-declaration formula.
    per_declaration_locals_cost: u64,
    locals_recorded: bool,
    opcode_cost: Arc<Mutex<OpcodeCost>>,
    breakpoints_middleware: Arc<Breakpoints>,
    global_indexes: MeteringGlobalIndexes,
}

impl FunctionMetering {
    fn inject_points_used_increment(&self, state: &mut MiddlewareReaderState) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_indexes.points_used_global_index.as_u32(),
            },
            Operator::I64Const {
                value: self.accumulated_cost as i64,
            },
            Operator::I64Add,
            Operator::GlobalSet {
                global_index: self.global_indexes.points_used_global_index.as_u32(),
            },
        ]);
    }

    fn inject_table_init_element_charge(&self, state: &mut MiddlewareReaderState) {
        state.extend(&[
            Operator::I64ExtendI32U,
            Operator::GlobalSet {
                global_index: self.global_indexes.table_init_element_count_global_index.as_u32(),
            },
            Operator::GlobalGet {
                global_index: self.global_indexes.points_used_global_index.as_u32(),
            },
            Operator::I64Const {
                value: self.accumulated_cost as i64,
            },
            Operator::I64Add,
            Operator::GlobalGet {
                global_index: self.global_indexes.table_init_element_count_global_index.as_u32(),
            },
            Operator::GlobalGet {
                global_index: self.global_indexes.table_init_per_element_global_index.as_u32(),
            },
            Operator::I64Mul,
            Operator::I64Add,
            Operator::GlobalSet {
                global_index: self.global_indexes.points_used_global_index.as_u32(),
            },
        ]);
    }

    fn restore_table_init_element_count(&self, state: &mut MiddlewareReaderState) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_indexes.table_init_element_count_global_index.as_u32(),
            },
            Operator::I32WrapI64,
        ]);
    }

    // Record-only messages for the replay build: what enforcing the total locals limit and
    // charging locals once per function would change for this function.
    fn function_locals_records(&self) -> Vec<String> {
        let mut records = Vec::new();

        if self.total_locals > MAX_LOCAL_COUNT {
            records.push(format!(
                "KLC2861_LOCALS over_limit total={}",
                self.total_locals
            ));
        }

        let per_function_cost = self.per_function_locals_cost(self.total_locals);
        if per_function_cost != self.per_declaration_locals_cost {
            records.push(format!(
                "KLC2861_GAS diff per_declaration={} per_function={} total_locals={} declarations={}",
                self.per_declaration_locals_cost,
                per_function_cost,
                self.total_locals,
                self.local_declarations
            ));
        }

        records
    }

    // Gas for `total_locals` locals charged once per function: only the locals beyond
    // `unmetered_locals` in total are metered.
    fn per_function_locals_cost(&self, total_locals: u32) -> u64 {
        let metered_locals = (total_locals as u64).saturating_sub(self.unmetered_locals as u64);
        let local_cost = get_local_cost(&self.opcode_cost.lock().unwrap()) as u64;
        metered_locals.saturating_mul(local_cost)
    }

    fn inject_out_of_gas_check(&self, state: &mut MiddlewareReaderState) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_indexes.points_used_global_index.as_u32(),
            },
            Operator::GlobalGet {
                global_index: self.global_indexes.points_limit_global_index.as_u32(),
            },
            Operator::I64GeU,
        ]);
        self.breakpoints_middleware
            .inject_breakpoint_condition(state, BREAKPOINT_VALUE_OUT_OF_GAS);
    }
}

impl FunctionMiddleware for FunctionMetering {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        // All local declarations are fed before the first operator, so the totals are final.
        if REPLAY_RECORD_MODE && !self.locals_recorded {
            self.locals_recorded = true;
            let mut stdout = std::io::stdout().lock();
            for record in self.function_locals_records() {
                let _ = writeln!(stdout, "{record}");
            }
        }

        // Get the cost of the current operator, and add it to the accumulator.
        // This needs to be done before the metering logic, to prevent operators like `Call` from escaping metering in some
        // corner cases.
        let option = get_opcode_cost(&operator, &self.opcode_cost.lock().unwrap());
        match option {
            Some(cost) => self.accumulated_cost += cost as u64,
            None => {
                return Err(MiddlewareError::new(
                    "metering_middleware",
                    format!("Unsupported operator: {operator:?}"),
                ))
            }
        }

        if matches!(operator, Operator::TableInit { .. }) {
            self.inject_table_init_element_charge(state);
            self.inject_out_of_gas_check(state);
            self.restore_table_init_element_count(state);

            self.accumulated_cost = 0;

            state.push_operator(operator);

            return Ok(());
        }

        if is_control_flow_operator(&operator) {
            self.inject_points_used_increment(state);
            self.inject_out_of_gas_check(state);

            self.accumulated_cost = 0;
        }

        state.push_operator(operator);

        Ok(())
    }

    fn feed_local_count(&mut self, count: u32) -> Result<(), MiddlewareError> {
        check_local_count_exceeded(count)?;

        let previous_total_locals = self.total_locals;
        self.total_locals = self.total_locals.saturating_add(count);
        self.local_declarations = self.local_declarations.saturating_add(1);
        if !REPLAY_RECORD_MODE {
            check_local_count_exceeded(self.total_locals)?;

            // Charge only what this declaration adds to the per-function cost, so the
            // charge after the last declaration is the cost of the function's total.
            self.accumulated_cost += self.per_function_locals_cost(self.total_locals)
                - self.per_function_locals_cost(previous_total_locals);
            return Ok(());
        }

        let unmetered_locals = self.unmetered_locals as u32;
        if count > unmetered_locals {
            let metered_locals = count - unmetered_locals;
            let local_cost = get_local_cost(&self.opcode_cost.lock().unwrap());
            let metered_locals_cost = metered_locals * local_cost;
            self.accumulated_cost += metered_locals_cost as u64;
            self.per_declaration_locals_cost = self
                .per_declaration_locals_cost
                .saturating_add(metered_locals_cost as u64);
        }

        Ok(())
    }
}

pub(crate) fn set_points_limit(instance: &Instance, limit: u64) -> Result<(), String> {
    let result = instance.exports.get_global(METERING_POINTS_LIMIT);
    match result {
        Ok(global) => {
            let result = global.set(limit.into());
            match result {
                Ok(_) => Ok(()),
                Err(err) => Err(err.message()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

pub(crate) fn set_table_init_per_element_cost(
    instance: &Instance,
    cost: u64,
) -> Result<(), String> {
    let result = instance.exports.get_global(METERING_TABLE_INIT_PER_ELEMENT);
    match result {
        Ok(global) => {
            let result = global.set(cost.into());
            match result {
                Ok(_) => Ok(()),
                Err(err) => Err(err.message()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

pub(crate) fn set_points_used(instance: &Instance, points: u64) -> Result<(), String> {
    let result = instance.exports.get_global(METERING_POINTS_USED);
    match result {
        Ok(global) => {
            let result = global.set(points.into());
            match result {
                Ok(_) => Ok(()),
                Err(err) => Err(err.message()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

pub(crate) fn get_points_used(instance: &Instance) -> Result<u64, String> {
    let result = instance.exports.get_global(METERING_POINTS_USED);
    match result {
        Ok(global) => {
            let result = global.get().try_into();
            match result {
                Ok(points) => Ok(points),
                Err(err) => Err(err.to_string()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

fn check_local_count_exceeded(count: u32) -> Result<(), MiddlewareError> {
    if count > MAX_LOCAL_COUNT {
        return Err(MiddlewareError::new(
            "metering_middleware",
            format!("maximum number of locals({MAX_LOCAL_COUNT}) exceeded({count})"),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function_metering(unmetered_locals: usize, local_cost: u32) -> FunctionMetering {
        FunctionMetering {
            accumulated_cost: 0,
            unmetered_locals,
            total_locals: 0,
            local_declarations: 0,
            per_declaration_locals_cost: 0,
            locals_recorded: false,
            opcode_cost: Arc::new(Mutex::new(OpcodeCost {
                opcode_localallocate: local_cost,
                ..Default::default()
            })),
            breakpoints_middleware: Arc::new(Breakpoints::new()),
            global_indexes: MeteringGlobalIndexes {
                points_limit_global_index: GlobalIndex::from_u32(0),
                points_used_global_index: GlobalIndex::from_u32(1),
                table_init_per_element_global_index: GlobalIndex::from_u32(2),
                table_init_element_count_global_index: GlobalIndex::from_u32(3),
            },
        }
    }

    fn feed_locals(metering: &mut FunctionMetering, groups: &[u32]) -> Result<(), MiddlewareError> {
        groups
            .iter()
            .try_for_each(|count| metering.feed_local_count(*count))
    }

    #[test]
    fn total_locals_limit_follows_the_mode() {
        let result = feed_locals(&mut function_metering(0, 1), &[2000, 2001]);
        if !REPLAY_RECORD_MODE {
            // Release: the split declarations are rejected like a single one.
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("maximum number of locals(4000) exceeded(4001)"),
                "unexpected error: {err}"
            );
        } else {
            // Replay: accepted, exactly as before the total was tracked.
            assert!(result.is_ok());
        }
    }

    #[test]
    fn total_locals_at_the_limit_are_accepted_in_both_modes() {
        assert!(feed_locals(&mut function_metering(0, 1), &[2000, 2000]).is_ok());
    }

    #[test]
    fn single_declaration_over_the_limit_is_rejected_in_both_modes() {
        assert!(feed_locals(&mut function_metering(0, 1), &[4001]).is_err());
    }

    #[test]
    fn local_charge_follows_the_mode() {
        let mut metering = function_metering(100, 10);
        feed_locals(&mut metering, &[50, 150, 50]).unwrap();

        if !REPLAY_RECORD_MODE {
            // Release: the 150 locals beyond the 100 unmetered ones in total.
            assert_eq!(metering.accumulated_cost, 1500);
        } else {
            // Replay: 50 metered locals in the one declaration over 100.
            assert_eq!(metering.accumulated_cost, 500);
            assert_eq!(metering.per_declaration_locals_cost, 500);
        }
        assert_eq!(metering.total_locals, 250);
        assert_eq!(metering.local_declarations, 3);
    }

    #[test]
    fn local_charge_is_the_same_in_both_modes_for_one_declaration() {
        let mut metering = function_metering(100, 10);
        feed_locals(&mut metering, &[300]).unwrap();
        assert_eq!(metering.accumulated_cost, 2000);

        let mut metering = function_metering(100, 10);
        feed_locals(&mut metering, &[60]).unwrap();
        assert_eq!(metering.accumulated_cost, 0);
    }

    #[test]
    fn records_the_per_function_gas_difference() {
        if !REPLAY_RECORD_MODE {
            // The release build tracks no per-declaration charge to compare with.
            return;
        }
        let mut metering = function_metering(100, 10);
        feed_locals(&mut metering, &[50, 150, 50]).unwrap();

        assert_eq!(
            metering.function_locals_records(),
            vec![
                "KLC2861_GAS diff per_declaration=500 per_function=1500 total_locals=250 declarations=3"
                    .to_string()
            ]
        );
    }

    #[test]
    fn records_nothing_when_both_charges_agree() {
        if !REPLAY_RECORD_MODE {
            // The release build tracks no per-declaration charge to compare with.
            return;
        }
        let mut metering = function_metering(100, 10);
        feed_locals(&mut metering, &[300]).unwrap();
        assert!(metering.function_locals_records().is_empty());

        let mut metering = function_metering(100, 10);
        feed_locals(&mut metering, &[200, 300]).unwrap();
        assert_eq!(metering.function_locals_records().len(), 1);

        let mut metering = function_metering(0, 10);
        feed_locals(&mut metering, &[200, 300]).unwrap();
        assert!(metering.function_locals_records().is_empty());
    }

    #[test]
    fn records_a_total_over_the_limit() {
        // Set directly: in the release mode feed_local_count rejects this total.
        let mut metering = function_metering(0, 0);
        metering.total_locals = 4001;
        metering.local_declarations = 2;

        assert_eq!(
            metering.function_locals_records(),
            vec!["KLC2861_LOCALS over_limit total=4001".to_string()]
        );
    }
}
