//! KLC-2789: bounds the native stack a contract call can use, measured as the wasm call
//! depth and stack weight reached per call.
//!
//! The weight of a frame is `params + locals + max_operand_height + FRAME_OVERHEAD_SLOTS`
//! slots, where `max_operand_height` is the highest the wasm operand stack gets in the
//! function, found by a validation pre-pass over the module before it is compiled
//! (`local_function_operand_heights`). Singlepass keeps every param, local and operand
//! stack value in one value slot (v128 included: it compiles no v128 instruction and no
//! call with a v128 argument). A value slot is 8 bytes, except that on aarch64 every
//! spilled operand stack value moves the stack pointer by 16; `SLOT_BYTES` gives the
//! measured bound, and the calibration test (`tests/stack_weight_calibration_test.rs`)
//! checks that a frame never takes more than `SLOT_BYTES` native bytes per weight slot.
//!
//! Accounting happens at call sites and function entry only. Before every `call` and
//! `call_indirect` the caller adds one level and its own frame weight to the current
//! counters, and removes them again after the call returns. On entry, every function
//! raises the max counters to the current counters plus its own frame. Nothing is done
//! at function exits: a trap aborts the whole call, and the host resets the counters
//! before each call.
//!
//! In the release build (`REPLAY_RECORD_MODE` false), every function entry traps with
//! `unreachable` when its frame would take the stack weight over `STACK_WEIGHT_BUDGET`.
//! That is the same outcome as running out of native stack today: a wasmer trap without
//! a breakpoint, which the Go node reports as an execution failure. In record mode no
//! check is injected and new maximums are logged to stdout for the mainnet replay.
//! Record mode still injects the accounting instructions, so its native frames are not
//! byte-identical to the unpatched executor's: the counter updates around each call site
//! hold up to two extra values on the operand stack, which can add a few slots per
//! frame. A call that came within a few slots per frame of the native stack limit before
//! can therefore run out of stack in the replay.
//!
//! A contract that calls another contract through a host import makes the Go node run
//! `Instance::call` on another instance while the outer call is still running. The Go
//! node runs those hooks in a new goroutine, and the outer call's OS thread stays locked
//! in its callback, so today the nested call always runs on another thread, with its own
//! native stack, and starts from zero there. If a nested call runs on the same thread,
//! it starts its counters at the outer instance's current counters plus
//! `NESTED_CALL_OVERHEAD_WEIGHT`, so calls sharing a native stack share one budget. Both
//! cases depend only on wasm-level accounting and on which thread the Go runtime must use,
//! so they are deterministic.
//!
//! A call can also end without returning to `Instance::call`: when a Go host function
//! panics (a timeout is delivered as a panic too), the Go node recovers in the goroutine
//! that started the top-level call and drops the Rust and C frames in between, so their
//! guards never run. Each active call therefore records the stack address it entered at.
//! A genuine nested call always enters deeper on the same thread's stack (at a lower
//! address) than the call running the host import; a call that enters at or above the
//! innermost recorded address cannot be nested in it, so those entries belong to
//! abandoned calls and are dropped before the new call starts. Without this, every later
//! call on that OS thread would start from the abandoned call's counters.
//!
//! The counters are exported globals named `klv.stack_*` (`RESERVED_EXPORT_NAMES`).
//! `create_global_index` inserts them into the module's export map, which silently
//! replaces a contract export of the same name, whatever its kind (function, memory,
//! table or global). The Go validator's `[a-z0-9_]` rule for endpoint names is not relied
//! on: the pre-pass looks at every export. In the release build a module exporting one of
//! these names fails to compile with `CompileError::Validate("export name reserved by the
//! VM: <name>")`; in record mode it compiles as before and a
//! `KLC2789_EXPORT_COLLISION name=<name> kind=<kind>` line is logged.
//!
//! The globals the metering, breakpoints and opcode control middlewares inject
//! (`LEGACY_INJECTED_GLOBAL_NAMES`) replace contract exports the same way, and their names
//! are valid endpoint names. They are not rejected in either mode: a deployed contract
//! with such an export may run fine today, and rejecting it would change which modules
//! are accepted, which needs replay evidence first. Record mode logs each such collision
//! with `legacy=true`. A new `create_global_index` name must be added to one of the two
//! lists (the `injected_global_names_are_all_listed` test checks it).
//!
//! A start function runs inside `wasmer::Instance::new`, not inside `Instance::call`, so
//! it is outside the depth accounting described above: its counters start from the
//! globals' initial zero rather than from an active outer call's counters. Record mode
//! logs `KLC2789_START_SECTION func=<index>` for every module with a start section, so the
//! replay shows whether any deployed contract has one.
//!
//! This middleware must be pushed after the metering and protected_globals middlewares,
//! so that its injected instructions are neither metered nor rejected as writes to
//! protected globals. It never sets a breakpoint.

