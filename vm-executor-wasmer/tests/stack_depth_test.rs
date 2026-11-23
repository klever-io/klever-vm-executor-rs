use klever_chain_vm_executor::{
    CompilationOptions, ExecutorService, Instance, OpcodeCost, VMHooksDefault,
};
use klever_chain_vm_executor_wasmer::{
    last_call_stack_depth_stats, override_stack_weight_budget_for_tests, BasicExecutorService,
    StackDepthStats, FRAME_OVERHEAD_SLOTS, NESTED_CALL_OVERHEAD_WEIGHT, REPLAY_RECORD_MODE,
    SLOT_BYTES, STACK_WEIGHT_BUDGET,
};
use stack_depth_hooks::{NestedCallHooks, NestedCallTarget};
use std::cell::RefCell;
use std::rc::Rc;
use wasmer::wat2wasm;

mod common;
mod stack_depth_hooks;

const GAS_LIMIT: u64 = 1_000_000_000;

/// Weight of one frame. `height` is the function's maximum operand stack height, counted
/// by hand from the wat in each test.
fn frame_weight(params: i64, locals: i64, height: i64) -> i64 {
    params + locals + height + FRAME_OVERHEAD_SLOTS
}

fn options_with_gas_limit(gas_limit: u64) -> CompilationOptions {
    CompilationOptions {
        gas_limit,
        ..common::DUMMY_COMPILATION_OPTIONS
    }
}

fn instance_with_opcode_cost(wat: &str, opcode_cost: &OpcodeCost) -> Box<dyn Instance> {
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
    let service = BasicExecutorService::new();
    let mut executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
    executor.set_opcode_cost(opcode_cost).unwrap();
    executor
        .new_instance(&wasm_bytes, &options_with_gas_limit(GAS_LIMIT))
        .unwrap()
}

fn test_instance(wat: &str) -> Box<dyn Instance> {
    instance_with_opcode_cost(wat, &OpcodeCost::default())
}

fn call_and_get_stats(
    instance: &dyn Instance,
    func_name: &str,
) -> (Result<(), String>, StackDepthStats) {
    let result = instance.call(func_name);
    let stats = last_call_stack_depth_stats().expect("stack depth globals must be readable");
    (result, stats)
}

