mod wasmer_breakpoints;
mod wasmer_executor;
mod wasmer_helpers;
mod wasmer_imports;
mod wasmer_instance;
mod wasmer_logger;
mod wasmer_metering;
mod wasmer_metering_helpers;
mod wasmer_opcode_control;
mod wasmer_opcode_trace;
mod wasmer_protected_globals;
mod wasmer_service;
mod wasmer_stack_depth;
mod wasmer_vm_hooks;

pub use wasmer_executor::*;
pub use wasmer_instance::*;
pub use wasmer_logger::*;
pub use wasmer_metering_helpers::*;
pub use wasmer_service::*;
pub use wasmer_stack_depth::{
    last_call_stack_depth_stats, override_stack_weight_budget_for_tests, StackDepthStats,
    StackWeightBudgetOverride, FRAME_OVERHEAD_SLOTS, NESTED_CALL_OVERHEAD_WEIGHT, SLOT_BYTES,
    STACK_WEIGHT_BUDGET,
};

/// The one switch between the release build and the mainnet replay build (KLC-2789,
/// KLC-2861). Setting it to `true` is the only edit the replay build needs.
///
/// `false` (release): every function entry traps once the stack weight would exceed
/// `STACK_WEIGHT_BUDGET`, the KLC-2861 limit on a function's total locals is enforced,
/// locals are charged once per function instead of once per declaration, a module exporting a `klv.stack_*` name is rejected, a cached module without the
/// stack depth counters is refused, and nothing is logged.
/// `true` (replay): none of these is enforced, so execution does not diverge from the
/// unpatched executor because of them, and `KLC2789_STACK`, `KLC2789_LOCALS`,
/// `KLC2789_EXPORT_COLLISION`, `KLC2789_START_SECTION`, `KLC2861_LOCALS` and
/// `KLC2861_GAS` lines are logged to stdout. The stack depth accounting is still
/// injected (see `wasmer_stack_depth`).
///
/// Both are compiled into the module: a compiled-code cache written in one mode keeps
/// that mode when loaded by the other, so flipping this requires a fresh cache.
pub const REPLAY_RECORD_MODE: bool = false;