use crate::wasmer_breakpoints::BREAKPOINT_VALUE;
use crate::wasmer_helpers::{create_global_index, MiddlewareWithProtectedGlobals};
use crate::wasmer_metering::{METERING_POINTS_LIMIT, METERING_POINTS_USED};
use crate::wasmer_opcode_control::{
    OPCODE_CONTROL_MEMORY_GROW_COUNT, OPCODE_CONTROL_OPERAND_BACKUP,
};
use crate::REPLAY_RECORD_MODE;
use loupe::{MemoryUsage, MemoryUsageTracker};
use std::cell::{Cell, RefCell};
use std::io::Write;
use std::mem;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;
use wasmer::wasmparser::{
    ExternalKind, Operator, Parser, Payload, Type as WpType, TypeOrFuncType as WpTypeOrFuncType,
    ValidPayload, Validator, WasmFeatures,
};
use wasmer::{
    CompileError, ExternType, Features, FunctionMiddleware, Instance, LocalFunctionIndex,
    MiddlewareError, MiddlewareReaderState, Module, ModuleMiddleware, Mutability, Type, Value,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

/// Maximum stack weight, in slots, a call may reach, nested calls on the same thread
/// included.
///
/// Sized so that a call at the budget fits the smallest native stack a validator runs
/// contracts on: the 128 KiB thread stack musl gives Go's cgo threads (measured 133,120
/// bytes under the Go node). Everything else that shares that stack, measured under the Go
/// node's kvm test suite with a release build on linux/amd64, glibc and musl (2026-10-30):
/// - host frames below the first wasm frame plus the host-import path above the deepest
///   wasm frame, beyond `SLOT_BYTES` per slot: 9,044 bytes at most (glibc; 4,644 on musl);
/// - the deepest Go or C work below a host import on the same thread: the Go runtime's
///   system-stack work during a callback fits in 16 KiB (the g0 stack Go gives its own
///   threads), and the only host import that calls C, `verifyBLS` (herumi through cgo),
///   uses 11.6 KiB: 16,384 bytes;
/// - one function prologue past the budget: the entry check runs after the frame's params
///   and locals are allocated, at most 4,000 locals × 8 + 50 params × 16 (the aarch64 worst
///   case per value) = 32,800 bytes. 50 is `MAX_FUNCTION_PARAMS` of the wasmer fork
///   (`engine-universal/src/artifact.rs`), checked before code generation;
/// - a margin of 8,192 bytes.
///
/// (131,072 − 9,044 − 16,384 − 32,800 − 8,192) / `SLOT_BYTES` = 3,232 slots. The budget
/// leaves about 16.6 KiB more on top of the margin, for host imports the kvm test suite
/// does not exercise and for the Go runtime figure, which is reasoned, not measured. The
/// mainnet replay (2026-10-30) reached 378 at most. `SLOT_BYTES` was calibrated without
/// the entry check; the check holds at most three values on the operand stack, within
/// `FRAME_OVERHEAD_SLOTS`.
///
/// A function whose own frame is over the budget still compiles, and traps at every
/// entry. That is deterministic; rejecting it at compile time would make the whole
/// contract fail to load instead of that one function.
///
/// The Go node runs contract-calling hooks (`ExecuteOnDestContext` and the like) in a new
/// goroutine while the caller's thread stays locked in its callback, so a nested contract
/// runs on another OS thread, with its own stack and its own budget. The budget is
/// shared only by a nested call that runs on the same thread (`NESTED_CALL_OVERHEAD_WEIGHT`).
/// Dropping abandoned calls by stack address (module docs) also relies on this: if a
/// contract-calling hook ever runs inline on the caller's thread, active calls must be
/// tracked by instance identity instead, and cleared when the instance is destroyed.
pub const STACK_WEIGHT_BUDGET: i64 = 2400;

/// Stack weight, in slots, charged for the host frames between a contract's call to a
/// host import and a nested `Instance::call` on the same thread: the import path, the Go
/// callback and the C API and executor frames down to the nested call's first wasm frame,
/// measured at about 8 KB under the Go node (about 400 slots), rounded up. The Go node runs
/// nested contract calls on another thread today, so this applies only if that changes.
pub const NESTED_CALL_OVERHEAD_WEIGHT: i64 = 512;

const STACK_DEPTH_CURRENT: &str = "klv.stack_depth_current";
const STACK_DEPTH_MAX: &str = "klv.stack_depth_max";
const STACK_WEIGHT_CURRENT: &str = "klv.stack_weight_current";
const STACK_WEIGHT_MAX: &str = "klv.stack_weight_max";

/// The stack depth counters' export names. The release build rejects a module that
/// exports one of them; record mode only logs it.
pub(crate) const RESERVED_EXPORT_NAMES: [&str; 4] = [
    STACK_DEPTH_CURRENT,
    STACK_DEPTH_MAX,
    STACK_WEIGHT_CURRENT,
    STACK_WEIGHT_MAX,
];

/// The export names of the globals the metering, breakpoints and opcode control
/// middlewares inject. A collision with one of them is only logged, in record mode.
pub(crate) const LEGACY_INJECTED_GLOBAL_NAMES: [&str; 5] = [
    METERING_POINTS_LIMIT,
    METERING_POINTS_USED,
    BREAKPOINT_VALUE,
    OPCODE_CONTROL_MEMORY_GROW_COUNT,
    OPCODE_CONTROL_OPERAND_BACKUP,
];

/// Fixed slots added to every frame's params + locals + operand stack height: the return
/// address, saved frame pointer and callee-saved registers, and the values the
/// middlewares' injected instructions hold on top of the contract's operand stack (the
/// metering and stack depth updates around a call site, up to three).
pub const FRAME_OVERHEAD_SLOTS: i64 = 8;

/// Upper bound on the native stack bytes one weight slot stands for, on both supported
/// architectures. The calibration test (`tests/stack_weight_calibration_test.rs`) measured
/// at most 15.92 bytes per slot on aarch64, where singlepass moves the stack pointer by 16
/// bytes per spilled operand stack value, and 10.81 on x86_64 (a call with 49 arguments;
/// 8 bytes per value or local otherwise). 20 keeps a 25% margin over the aarch64 maximum.
pub const SLOT_BYTES: i64 = 20;

static MAX_DEPTH_SO_FAR: AtomicI64 = AtomicI64::new(0);
static MAX_WEIGHT_SO_FAR: AtomicI64 = AtomicI64::new(0);
static MAX_LOCALS_SO_FAR: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static LAST_CALL_STACK_DEPTH_STATS: Cell<Option<StackDepthStats>> = const { Cell::new(None) };
    static STACK_WEIGHT_BUDGET_OVERRIDE: Cell<Option<Option<i64>>> = const { Cell::new(None) };
    /// The calls running `Instance::call` on this thread, innermost last.
    static ACTIVE_INSTANCES: RefCell<Vec<ActiveCall>> = const { RefCell::new(Vec::new()) };
}