fn recursive_wat(depth: u32) -> String {
    format!(
        r#"
(module
    (func $rec (param $n i32) (param $acc i32) (result i32)
        (local $unused i32)
        local.get $n
        i32.eqz
        if
            local.get $acc
            return
        end
        local.get $n
        i32.const 1
        i32.sub
        local.get $acc
        i32.const 1
        i32.add
        call $rec)
    (func $main
        i32.const {depth}
        i32.const 0
        call $rec
        i32.const {depth}
        i32.ne
        if
            unreachable
        end)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#
    )
}

#[test]
fn recursion_records_exact_depth_and_weight() {
    for depth in [0u32, 10, 100] {
        let instance = test_instance(&recursive_wat(depth));
        let (result, stats) = call_and_get_stats(instance.as_ref(), "main");
        assert_eq!(
            result,
            Ok(()),
            "depth {depth}: the instrumented recursion must still return {depth}"
        );

        // main -> rec(depth) -> ... -> rec(0): main plus depth + 1 rec frames.
        let rec_frames = depth as i64 + 1;
        assert_eq!(stats.depth_max, 1 + rec_frames, "depth {depth}");
        assert_eq!(
            stats.weight_max,
            frame_weight(0, 0, 2) + rec_frames * frame_weight(2, 1, 3),
            "depth {depth}"
        );
        assert_eq!(
            stats.depth_current, 0,
            "depth {depth}: early return must unwind the depth"
        );
        assert_eq!(
            stats.weight_current, 0,
            "depth {depth}: early return must unwind the weight"
        );
    }
}

#[test]
fn locals_from_every_declaration_group_are_weighted() {
    // Distinct adjacent types force wat2wasm to emit one declaration group per run.
    let wat = r#"
(module
    (func $wide (param i32 i64) (result i32)
        (local i32)
        (local i64 i64)
        (local f32 f32 f32)
        (local i32)
        local.get 0)
    (func $main
        i32.const 1
        i64.const 2
        call $wide
        drop)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#;
    let instance = test_instance(wat);
    let (result, stats) = call_and_get_stats(instance.as_ref(), "main");
    assert_eq!(result, Ok(()));
    assert_eq!(stats.depth_max, 2);
    assert_eq!(
        stats.weight_max,
        frame_weight(0, 0, 2) + frame_weight(2, 7, 1)
    );
}

#[test]
fn branches_returns_and_indirect_calls_keep_the_count_balanced() {
    let wat = r#"
(module
    (type $unary (func (param i32) (result i32)))
    (func $leaf (type $unary)
        local.get 0)
    (func $wide (type $unary)
        (local i64 i64 i64 i64 i64 i64 i64 i64 i64 i64)
        local.get 0)
    (func $branchy (param $x i32) (result i32)
        block $b1
            block $b2
                local.get $x
                call $leaf
                br_if $b1
                local.get $x
                call $leaf
                drop
                br $b2
            end
            i32.const 0
            call $leaf
            return
        end
        block $b3
            block $b4
                local.get $x
                call $leaf
                br_table $b3 $b4
            end
        end
        i32.const 1
        call $leaf)
    (func $main
        i32.const 1
        call $branchy
        drop
        i32.const 0
        call $branchy
        drop
        i32.const 7
        i32.const 1
        call_indirect (type $unary)
        i32.const 7
        i32.ne
        if
            unreachable
        end)
    (table 2 2 funcref)
    (elem (i32.const 0) $leaf $wide)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#;
    let instance = test_instance(wat);
    let (result, stats) = call_and_get_stats(instance.as_ref(), "main");
    assert_eq!(result, Ok(()));
    // Deepest chain: main -> branchy -> leaf.
    assert_eq!(stats.depth_max, 3);
    // Heaviest chain: main -call_indirect-> wide or main -> branchy -> leaf.
    assert_eq!(
        stats.weight_max,
        frame_weight(0, 0, 2) + frame_weight(1, 10, 1).max(2 * frame_weight(1, 0, 1))
    );
    assert_eq!(stats.depth_current, 0);
    assert_eq!(stats.weight_current, 0);
}

#[test]
fn trapped_call_is_recorded_and_the_next_call_starts_from_zero() {
    let wat = r#"
(module
    (func $rec (param $n i32)
        local.get $n
        i32.eqz
        if
            unreachable
        end
        local.get $n
        i32.const 1
        i32.sub
        call $rec)
    (func $deep
        i32.const 50
        call $rec)
    (func $shallow
        i32.const 1
        drop)
    (memory 1)
    (export "memory" (memory 0))
    (export "deep" (func $deep))
    (export "shallow" (func $shallow)))
"#;
    let instance = test_instance(wat);

    let (result, stats) = call_and_get_stats(instance.as_ref(), "deep");
    assert!(result.is_err(), "rec(0) must trap");
    assert_eq!(stats.depth_max, 1 + 51);
    assert_eq!(
        stats.weight_max,
        frame_weight(0, 0, 1) + 51 * frame_weight(1, 0, 2)
    );

    let (result, stats) = call_and_get_stats(instance.as_ref(), "shallow");
    assert_eq!(result, Ok(()));
    assert_eq!(
        stats,
        StackDepthStats {
            depth_current: 0,
            depth_max: 1,
            weight_current: 0,
            weight_max: frame_weight(0, 0, 1),
        }
    );
}

#[test]
fn cached_module_records_stack_depth() {
    let wat = recursive_wat(10);
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
    let service = BasicExecutorService::new();
    let executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
    let options = options_with_gas_limit(GAS_LIMIT);
    let cache_bytes = executor
        .new_instance(&wasm_bytes, &options)
        .unwrap()
        .cache()
        .unwrap();
    let instance = executor
        .new_instance_from_cache(&cache_bytes, &options)
        .unwrap();

    let (result, stats) = call_and_get_stats(instance.as_ref(), "main");
    assert_eq!(result, Ok(()));
    assert_eq!(stats.depth_max, 12);
    assert_eq!(
        stats.weight_max,
        frame_weight(0, 0, 2) + 11 * frame_weight(2, 1, 3)
    );
}

fn gas_wat(depth: u32) -> String {
    format!(
        r#"
(module
    (func $rec (param $n i32)
        local.get $n
        i32.eqz
        br_if 0
        local.get $n
        i32.const 1
        i32.sub
        call $rec)
    (func $main
        i32.const {depth}
        call $rec)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#
    )
}

/// The contract's own opcodes in `gas_wat` and `recursive_wat` cost 1. Every opcode the
/// stack depth middleware injects that the contracts do not use costs 1000, so metering
/// them would be impossible to miss.
fn opcode_cost_with_expensive_injected_opcodes() -> OpcodeCost {
    OpcodeCost {
        opcode_localget: 1,
        opcode_i32eqz: 1,
        opcode_brif: 1,
        opcode_i32const: 1,
        opcode_i32sub: 1,
        opcode_call: 1,
        opcode_end: 1,
        opcode_globalget: 1000,
        opcode_globalset: 1000,
        opcode_i64const: 1000,
        opcode_i64add: 1000,
        opcode_i64sub: 1000,
        opcode_i64gts: 1000,
        opcode_if: 1000,
        opcode_unreachable: 1000,
        ..OpcodeCost::default()
    }
}

#[test]
fn injected_instructions_are_not_metered() {
    let opcode_cost = opcode_cost_with_expensive_injected_opcodes();

    // Record mode, enforce mode with the default budget, and enforce mode with a budget
    // the deepest call reaches exactly.
    let exact_budget = frame_weight(0, 0, 1) + 11 * frame_weight(1, 0, 2);
    for (budget, depth) in [None, Some(STACK_WEIGHT_BUDGET), Some(exact_budget)]
        .into_iter()
        .flat_map(|budget| [(budget, 0u32), (budget, 10)])
    {
        let instance = {
            let _budget = override_stack_weight_budget_for_tests(budget);
            instance_with_opcode_cost(&gas_wat(depth), &opcode_cost)
        };
        instance.set_points_limit(GAS_LIMIT).unwrap();
        instance.set_points_used(0).unwrap();
        assert_eq!(instance.call("main"), Ok(()));

        // main charges 2 at `call` and 1 at `end`. Each rec(n > 0) charges 3 at `br_if`,
        // 4 at `call` and 1 at `end`; rec(0) charges 3 and branches past its `end`.
        let expected = 3 + 8 * depth as u64 + 3;
        assert_eq!(
            instance.get_points_used(),
            Ok(expected),
            "budget {budget:?}, depth {depth}"
        );

        let stats = last_call_stack_depth_stats().unwrap();
        assert_eq!(
            stats.depth_max,
            depth as i64 + 2,
            "the middleware must be active"
        );
    }
}

/// The stack depth globals are appended by the middleware, so the contract's own code
/// can only name them with an index past the globals it declares or imports. wasmparser
/// rejects such an index while validating the module, before any middleware runs: that
/// validation is what keeps a contract from writing the counters. The protected_globals
/// middleware check on `global.set` is a second line that a valid module never reaches;
/// it is covered by the unit tests in src/wasmer_protected_globals.rs.
#[test]
fn contract_cannot_set_stack_depth_globals() {
    // The middleware appends its globals after metering (4), opcode control (2) and
    // breakpoints (1), so in a module without globals of its own they are 7..=10.
    for global_index in 7..=10 {
        let wat = format!(
            r#"
(module
    (func $main
        i64.const 0
        global.set {global_index})
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#
        );
        let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
        let service = BasicExecutorService::new();
        let executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
        let err = executor
            .new_instance(&wasm_bytes, &common::DUMMY_COMPILATION_OPTIONS)
            .err()
            .unwrap_or_else(|| panic!("global.set {global_index} must not compile"))
            .to_string();
        assert!(
            err.contains("unknown global: global index out of bounds"),
            "global.set {global_index} must fail validation, got: {err}"
        );
    }

    // Positive control: the same shape against a global the contract declares compiles
    // and runs.
    let wat = r#"
(module
    (global $own (mut i64) (i64.const 1))
    (func $main
        i64.const 0
        global.set $own)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#;
    let instance = test_instance(wat);
    assert_eq!(instance.call("main"), Ok(()));
}

fn instance_with_budget(wat: &str, budget: Option<i64>) -> Box<dyn Instance> {
    let _budget = override_stack_weight_budget_for_tests(budget);
    test_instance(wat)
}

/// Stack weight `recursive_wat(depth)` reaches: main plus depth + 1 rec frames.
fn recursive_wat_weight(depth: u32) -> i64 {
    frame_weight(0, 0, 2) + (depth as i64 + 1) * frame_weight(2, 1, 3)
}

#[test]
fn budget_threshold_is_exact() {
    let depth = 20u32;
    let budget = recursive_wat_weight(depth);

    // Reaching the budget exactly is allowed.
    let instance = instance_with_budget(&recursive_wat(depth), Some(budget));
    let (result, stats) = call_and_get_stats(instance.as_ref(), "main");
    assert_eq!(result, Ok(()));
    assert_eq!(stats.weight_max, budget);

    // One more frame traps on its entry, after the max update.
    let instance = instance_with_budget(&recursive_wat(depth + 1), Some(budget));
    let (result, stats) = call_and_get_stats(instance.as_ref(), "main");
    let err = result.expect_err("one frame over the budget must trap");
    assert!(err.contains("unreachable"), "unexpected trap: {err}");
    assert_eq!(stats.weight_max, recursive_wat_weight(depth + 1));

    // Positive control: the same recursion runs in record mode and with a larger budget.
    for budget in [None, Some(recursive_wat_weight(depth + 1))] {
        let instance = instance_with_budget(&recursive_wat(depth + 1), budget);
        let (result, stats) = call_and_get_stats(instance.as_ref(), "main");
        assert_eq!(result, Ok(()), "budget {budget:?}");
        assert_eq!(stats.weight_max, recursive_wat_weight(depth + 1));
    }
}

#[test]
fn below_budget_runs_match_record_mode() {
    let depth = 30u32;
    let opcode_cost = opcode_cost_with_expensive_injected_opcodes();

    let run = |budget: Option<i64>| {
        let instance = {
            let _budget = override_stack_weight_budget_for_tests(budget);
            instance_with_opcode_cost(&recursive_wat(depth), &opcode_cost)
        };
        instance.set_points_limit(GAS_LIMIT).unwrap();
        instance.set_points_used(0).unwrap();
        let (result, stats) = call_and_get_stats(instance.as_ref(), "main");
        (result, instance.get_points_used(), stats)
    };

    let record = run(None);
    assert_eq!(record.0, Ok(()));
    assert!(record.1.clone().unwrap() > 0, "the run must be metered");
    for budget in [i64::MAX, STACK_WEIGHT_BUDGET, recursive_wat_weight(depth)] {
        assert_eq!(run(Some(budget)), record, "budget {budget}");
    }
}

/// Calls `getGasLeft` from `$mid`, so the host import runs with main and mid on the stack.
const NESTED_CALLER_WAT: &str = r#"
(module
    (import "env" "getGasLeft" (func $getGasLeft (result i64)))
    (func $mid (param i32)
        (local i64 i64 i64)
        call $getGasLeft
        drop)
    (func $main
        i32.const 0
        call $mid)
    (func $leaf
        i32.const 1
        drop)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main))
    (export "leaf" (func $leaf)))
"#;

/// Counters of the caller while `getGasLeft` runs: main and mid are on the stack.
const NESTED_CALLER_DEPTH: i64 = 2;
fn nested_caller_weight() -> i64 {
    frame_weight(0, 0, 1) + frame_weight(1, 3, 1)
}

fn nested_caller_instance(target: &Rc<RefCell<NestedCallTarget>>) -> Rc<dyn Instance> {
    let wasm_bytes = wat2wasm(NESTED_CALLER_WAT.as_bytes()).unwrap();
    let service = BasicExecutorService::new();
    let hooks = NestedCallHooks {
        target: target.clone(),
    };
    let executor = service.new_executor(Box::new(hooks)).unwrap();
    Rc::from(
        executor
            .new_instance(&wasm_bytes, &options_with_gas_limit(GAS_LIMIT))
            .unwrap(),
    )
}

/// Runs main of the caller, which calls `func_name` on `callee` from `getGasLeft`, and
/// returns the caller's stats and the nested call's result and stats.
fn run_nested_call(
    caller: &Rc<dyn Instance>,
    target: &Rc<RefCell<NestedCallTarget>>,
    callee: Rc<dyn Instance>,
    func_name: &str,
) -> (StackDepthStats, Result<(), String>, StackDepthStats) {
    {
        let mut target = target.borrow_mut();
        target.instance = Some(callee);
        target.func_name = func_name.to_string();
        target.records.clear();
    }
    let (result, caller_stats) = call_and_get_stats(caller.as_ref(), "main");
    assert_eq!(result, Ok(()), "the caller must not see the nested result");

    // Drops the callee, which may be the caller itself.
    let mut target = target.borrow_mut();
    target.instance = None;
    let mut records = std::mem::take(&mut target.records);
    assert_eq!(records.len(), 1, "getGasLeft must run the nested call once");
    let (nested_result, nested_stats) = records.pop().unwrap();
    (
        caller_stats,
        nested_result,
        nested_stats.expect("nested stack depth globals must be readable"),
    )
}

#[test]
fn nested_call_starts_from_the_caller_counters() {
    let depth = 10u32;
    let start_depth = NESTED_CALLER_DEPTH + 1;
    let start_weight = nested_caller_weight() + NESTED_CALL_OVERHEAD_WEIGHT;

    for budget in [None, Some(STACK_WEIGHT_BUDGET)] {
        let target = Rc::new(RefCell::new(NestedCallTarget::default()));
        let (caller, callee) = {
            let _budget = override_stack_weight_budget_for_tests(budget);
            (
                nested_caller_instance(&target),
                Rc::<dyn Instance>::from(test_instance(&recursive_wat(depth))),
            )
        };

        let (caller_stats, nested_result, nested_stats) =
            run_nested_call(&caller, &target, callee.clone(), "main");
        assert_eq!(nested_result, Ok(()), "budget {budget:?}");
        assert_eq!(
            nested_stats,
            StackDepthStats {
                depth_current: start_depth,
                depth_max: start_depth + 1 + depth as i64 + 1,
                weight_current: start_weight,
                weight_max: start_weight + recursive_wat_weight(depth),
            },
            "budget {budget:?}"
        );

        // The nested call leaves the caller's own counters alone.
        assert_eq!(
            caller_stats,
            StackDepthStats {
                depth_current: 0,
                depth_max: NESTED_CALLER_DEPTH,
                weight_current: 0,
                weight_max: nested_caller_weight(),
            },
            "budget {budget:?}"
        );

        // Once the caller has returned, a call on the callee starts from zero again.
        let (result, stats) = call_and_get_stats(callee.as_ref(), "main");
        assert_eq!(result, Ok(()));
        assert_eq!(stats.depth_current, 0);
        assert_eq!(stats.weight_max, recursive_wat_weight(depth));
    }
}

#[test]
fn nested_call_shares_the_budget() {
    let depth = 20u32;
    let start_weight = nested_caller_weight() + NESTED_CALL_OVERHEAD_WEIGHT;
    let budget = start_weight + recursive_wat_weight(depth);

    let run = |callee_depth: u32| {
        let target = Rc::new(RefCell::new(NestedCallTarget::default()));
        let (caller, callee) = {
            let _budget = override_stack_weight_budget_for_tests(Some(budget));
            (
                nested_caller_instance(&target),
                Rc::<dyn Instance>::from(test_instance(&recursive_wat(callee_depth))),
            )
        };
        let (_, nested_result, nested_stats) =
            run_nested_call(&caller, &target, callee.clone(), "main");
        (nested_result, nested_stats, callee)
    };

    // Reaching the budget exactly is allowed in a nested call too.
    let (result, stats, _) = run(depth);
    assert_eq!(result, Ok(()));
    assert_eq!(stats.weight_max, budget);

    // One more frame traps, although the callee alone is far below the budget.
    let (result, stats, callee) = run(depth + 1);
    let err = result.expect_err("one frame over the shared budget must trap");
    assert!(err.contains("unreachable"), "unexpected trap: {err}");
    assert_eq!(stats.weight_max, budget + frame_weight(2, 1, 3));

    // Positive control: the same callee instance runs fine as a top-level call.
    let (result, stats) = call_and_get_stats(callee.as_ref(), "main");
    assert_eq!(result, Ok(()));
    assert_eq!(stats.weight_max, recursive_wat_weight(depth + 1));
}

#[test]
fn reentrant_call_restores_the_caller_counters() {
    let start_depth = NESTED_CALLER_DEPTH + 1;
    let start_weight = nested_caller_weight() + NESTED_CALL_OVERHEAD_WEIGHT;

    let target = Rc::new(RefCell::new(NestedCallTarget::default()));
    let caller = nested_caller_instance(&target);

    let (caller_stats, nested_result, nested_stats) =
        run_nested_call(&caller, &target, caller.clone(), "leaf");
    assert_eq!(nested_result, Ok(()));
    assert_eq!(
        nested_stats,
        StackDepthStats {
            depth_current: start_depth,
            depth_max: start_depth + 1,
            weight_current: start_weight,
            weight_max: start_weight + frame_weight(0, 0, 1),
        }
    );

    // The caller's current counters are back to what they were before the re-entrant
    // call, so they unwind to zero; its maxes include the re-entrant call.
    assert_eq!(
        caller_stats,
        StackDepthStats {
            depth_current: 0,
            depth_max: start_depth + 1,
            weight_current: 0,
            weight_max: start_weight + frame_weight(0, 0, 1),
        }
    );
}

fn heavy_recursive_wat(depth: u32, locals: u32) -> String {
    let local_decls = " i64".repeat(locals as usize);
    format!(
        r#"
(module
    (func $rec (param $n i32)
        (local{local_decls})
        local.get $n
        i32.eqz
        br_if 0
        local.get $n
        i32.const 1
        i32.sub
        call $rec)
    (func $main
        i32.const {depth}
        call $rec)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#
    )
}

#[test]
fn default_budget_applies_at_its_exact_threshold() {
    // No override: the module is compiled with REPLAY_RECORD_MODE and
    // STACK_WEIGHT_BUDGET. Heavy frames, about a sixth of the budget each, keep the frame
    // count, and so the test time, low.
    let locals = (STACK_WEIGHT_BUDGET / 6) as u32;
    let rec_weight = frame_weight(1, locals as i64, 2);
    let frames_within_budget = (STACK_WEIGHT_BUDGET - frame_weight(0, 0, 1)) / rec_weight;
    assert!(frames_within_budget > 1);
    let depth_within_budget = (frames_within_budget - 1) as u32;

    // Every frame keeps its locals on the native stack: give the thread room for twice
    // the frames this test reaches.
    let stack_size =
        (frames_within_budget as usize + 2) * (locals as usize * 8 + 1024) * 2 + (16 << 20);

    std::thread::Builder::new()
        .stack_size(stack_size)
        .spawn(move || {
            let instance = test_instance(&heavy_recursive_wat(depth_within_budget, locals));
            let (result, stats) = call_and_get_stats(instance.as_ref(), "main");
            assert_eq!(result, Ok(()));
            assert_eq!(
                stats.weight_max,
                frame_weight(0, 0, 1) + frames_within_budget * rec_weight
            );

            let instance = test_instance(&heavy_recursive_wat(depth_within_budget + 1, locals));
            let (result, stats) = call_and_get_stats(instance.as_ref(), "main");
            assert_eq!(
                stats.weight_max,
                frame_weight(0, 0, 1) + (frames_within_budget + 1) * rec_weight
            );
            if !REPLAY_RECORD_MODE {
                assert!(
                    stats.weight_max > STACK_WEIGHT_BUDGET,
                    "the extra frame must cross the budget"
                );
                let err = result.expect_err("one frame over the default budget must trap");
                assert!(err.contains("unreachable"), "unexpected trap: {err}");
            } else {
                assert_eq!(result, Ok(()), "record mode must not trap");
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

/// The reentrant call goes A -> B -> A: B is the innermost active instance when A is
/// entered again, so A's counters must be found further down the active list.
#[test]
fn reentrant_call_through_another_instance_restores_the_caller_counters() {
    let target_a = Rc::new(RefCell::new(NestedCallTarget::default()));
    let target_b = Rc::new(RefCell::new(NestedCallTarget::default()));
    let a = nested_caller_instance(&target_a);
    let b = nested_caller_instance(&target_b);
    {
        let mut target = target_a.borrow_mut();
        target.instance = Some(b.clone());
        target.func_name = "main".to_string();
    }
    {
        let mut target = target_b.borrow_mut();
        target.instance = Some(a.clone());
        target.func_name = "leaf".to_string();
    }

    let (result, a_stats) = call_and_get_stats(a.as_ref(), "main");

    // Break the A -> B -> A reference cycle before asserting.
    let a_records = std::mem::take(&mut target_a.borrow_mut().records);
    let b_records = std::mem::take(&mut target_b.borrow_mut().records);
    target_a.borrow_mut().instance = None;
    target_b.borrow_mut().instance = None;

    assert_eq!(result, Ok(()));
    // A's own call unwinds to zero once the reentrant call has put its counters back.
    assert_eq!((a_stats.depth_current, a_stats.weight_current), (0, 0));
    let b_start_depth = NESTED_CALLER_DEPTH + 1;
    let b_start_weight = nested_caller_weight() + NESTED_CALL_OVERHEAD_WEIGHT;
    let leaf_start_depth = b_start_depth + NESTED_CALLER_DEPTH + 1;
    let leaf_start_weight = b_start_weight + nested_caller_weight() + NESTED_CALL_OVERHEAD_WEIGHT;
    let leaf_stats = StackDepthStats {
        depth_current: leaf_start_depth,
        depth_max: leaf_start_depth + 1,
        weight_current: leaf_start_weight,
        weight_max: leaf_start_weight + frame_weight(0, 0, 1),
    };
    assert_eq!(b_records, vec![(Ok(()), Some(leaf_stats))]);
    assert_eq!(
        a_records,
        vec![(
            Ok(()),
            Some(StackDepthStats {
                depth_current: b_start_depth,
                depth_max: b_start_depth + NESTED_CALLER_DEPTH,
                weight_current: b_start_weight,
                weight_max: b_start_weight + nested_caller_weight(),
            })
        )]
    );

    // A's current counters are back to its outer call's, so they unwind to zero; its
    // maxes include the reentrant leaf call.
    assert_eq!(
        a_stats,
        StackDepthStats {
            depth_current: 0,
            depth_max: leaf_stats.depth_max,
            weight_current: 0,
            weight_max: leaf_stats.weight_max,
        }
    );
}

/// The stack depth globals are exported by name. A contract endpoint with the old global
/// names must still be callable: the globals' names cannot collide with an endpoint.
#[test]
fn endpoints_named_like_the_counters_are_kept() {
    for name in [
        "stack_depth_current",
        "stack_depth_max",
        "stack_weight_current",
        "stack_weight_max",
    ] {
        let wat = format!(
            r#"
(module
    (func $endpoint
        i32.const 1
        drop)
    (memory 1)
    (export "memory" (memory 0))
    (export "{name}" (func $endpoint)))
"#
        );
        let instance = test_instance(&wat);
        assert!(
            instance.has_function(name),
            "endpoint {name} must be exported"
        );
        let (result, stats) = call_and_get_stats(instance.as_ref(), name);
        assert_eq!(result, Ok(()), "endpoint {name}");
        // Positive control: the counters are still there and counted the call.
        assert_eq!(stats.depth_max, 1, "endpoint {name}");
        assert_eq!(stats.weight_max, frame_weight(0, 0, 1), "endpoint {name}");
    }
}

/// Holds an instance reference that is only used by one thread at a time: the spawning
/// thread blocks on `join` while the stack-size thread runs the call.
struct SendInstance<'a>(&'a dyn Instance);

// SAFETY: see `SendInstance`; the instance is never used from two threads at once.
unsafe impl Send for SendInstance<'_> {}

/// Calls `func_name` on a thread with `stack_size` bytes of native stack.
fn call_on_stack(
    instance: &dyn Instance,
    func_name: &str,
    stack_size: usize,
) -> (Result<(), String>, Option<StackDepthStats>) {
    let instance = SendInstance(instance);
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(stack_size)
            .spawn_scoped(scope, move || {
                let instance = instance;
                let result = instance.0.call(func_name);
                (result, last_call_stack_depth_stats())
            })
            .unwrap()
            .join()
            .unwrap()
    })
}

/// The KLC-2789 review demo: one i64 local, `height` copies of it on the operand stack,
/// summed and dropped. Before the operand stack height was weighted, its frame weighed
/// 5 slots while singlepass spilled `height` values to the native stack.
fn deep_operand_stack_wat(height: u32) -> String {
    let gets = "        local.get 0\n".repeat(height as usize);
    let adds = "        i64.add\n".repeat(height as usize - 1);
    format!(
        r#"
(module
    (func $deep
        (local i64)
{gets}{adds}        drop)
    (func $main
        call $deep)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#
    )
}

#[test]
fn deep_operand_stack_is_weighted_and_traps_at_the_budget() {
    let height = 40_000u32;
    let wat = deep_operand_stack_wat(height);
    let weight = frame_weight(0, 0, 0) + frame_weight(0, 1, height as i64);
    assert!(weight > 40_000);

    // Positive control: in record mode the demo runs on a large stack and the counters
    // see the whole operand stack.
    let instance = instance_with_budget(&wat, None);
    let (result, stats) = call_on_stack(instance.as_ref(), "main", 8 << 20);
    assert_eq!(result, Ok(()));
    assert_eq!(stats.map(|stats| stats.weight_max), Some(weight));

    // A budget whose native bytes fit well within 128 KiB: the demo traps at the budget
    // check on entry to $deep, with the same outcome on every stack size, instead of
    // running out of native stack on the small ones.
    let budget = (64 << 10) / SLOT_BYTES;
    assert!(frame_weight(0, 0, 0) <= budget && weight > budget);
    let instance = instance_with_budget(&wat, Some(budget));
    for stack_size in [128 << 10, 256 << 10, 512 << 10, 8 << 20] {
        let (result, stats) = call_on_stack(instance.as_ref(), "main", stack_size);
        let err = result.expect_err("the demo must trap at the budget");
        assert!(
            err.contains("unreachable"),
            "stack {stack_size}: expected the budget trap, got: {err}"
        );
        assert_eq!(
            stats.map(|stats| stats.weight_max),
            Some(weight),
            "stack {stack_size}"
        );
    }
}

#[test]
fn invalid_module_is_rejected_with_a_validation_error() {
    // i32.add with an empty operand stack: wat2wasm encodes it, validation rejects it.
    let wat = r#"
(module
    (func $main
        i32.add
        drop)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#;
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
    let service = BasicExecutorService::new();
    let executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
    let err = executor
        .new_instance(&wasm_bytes, &common::DUMMY_COMPILATION_OPTIONS)
        .err()
        .expect("an invalid module must not compile")
        .to_string();
    assert!(
        err.starts_with("Validation error"),
        "unexpected error: {err}"
    );
}

/// Singlepass compiles no v128 instruction and no call with a v128 argument. A v128
/// local compiles, but singlepass keeps it in one 8-byte slot like any other value, which
/// the calibration test measures.
#[test]
fn v128_instructions_and_arguments_do_not_compile() {
    for body in [
        "v128.const i64x2 0 0\n        drop",
        "(local v128)\n        local.get 0\n        call $take",
    ] {
        let wat = format!(
            r#"
(module
    (func $take (param v128))
    (func $main
        {body})
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#
        );
        let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
        let service = BasicExecutorService::new();
        let executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
        let result = executor.new_instance(&wasm_bytes, &common::DUMMY_COMPILATION_OPTIONS);
        assert!(result.is_err(), "{body} must not compile");
    }
}

const RESERVED_EXPORT_NAMES: [&str; 4] = [
    "klv.stack_depth_current",
    "klv.stack_depth_max",
    "klv.stack_weight_current",
    "klv.stack_weight_max",
];

fn exported_function_wat(name: &str) -> String {
    format!(
        r#"
(module
    (func $main)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main))
    (export "{name}" (func $main)))
"#
    )
}

fn exported_memory_wat(name: &str) -> String {
    format!(
        r#"
(module
    (func $main)
    (memory 1)
    (export "{name}" (memory 0))
    (export "main" (func $main)))
"#
    )
}

/// Mode-aware. The stack depth counters are exported under the `klv.stack_*` names,
/// which would replace a contract export of the same name: the release build rejects
/// such a module, the replay build only logs it (KLC2789_EXPORT_COLLISION).
#[test]
fn exports_with_reserved_names_are_rejected() {
    let service = BasicExecutorService::new();
    let executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
    for (wat_with_name, control_name) in [
        (exported_function_wat as fn(&str) -> String, "other_endpoint"),
        (exported_memory_wat, "memory"),
    ] {
        // Positive control: the same module with an unreserved name compiles and runs.
        let control = wat_with_name(control_name);
        let instance = executor
            .new_instance(
                &wat2wasm(control.as_bytes()).unwrap(),
                &options_with_gas_limit(GAS_LIMIT),
            )
            .expect("the control module must compile");
        assert_eq!(instance.call("main"), Ok(()));

        for name in RESERVED_EXPORT_NAMES {
            let wat = wat_with_name(name);
            let result = executor.new_instance(
                &wat2wasm(wat.as_bytes()).unwrap(),
                &options_with_gas_limit(GAS_LIMIT),
            );
            let err = result.err().map(|err| err.to_string());
            if REPLAY_RECORD_MODE {
                assert!(
                    !err.as_deref()
                        .unwrap_or_default()
                        .contains("reserved by the VM"),
                    "record mode must not reject {name}: {err:?}"
                );
            } else {
                let err = err.unwrap_or_else(|| panic!("export {name} must be rejected"));
                assert!(
                    err.contains(&format!("export name reserved by the VM: {name}")),
                    "unexpected error for {name}: {err}"
                );
            }
        }
    }
}

/// A module compiled without the executor's middlewares, as an old executor's cache
/// would hold it, that still exports the metering global instantiation sets.
fn stale_cache_bytes(with_stack_depth_counters: bool) -> Vec<u8> {
    let counters = if with_stack_depth_counters {
        RESERVED_EXPORT_NAMES
            .iter()
            .map(|name| format!("(global (export \"{name}\") (mut i64) (i64.const 0))\n"))
            .collect::<String>()
    } else {
        String::new()
    };
    let wat = format!(
        r#"
(module
    (global (export "metering_points_limit") (mut i64) (i64.const 0))
    {counters}
    (func $main)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#
    );
    let store = wasmer::Store::new(&wasmer::Universal::new(wasmer::Singlepass::default()).engine());
    let module = wasmer::Module::new(&store, wat2wasm(wat.as_bytes()).unwrap()).unwrap();
    module.serialize().unwrap()
}

/// Mode-aware. A cached module without the stack depth counters would run without a
/// budget: the release build refuses to load it, the replay build loads it as before.
/// `cached_module_records_stack_depth` covers a cache written by this executor.
#[test]
fn cached_module_without_stack_depth_counters() {
    let service = BasicExecutorService::new();
    let executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
    let options = options_with_gas_limit(GAS_LIMIT);

    // Positive control: the same stale module with the four counter globals loads and
    // runs, so the setup gets past every other instantiation step.
    let instance = executor
        .new_instance_from_cache(&stale_cache_bytes(true), &options)
        .expect("a cached module with the counters must load");
    assert_eq!(instance.call("main"), Ok(()));

    let result = executor.new_instance_from_cache(&stale_cache_bytes(false), &options);
    if REPLAY_RECORD_MODE {
        let instance = result.expect("record mode must load a cache without the counters");
        assert_eq!(instance.call("main"), Ok(()));
        assert_eq!(last_call_stack_depth_stats(), None);
    } else {
        let err = result
            .err()
            .expect("a cache without the counters must not load")
            .to_string();
        assert!(
            err.contains("cached module has no stack depth counters"),
            "unexpected error: {err}"
        );
    }
}

/// Same in both modes: an export named like a legacy injected global and a start section
/// do not change acceptance. Record mode only logs them (KLC2789_EXPORT_COLLISION with
/// legacy=true, KLC2789_START_SECTION).
#[test]
fn legacy_export_collisions_and_start_sections_are_accepted() {
    let wat = r#"
(module
    (func $start)
    (func $main)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main))
    (export "metering_points_used" (func $main))
    (start $start))
"#;
    let instance = test_instance(wat);
    assert_eq!(instance.call("main"), Ok(()));
}