/// An `Instance::call` running on this thread.
#[derive(Clone)]
struct ActiveCall {
    instance: Instance,
    /// Stack address in `enter_stack_depth_call` when the call started.
    stack_address: usize,
}

/// An address in the caller's stack frame. Stacks grow down on every supported target.
#[inline(never)]
fn current_stack_address() -> usize {
    let marker = 0u8;
    std::hint::black_box(&marker) as *const u8 as usize
}

/// Drops the calls a new call at `stack_address` cannot be nested in: those that entered
/// at or below it on this thread's stack, which are calls whose frames the Go node
/// abandoned (see the module docs).
fn drop_abandoned_calls(active: &mut Vec<ActiveCall>, stack_address: usize) {
    while active
        .last()
        .is_some_and(|call| call.stack_address <= stack_address)
    {
        active.pop();
    }
}

/// The budget compiled into new modules: `Some(budget)` enforces it, `None` only records.
fn stack_weight_budget() -> Option<i64> {
    STACK_WEIGHT_BUDGET_OVERRIDE
        .with(|budget_override| budget_override.get())
        .unwrap_or(if REPLAY_RECORD_MODE {
            None
        } else {
            Some(STACK_WEIGHT_BUDGET)
        })
}

/// Test support: modules compiled on this thread while the returned guard lives use
/// `budget` (`None` = record only) instead of the `REPLAY_RECORD_MODE` /
/// `STACK_WEIGHT_BUDGET` constants. Not part of the executor API.
#[doc(hidden)]
pub fn override_stack_weight_budget_for_tests(budget: Option<i64>) -> StackWeightBudgetOverride {
    let previous =
        STACK_WEIGHT_BUDGET_OVERRIDE.with(|budget_override| budget_override.replace(Some(budget)));
    StackWeightBudgetOverride { previous }
}

/// Restores the previous budget override when dropped.
#[doc(hidden)]
pub struct StackWeightBudgetOverride {
    previous: Option<Option<i64>>,
}

impl Drop for StackWeightBudgetOverride {
    fn drop(&mut self) {
        let _ = STACK_WEIGHT_BUDGET_OVERRIDE
            .try_with(|budget_override| budget_override.set(self.previous));
    }
}

/// The stack depth counters of an instance, as read after a call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StackDepthStats {
    pub depth_current: i64,
    pub depth_max: i64,
    pub weight_current: i64,
    pub weight_max: i64,
}

/// The counters read after the last `Instance::call` on this thread, or `None` if the
/// instance does not carry the stack depth globals.
pub fn last_call_stack_depth_stats() -> Option<StackDepthStats> {
    LAST_CALL_STACK_DEPTH_STATS.with(|stats| stats.get())
}

#[derive(Clone, Debug, MemoryUsage)]
struct StackDepthGlobalIndexes {
    depth_current_global_index: GlobalIndex,
    depth_max_global_index: GlobalIndex,
    weight_current_global_index: GlobalIndex,
    weight_max_global_index: GlobalIndex,
}

/// Returns the maximum operand stack height of every local function, in code section
/// order, while validating `wasm_bytes` exactly as wasmer does before compiling it (same
/// features, same order, same error). Never panics on malformed input.
///
/// Only once the whole module is valid, applies the export name rules of the module docs
/// for `REPLAY_RECORD_MODE`: the release build fails on an export with a reserved name,
/// record mode logs export collisions and start sections. A module that is not valid
/// therefore fails with the same error in both modes as without these rules.
pub(crate) fn local_function_operand_heights(
    wasm_bytes: &[u8],
    features: &Features,
) -> Result<Vec<u32>, CompileError> {
    let scan = scan_module(wasm_bytes, features)?;
    let log_lines = module_check_outcome(&scan, REPLAY_RECORD_MODE)?;
    if !log_lines.is_empty() {
        // Write errors are ignored: a closed stdout must not turn into a panic.
        let mut stdout = std::io::stdout().lock();
        for line in log_lines {
            let _ = writeln!(stdout, "{line}");
        }
    }
    Ok(scan.operand_heights)
}

/// What the pre-pass learns about a valid module.
#[derive(Debug, Default, PartialEq, Eq)]
struct ModuleScan {
    /// Maximum operand stack height of every local function, in code section order.
    operand_heights: Vec<u32>,
    /// Exports an injected global replaces, in export section order.
    export_collisions: Vec<ExportCollision>,
    /// The start section's function index, if the module has one.
    start_function: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
struct ExportCollision {
    name: String,
    kind: &'static str,
    /// The name is one of `LEGACY_INJECTED_GLOBAL_NAMES`, not `RESERVED_EXPORT_NAMES`.
    legacy: bool,
}

fn external_kind_name(kind: ExternalKind) -> &'static str {
    match kind {
        ExternalKind::Function => "function",
        ExternalKind::Table => "table",
        ExternalKind::Memory => "memory",
        ExternalKind::Global => "global",
        ExternalKind::Event => "event",
        ExternalKind::Type => "type",
        ExternalKind::Module => "module",
        ExternalKind::Instance => "instance",
    }
}

/// The export name rules for a valid module. Release (`replay_record_mode` false): fails
/// on the first export with a reserved name and logs nothing. Record mode: never fails,
/// returns the lines to log.
fn module_check_outcome(
    scan: &ModuleScan,
    replay_record_mode: bool,
) -> Result<Vec<String>, CompileError> {
    if !replay_record_mode {
        // Legacy collisions are not rejected: see the module docs.
        return match scan.export_collisions.iter().find(|c| !c.legacy) {
            Some(collision) => Err(CompileError::Validate(format!(
                "export name reserved by the VM: {}",
                collision.name
            ))),
            None => Ok(Vec::new()),
        };
    }

    let mut log_lines = Vec::new();
    for collision in &scan.export_collisions {
        log_lines.push(format!(
            "KLC2789_EXPORT_COLLISION name={} kind={}{}",
            collision.name,
            collision.kind,
            if collision.legacy { " legacy=true" } else { "" }
        ));
    }
    if let Some(func) = scan.start_function {
        log_lines.push(format!("KLC2789_START_SECTION func={func}"));
    }
    Ok(log_lines)
}

/// Validates `wasm_bytes` and collects the `ModuleScan`. See
/// `local_function_operand_heights`.
fn scan_module(wasm_bytes: &[u8], features: &Features) -> Result<ModuleScan, CompileError> {
    // Mirrors wasmer_compiler::Compiler::validate_module.
    let mut validator = Validator::new();
    validator.wasm_features(WasmFeatures {
        bulk_memory: features.bulk_memory,
        threads: features.threads,
        reference_types: features.reference_types,
        multi_value: features.multi_value,
        simd: features.simd,
        tail_call: features.tail_call,
        module_linking: features.module_linking,
        multi_memory: features.multi_memory,
        memory64: features.memory64,
        exceptions: features.exceptions,
        deterministic_only: false,
    });
    let validate_error =
        |err: wasmer::wasmparser::BinaryReaderError| CompileError::Validate(format!("{}", err));

    // Mirrors Validator::validate_all: the module first, then the function bodies.
    let mut functions = Vec::new();
    let mut export_collisions = Vec::new();
    let mut start_function = None;
    for payload in Parser::new(0).parse_all(wasm_bytes) {
        let payload = payload.map_err(validate_error)?;
        let valid_payload = validator.payload(&payload).map_err(validate_error)?;
        match &payload {
            // The section has just been validated, so it reads again without error.
            Payload::ExportSection(exports) => {
                for export in exports.clone() {
                    let export = export.map_err(validate_error)?;
                    let legacy = if RESERVED_EXPORT_NAMES.contains(&export.field) {
                        false
                    } else if LEGACY_INJECTED_GLOBAL_NAMES.contains(&export.field) {
                        true
                    } else {
                        continue;
                    };
                    export_collisions.push(ExportCollision {
                        name: export.field.to_string(),
                        kind: external_kind_name(export.kind),
                        legacy,
                    });
                }
            }
            Payload::StartSection { func, .. } => start_function = Some(*func),
            _ => {}
        }
        if let ValidPayload::Func(func_validator, body) = valid_payload {
            functions.push((func_validator, body));
        }
    }

    let mut heights = Vec::with_capacity(functions.len());
    for (mut func_validator, body) in functions {
        // Mirrors FuncValidator::validate, tracking the operand stack height after every
        // operator: values are only pushed at the end of an operator, so that covers the
        // highest point.
        let mut reader = body.get_binary_reader();
        func_validator
            .read_locals(&mut reader)
            .map_err(validate_error)?;
        let mut max_height = func_validator.operand_stack_height();
        while !reader.eof() {
            let offset = reader.original_position();
            let operator = reader.read_operator().map_err(validate_error)?;
            func_validator
                .op(offset, &operator)
                .map_err(validate_error)?;
            max_height = max_height.max(func_validator.operand_stack_height());
        }
        func_validator
            .finish(reader.original_position())
            .map_err(validate_error)?;
        heights.push(max_height);
    }

    Ok(ModuleScan {
        operand_heights: heights,
        export_collisions,
        start_function,
    })
}

#[derive(Debug)]
pub(crate) struct StackDepth {
    budget: Option<i64>,
    global_indexes: Mutex<Option<StackDepthGlobalIndexes>>,
    local_function_param_counts: Mutex<Vec<u32>>,
    /// From `local_function_operand_heights`. Empty when the module is deserialized from
    /// the cache: no function is compiled then.
    local_function_operand_heights: Vec<u32>,
}

impl StackDepth {
    pub(crate) fn new(local_function_operand_heights: Vec<u32>) -> Self {
        Self::with_budget(stack_weight_budget(), local_function_operand_heights)
    }

    /// `Some(budget)` enforces the budget, `None` only records.
    pub(crate) fn with_budget(
        budget: Option<i64>,
        local_function_operand_heights: Vec<u32>,
    ) -> Self {
        Self {
            budget,
            global_indexes: Mutex::new(None),
            local_function_param_counts: Mutex::new(vec![]),
            local_function_operand_heights,
        }
    }

    fn get_global_indexes(&self) -> StackDepthGlobalIndexes {
        self.global_indexes.lock().unwrap().clone().unwrap()
    }
}

unsafe impl Send for StackDepth {}
unsafe impl Sync for StackDepth {}

impl MemoryUsage for StackDepth {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.global_indexes.size_of_val(tracker)
            - mem::size_of_val(&self.global_indexes)
            + self.local_function_param_counts.size_of_val(tracker)
            - mem::size_of_val(&self.local_function_param_counts)
            + self.local_function_operand_heights.size_of_val(tracker)
            - mem::size_of_val(&self.local_function_operand_heights)
    }
}

impl ModuleMiddleware for StackDepth {
    fn generate_function_middleware(
        &self,
        local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        let param_count = self.local_function_param_counts.lock().unwrap()
            [local_function_index.as_u32() as usize];
        let max_operand_height = self
            .local_function_operand_heights
            .get(local_function_index.as_u32() as usize)
            .copied();

        Box::new(FunctionStackDepth {
            budget: self.budget,
            param_count,
            local_count: 0,
            max_operand_height,
            entry_injected: false,
            global_indexes: self.get_global_indexes(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_indexes = self.global_indexes.lock().unwrap();

        *global_indexes = Some(StackDepthGlobalIndexes {
            depth_current_global_index: create_global_index(module_info, STACK_DEPTH_CURRENT, 0),
            depth_max_global_index: create_global_index(module_info, STACK_DEPTH_MAX, 0),
            weight_current_global_index: create_global_index(module_info, STACK_WEIGHT_CURRENT, 0),
            weight_max_global_index: create_global_index(module_info, STACK_WEIGHT_MAX, 0),
        });

        // Local function i is function num_imported_functions + i (ModuleInfo::func_index).
        *self.local_function_param_counts.lock().unwrap() = module_info
            .functions
            .values()
            .skip(module_info.num_imported_functions)
            .map(|signature_index| module_info.signatures[*signature_index].params().len() as u32)
            .collect();
    }
}

impl MiddlewareWithProtectedGlobals for StackDepth {
    fn protected_globals(&self) -> Vec<u32> {
        let global_indexes = self.get_global_indexes();
        vec![
            global_indexes.depth_current_global_index.as_u32(),
            global_indexes.depth_max_global_index.as_u32(),
            global_indexes.weight_current_global_index.as_u32(),
            global_indexes.weight_max_global_index.as_u32(),
        ]
    }
}

#[derive(Debug)]
struct FunctionStackDepth {
    budget: Option<i64>,
    param_count: u32,
    local_count: u64,
    /// `None` only if the pre-pass did not run for this module: compiling it fails.
    max_operand_height: Option<u32>,
    entry_injected: bool,
    global_indexes: StackDepthGlobalIndexes,
}

impl FunctionStackDepth {
    fn frame_weight(&self) -> i64 {
        self.param_count as i64
            + self.local_count as i64
            + self.max_operand_height.unwrap_or_default() as i64
            + FRAME_OVERHEAD_SLOTS
    }

    /// `max = max(max, current + delta)`; leaves the value stack unchanged.
    fn inject_max_update(
        &self,
        state: &mut MiddlewareReaderState,
        current_global_index: GlobalIndex,
        max_global_index: GlobalIndex,
        delta: i64,
    ) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: current_global_index.as_u32(),
            },
            Operator::I64Const { value: delta },
            Operator::I64Add,
            Operator::GlobalGet {
                global_index: max_global_index.as_u32(),
            },
            Operator::I64GtS,
            Operator::If {
                ty: WpTypeOrFuncType::Type(WpType::EmptyBlockType),
            },
            Operator::GlobalGet {
                global_index: current_global_index.as_u32(),
            },
            Operator::I64Const { value: delta },
            Operator::I64Add,
            Operator::GlobalSet {
                global_index: max_global_index.as_u32(),
            },
            Operator::End,
        ]);
    }

    /// `global = global + delta` or `global - delta`; leaves the value stack unchanged.
    fn inject_global_update(
        &self,
        state: &mut MiddlewareReaderState,
        global_index: GlobalIndex,
        delta: i64,
        increment: bool,
    ) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: global_index.as_u32(),
            },
            Operator::I64Const { value: delta },
            if increment {
                Operator::I64Add
            } else {
                Operator::I64Sub
            },
            Operator::GlobalSet {
                global_index: global_index.as_u32(),
            },
        ]);
    }

    fn inject_entry_accounting(&self, state: &mut MiddlewareReaderState) {
        self.inject_max_update(
            state,
            self.global_indexes.depth_current_global_index,
            self.global_indexes.depth_max_global_index,
            1,
        );
        self.inject_max_update(
            state,
            self.global_indexes.weight_current_global_index,
            self.global_indexes.weight_max_global_index,
            self.frame_weight(),
        );
    }

    /// `if current + w_self > budget { unreachable }`; leaves the value stack unchanged.
    fn inject_budget_check(&self, state: &mut MiddlewareReaderState, budget: i64) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_indexes.weight_current_global_index.as_u32(),
            },
            Operator::I64Const {
                value: self.frame_weight(),
            },
            Operator::I64Add,
            Operator::I64Const { value: budget },
            Operator::I64GtS,
            Operator::If {
                ty: WpTypeOrFuncType::Type(WpType::EmptyBlockType),
            },
            Operator::Unreachable,
            Operator::End,
        ]);
    }

    fn inject_call_accounting(&self, state: &mut MiddlewareReaderState, increment: bool) {
        self.inject_global_update(
            state,
            self.global_indexes.depth_current_global_index,
            1,
            increment,
        );
        self.inject_global_update(
            state,
            self.global_indexes.weight_current_global_index,
            self.frame_weight(),
            increment,
        );
    }
}

impl FunctionMiddleware for FunctionStackDepth {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        // All local declarations are fed before the first operator.
        if !self.entry_injected {
            if self.max_operand_height.is_none() {
                return Err(MiddlewareError::new(
                    "stack_depth_middleware",
                    "missing operand stack height",
                ));
            }
            self.inject_entry_accounting(state);
            if let Some(budget) = self.budget {
                self.inject_budget_check(state, budget);
            }
            if REPLAY_RECORD_MODE {
                record_function_locals(self.local_count);
            }
            self.entry_injected = true;
        }

        let is_call = matches!(
            operator,
            Operator::Call { .. } | Operator::CallIndirect { .. }
        );

        if is_call {
            self.inject_call_accounting(state, true);
        }

        state.push_operator(operator);

        if is_call {
            self.inject_call_accounting(state, false);
        }

        Ok(())
    }

    fn feed_local_count(&mut self, count: u32) -> Result<(), MiddlewareError> {
        self.local_count += count as u64;

        Ok(())
    }
}

/// Whether `module` exports the four stack depth counters as the mutable i64 globals the
/// middleware injects. A module cached before KLC-2789 does not.
pub(crate) fn has_stack_depth_counters(module: &Module) -> bool {
    RESERVED_EXPORT_NAMES.iter().all(|name| {
        module.exports().any(|export| {
            export.name() == *name
                && matches!(
                    export.ty(),
                    ExternType::Global(global_type)
                        if global_type.ty == Type::I64 && global_type.mutability == Mutability::Var
                )
        })
    })
}

fn get_global_i64(instance: &Instance, name: &str) -> Option<i64> {
    instance.exports.get_global(name).ok()?.get().i64()
}

fn set_global_i64(instance: &Instance, name: &str, value: i64) {
    if let Ok(global) = instance.exports.get_global(name) {
        let _ = global.set(Value::I64(value));
    }
}

fn read_stack_depth_stats(instance: &Instance) -> Option<StackDepthStats> {
    Some(StackDepthStats {
        depth_current: get_global_i64(instance, STACK_DEPTH_CURRENT)?,
        depth_max: get_global_i64(instance, STACK_DEPTH_MAX)?,
        weight_current: get_global_i64(instance, STACK_WEIGHT_CURRENT)?,
        weight_max: get_global_i64(instance, STACK_WEIGHT_MAX)?,
    })
}

/// Sets the stack depth globals. Missing globals are skipped.
fn write_stack_depth_stats(instance: &Instance, stats: StackDepthStats) {
    set_global_i64(instance, STACK_DEPTH_CURRENT, stats.depth_current);
    set_global_i64(instance, STACK_DEPTH_MAX, stats.depth_max);
    set_global_i64(instance, STACK_WEIGHT_CURRENT, stats.weight_current);
    set_global_i64(instance, STACK_WEIGHT_MAX, stats.weight_max);
}

/// Marks `instance` active on this thread for the lifetime of the guard and sets its
/// counters for the call: zero for a top-level call, or the counters of the innermost
/// active instance plus one level and `NESTED_CALL_OVERHEAD_WEIGHT` for a nested call.
pub(crate) fn enter_stack_depth_call(instance: &Instance) -> StackDepthCallGuard {
    let stack_address = current_stack_address();

    // No borrow of ACTIVE_INSTANCES is held while globals are read or written, nor
    // during the call itself: a nested call borrows it again.
    let active_instances = ACTIVE_INSTANCES
        .try_with(|active| {
            let mut active = active.try_borrow_mut().ok()?;
            drop_abandoned_calls(&mut active, stack_address);
            Some(active.clone())
        })
        .ok()
        .flatten()
        .unwrap_or_default();

    let mut start = StackDepthStats::default();
    if let Some(outer) = active_instances.last() {
        let outer_stats = read_stack_depth_stats(&outer.instance).unwrap_or_default();
        start.depth_current = outer_stats.depth_current.saturating_add(1);
        start.weight_current = outer_stats
            .weight_current
            .saturating_add(NESTED_CALL_OVERHEAD_WEIGHT);
    }

    // A re-entrant call on an instance that is still running further down the stack
    // (A -> A, or A -> B -> A) overwrites the counters that call is still using: put
    // them back when the nested call ends.
    let restore_on_exit = active_instances
        .iter()
        .any(|active| is_same_instance(&active.instance, instance))
        .then(|| read_stack_depth_stats(instance).unwrap_or_default());
    start.depth_max = start.depth_current;
    start.weight_max = start.weight_current;
    write_stack_depth_stats(instance, start);

    let stack_len = ACTIVE_INSTANCES
        .try_with(|active| {
            let mut active = active.try_borrow_mut().ok()?;
            active.push(ActiveCall {
                instance: instance.clone(),
                stack_address,
            });
            Some(active.len() - 1)
        })
        .ok()
        .flatten();

    StackDepthCallGuard {
        instance: instance.clone(),
        stack_len,
        restore_on_exit,
    }
}

fn is_same_instance(a: &Instance, b: &Instance) -> bool {
    match (
        a.exports.get_global(STACK_DEPTH_CURRENT),
        b.exports.get_global(STACK_DEPTH_CURRENT),
    ) {
        (Ok(a), Ok(b)) => a.same(b),
        _ => false,
    }
}

/// Pops the instance pushed by `enter_stack_depth_call` when dropped, also when the
/// call fails or panics. Never panics itself.
pub(crate) struct StackDepthCallGuard {
    instance: Instance,
    /// Length of the active stack before the push; `None` if the push failed.
    stack_len: Option<usize>,
    /// Counters of the outer call still running on the same instance, to restore on exit.
    restore_on_exit: Option<StackDepthStats>,
}

impl Drop for StackDepthCallGuard {
    fn drop(&mut self) {
        if let Some(stack_len) = self.stack_len {
            let _ = ACTIVE_INSTANCES.try_with(|active| {
                if let Ok(mut active) = active.try_borrow_mut() {
                    active.truncate(stack_len);
                }
            });
        }

        if let Some(outer_stats) = self.restore_on_exit {
            let nested_stats = read_stack_depth_stats(&self.instance).unwrap_or_default();
            write_stack_depth_stats(
                &self.instance,
                StackDepthStats {
                    depth_current: outer_stats.depth_current,
                    depth_max: outer_stats.depth_max.max(nested_stats.depth_max),
                    weight_current: outer_stats.weight_current,
                    weight_max: outer_stats.weight_max.max(nested_stats.weight_max),
                },
            );
        }
    }
}

/// Reads the stack depth globals after a call and, in record mode, logs a line when the
/// call sets a new process-wide maximum. Missing globals are skipped.
pub(crate) fn record_stack_depth(instance: &Instance, func_name: &str, trapped: bool) {
    let stats = read_stack_depth_stats(instance);

    LAST_CALL_STACK_DEPTH_STATS.with(|last| last.set(stats));

    if !REPLAY_RECORD_MODE {
        return;
    }

    let Some(stats) = stats else {
        return;
    };

    let previous_max_depth = MAX_DEPTH_SO_FAR.fetch_max(stats.depth_max, Ordering::Relaxed);
    let previous_max_weight = MAX_WEIGHT_SO_FAR.fetch_max(stats.weight_max, Ordering::Relaxed);

    if stats.depth_max > previous_max_depth || stats.weight_max > previous_max_weight {
        // Write errors are ignored: a closed stdout must not turn into a panic.
        let _ = writeln!(
            std::io::stdout().lock(),
            "KLC2789_STACK new_max depth={} weight={} func={} trapped={} max_depth_so_far={} max_weight_so_far={}",
            stats.depth_max,
            stats.weight_max,
            func_name,
            trapped,
            previous_max_depth.max(stats.depth_max),
            previous_max_weight.max(stats.weight_max),
        );
    }
}

/// Record mode only: logs a line when a newly compiled function declares more locals
/// than any function compiled before it in this process (KLC-2861).
fn record_function_locals(local_count: u64) {
    let previous_max_locals = MAX_LOCALS_SO_FAR.fetch_max(local_count, Ordering::Relaxed);

    if local_count > previous_max_locals {
        // Write errors are ignored: a closed stdout must not turn into a panic.
        let _ = writeln!(
            std::io::stdout().lock(),
            "KLC2789_LOCALS new_max locals={local_count}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmer::{imports, CompilerConfig, Singlepass, Store, Target, Universal};

    fn singlepass_features() -> Features {
        Singlepass::default().default_features_for_target(&Target::default())
    }

    fn heights(wat: &str) -> Result<Vec<u32>, CompileError> {
        local_function_operand_heights(
            &wasmer::wat2wasm(wat.as_bytes()).unwrap(),
            &singlepass_features(),
        )
    }

    #[test]
    fn heights_are_per_local_function_in_code_order() {
        let wat = r#"
(module
    (import "env" "f" (func $f (param i32 i32) (result i32)))
    (func)
    (func
        i32.const 1
        i32.const 2
        i32.const 3
        call $f
        drop
        drop)
    (func (result i64)
        block (result i64)
            i64.const 1
            i64.const 2
            i64.add
        end))
"#;
        // The imported function has no body and no entry.
        assert_eq!(heights(wat).unwrap(), vec![0, 3, 2]);
    }

    #[test]
    fn unreachable_code_counts_toward_the_height() {
        // After `unreachable` the stack is polymorphic: wasmparser still counts what the
        // dead code pushes, which can only overestimate.
        let wat = r#"
(module
    (func
        unreachable
        i32.const 1
        i32.const 2
        drop
        drop))
"#;
        assert_eq!(heights(wat).unwrap(), vec![2]);
    }

    #[test]
    fn invalid_modules_return_the_validation_error() {
        let type_error = heights("(module (func i32.add drop))").unwrap_err();
        assert!(
            matches!(&type_error, CompileError::Validate(message) if message.contains("type mismatch")),
            "{type_error:?}"
        );

        // Multi-value is off for singlepass, so block params do not validate.
        let block_params = heights("(module (func i32.const 1 block (param i32) drop end))");
        assert!(matches!(block_params, Err(CompileError::Validate(_))));

        // Malformed bytes: an error, never a panic.
        let valid = wasmer::wat2wasm(b"(module (func i32.const 1 drop))").unwrap();
        for bytes in [
            &b""[..],
            &b"\0asm"[..],
            &valid[..valid.len() - 1],
            &[0xff; 64][..],
        ] {
            assert!(matches!(
                local_function_operand_heights(bytes, &singlepass_features()),
                Err(CompileError::Validate(_))
            ));
        }
        // Positive control: the full module is valid.
        assert_eq!(
            local_function_operand_heights(&valid, &singlepass_features()).unwrap(),
            vec![1]
        );
    }

    fn scan(wat: &str) -> Result<ModuleScan, CompileError> {
        scan_module(
            &wasmer::wat2wasm(wat.as_bytes()).unwrap(),
            &singlepass_features(),
        )
    }

    const COLLIDING_WAT: &str = r#"
(module
    (func $f)
    (func $start)
    (global $g (mut i64) (i64.const 0))
    (table 1 funcref)
    (memory 1)
    (export "main" (func $f))
    (export "klv.stack_depth_current" (func $f))
    (export "klv.stack_depth_max" (memory 0))
    (export "klv.stack_weight_current" (global $g))
    (export "klv.stack_weight_max" (table 0))
    (export "metering_points_used" (func $f))
    (export "breakpoint_value" (global $g))
    (start $start))
"#;

    fn collision(name: &str, kind: &'static str, legacy: bool) -> ExportCollision {
        ExportCollision {
            name: name.to_string(),
            kind,
            legacy,
        }
    }

    #[test]
    fn scan_finds_export_collisions_and_the_start_function() {
        assert_eq!(
            scan(COLLIDING_WAT).unwrap(),
            ModuleScan {
                operand_heights: vec![0, 0],
                export_collisions: vec![
                    collision("klv.stack_depth_current", "function", false),
                    collision("klv.stack_depth_max", "memory", false),
                    collision("klv.stack_weight_current", "global", false),
                    collision("klv.stack_weight_max", "table", false),
                    collision("metering_points_used", "function", true),
                    collision("breakpoint_value", "global", true),
                ],
                start_function: Some(1),
            }
        );
        // Positive control for the negative: other names do not collide.
        assert_eq!(
            scan("(module (func $f) (export \"main\" (func $f)) (export \"klv.other\" (func $f)))")
                .unwrap(),
            ModuleScan {
                operand_heights: vec![0],
                ..ModuleScan::default()
            }
        );
    }

    #[test]
    fn release_rejects_reserved_names_only() {
        let scan = scan(COLLIDING_WAT).unwrap();
        let err = module_check_outcome(&scan, false).unwrap_err();
        assert!(
            matches!(&err, CompileError::Validate(message)
                if message == "export name reserved by the VM: klv.stack_depth_current"),
            "{err:?}"
        );

        // Legacy collisions and start sections change nothing and log nothing.
        let legacy_only = ModuleScan {
            export_collisions: vec![collision("metering_points_used", "function", true)],
            start_function: Some(0),
            ..ModuleScan::default()
        };
        assert_eq!(
            module_check_outcome(&legacy_only, false).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn record_mode_only_logs() {
        let scan = scan(COLLIDING_WAT).unwrap();
        assert_eq!(
            module_check_outcome(&scan, true).unwrap(),
            vec![
                "KLC2789_EXPORT_COLLISION name=klv.stack_depth_current kind=function",
                "KLC2789_EXPORT_COLLISION name=klv.stack_depth_max kind=memory",
                "KLC2789_EXPORT_COLLISION name=klv.stack_weight_current kind=global",
                "KLC2789_EXPORT_COLLISION name=klv.stack_weight_max kind=table",
                "KLC2789_EXPORT_COLLISION name=metering_points_used kind=function legacy=true",
                "KLC2789_EXPORT_COLLISION name=breakpoint_value kind=global legacy=true",
                "KLC2789_START_SECTION func=1",
            ]
        );
        assert_eq!(
            module_check_outcome(&ModuleScan::default(), true).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn invalid_module_with_a_reserved_export_keeps_its_validation_error() {
        // The export section is valid; the function body is not.
        let err = local_function_operand_heights(
            &wasmer::wat2wasm(
                b"(module (func $f i32.add drop) (export \"klv.stack_depth_max\" (func $f)))",
            )
            .unwrap(),
            &singlepass_features(),
        )
        .unwrap_err();
        assert!(
            matches!(&err, CompileError::Validate(message) if message.contains("type mismatch")),
            "{err:?}"
        );
    }

    /// An instance exporting the four counters as plain mutable globals.
    fn counters_instance() -> Instance {
        let wat = format!(
            r#"(module
                (global (export "{STACK_DEPTH_CURRENT}") (mut i64) (i64.const 0))
                (global (export "{STACK_DEPTH_MAX}") (mut i64) (i64.const 0))
                (global (export "{STACK_WEIGHT_CURRENT}") (mut i64) (i64.const 0))
                (global (export "{STACK_WEIGHT_MAX}") (mut i64) (i64.const 0)))"#
        );
        let store = Store::new(&Universal::new(Singlepass::default()).engine());
        let module = Module::new(&store, wat).unwrap();
        Instance::new(&module, &imports! {}).unwrap()
    }

    fn active_call_count() -> usize {
        ACTIVE_INSTANCES.with(|active| active.borrow().len())
    }

    /// Enters a call from a frame below the caller's and drops its guard without running
    /// it, like a call whose frames the Go node abandoned after a host panic.
    #[inline(never)]
    fn abandon_call_further_down(instance: &Instance) {
        let padding = [0u8; 4096];
        std::hint::black_box(&padding);
        mem::forget(enter_stack_depth_call(instance));
    }

    #[inline(never)]
    fn enter_further_down(instance: &Instance) -> StackDepthCallGuard {
        let padding = [0u8; 4096];
        std::hint::black_box(&padding);
        enter_stack_depth_call(instance)
    }

    fn stats_with_weight(weight: i64) -> StackDepthStats {
        StackDepthStats {
            depth_current: 3,
            depth_max: 3,
            weight_current: weight,
            weight_max: weight,
        }
    }

    #[test]
    fn a_call_after_an_abandoned_call_starts_from_zero() {
        let abandoned = counters_instance();
        abandon_call_further_down(&abandoned);
        write_stack_depth_stats(&abandoned, stats_with_weight(500));
        assert_eq!(active_call_count(), 1);

        let next = counters_instance();
        let guard = enter_stack_depth_call(&next);
        assert_eq!(
            read_stack_depth_stats(&next),
            Some(StackDepthStats::default())
        );
        assert_eq!(active_call_count(), 1);
        drop(guard);
        assert_eq!(active_call_count(), 0);
    }

    #[test]
    fn a_call_at_the_same_depth_as_an_abandoned_call_starts_from_zero() {
        let abandoned = counters_instance();
        mem::forget(enter_further_down(&abandoned));
        write_stack_depth_stats(&abandoned, stats_with_weight(500));

        let next = counters_instance();
        let guard = enter_further_down(&next);
        assert_eq!(
            read_stack_depth_stats(&next),
            Some(StackDepthStats::default())
        );
        drop(guard);
        assert_eq!(active_call_count(), 0);
    }

    #[test]
    fn a_deeper_call_is_still_nested() {
        let outer = counters_instance();
        let outer_guard = enter_stack_depth_call(&outer);
        write_stack_depth_stats(&outer, stats_with_weight(500));

        let nested = counters_instance();
        let nested_guard = enter_further_down(&nested);
        assert_eq!(
            read_stack_depth_stats(&nested).map(|stats| stats.weight_current),
            Some(500 + NESTED_CALL_OVERHEAD_WEIGHT)
        );
        assert_eq!(active_call_count(), 2);
        drop(nested_guard);
        drop(outer_guard);
        assert_eq!(active_call_count(), 0);
    }
}
