// The function locals limit (MAX_LOCAL_COUNT = 4000) must hold for the whole function, not
// for each local declaration group the binary happens to use.
//
// Mode: the two `*_over_the_limit_do_not_compile` tests branch on REPLAY_RECORD_MODE. In the
// release build (false) split declarations over the limit are rejected; in the replay build
// (true) they compile, because a replay must accept every function the per-declaration check
// accepted. Gas follows the mode too: the release build charges locals once per function,
// the replay build per declaration, as before.
//
// wat2wasm run-length encodes consecutive locals of the same type into one declaration, so
// the modules below alternate i32 and i64 groups to get genuinely separate declarations.

use klever_chain_vm_executor::{
    CompilationOptions, ExecutorError, ExecutorService, Instance, OpcodeCost, VMHooksDefault,
};
use klever_chain_vm_executor_wasmer::{BasicExecutorService, REPLAY_RECORD_MODE};
use wasmer::wat2wasm;

mod common;

const LOCAL_COST: u32 = 10;
const UNMETERED_LOCALS: usize = 100;
const GAS_LIMIT: u64 = 1_000_000_000;

fn locals_decl(groups: &[usize]) -> String {
    groups
        .iter()
        .enumerate()
        .map(|(index, count)| {
            let ty = if index % 2 == 0 { "i32" } else { "i64" };
            format!("(local {})", vec![ty; *count].join(" "))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn module_with_functions(functions: &[&[usize]]) -> String {
    let funcs = functions
        .iter()
        .map(|groups| format!("    (func (type 0) {})\n", locals_decl(groups)))
        .collect::<String>();
    format!(
        r#"
(module
    (type (;0;) (func))
{funcs}    (memory (;0;) 1)
    (export "memory" (memory 0))
    (export "main" (func 0)))
"#
    )
}

fn options() -> CompilationOptions {
    CompilationOptions {
        gas_limit: GAS_LIMIT,
        unmetered_locals: UNMETERED_LOCALS,
        ..common::DUMMY_COMPILATION_OPTIONS
    }
}

fn instance_with_local_cost(wat: &str) -> Result<Box<dyn Instance>, ExecutorError> {
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
    let service = BasicExecutorService::new();
    let mut executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
    executor
        .set_opcode_cost(&OpcodeCost {
            opcode_localallocate: LOCAL_COST,
            ..Default::default()
        })
        .unwrap();
    executor.new_instance(&wasm_bytes, &options())
}

fn compile(functions: &[&[usize]]) -> Result<Box<dyn Instance>, ExecutorError> {
    common::test_instance_with_options(&module_with_functions(functions), &options())
}

fn assert_locals_limit_error(result: Result<Box<dyn Instance>, ExecutorError>, total: usize) {
    let err = match result {
        Ok(_) => panic!("a function with {total} locals must not compile"),
        Err(err) => err.to_string(),
    };
    assert!(
        err.contains(&format!("maximum number of locals(4000) exceeded({total})")),
        "unexpected error: {err}"
    );
}

fn points_used_running_main(functions: &[&[usize]]) -> u64 {
    let instance = instance_with_local_cost(&module_with_functions(functions)).unwrap();
    instance.set_points_limit(GAS_LIMIT).unwrap();
    instance.set_points_used(0).unwrap();
    instance.call("main").unwrap();
    instance.get_points_used().unwrap()
}

// Rejected in the release build, accepted as before in the replay build.
#[test]
fn split_declarations_over_the_limit_do_not_compile() {
    let result = compile(&[&[2000, 2001]]);
    if REPLAY_RECORD_MODE {
        assert!(result.is_ok());
    } else {
        assert_locals_limit_error(result, 4001);
    }
}

// Rejected in the release build, accepted as before in the replay build.
#[test]
fn many_small_split_declarations_over_the_limit_do_not_compile() {
    // Groups below the unmetered allowance, so each declaration is also free today.
    let groups = [100usize; 41];
    let result = compile(&[&groups]);
    if REPLAY_RECORD_MODE {
        assert!(result.is_ok());
    } else {
        assert_locals_limit_error(result, 4100);
    }
}

// Both modes.
#[test]
fn split_declarations_at_the_limit_compile() {
    assert!(compile(&[&[2000, 2000]]).is_ok());
    assert!(compile(&[&[1000, 1000, 1000, 1000]]).is_ok());
}

// Both modes: unchanged per-declaration check.
#[test]
fn single_declaration_over_the_limit_does_not_compile() {
    assert_locals_limit_error(compile(&[&[4001]]), 4001);
    assert!(compile(&[&[4000]]).is_ok());
}

// Both modes: the total is per function, not per module.
#[test]
fn the_limit_applies_to_each_function_separately() {
    assert!(compile(&[&[1500, 1500], &[1500, 1500], &[4000]]).is_ok());
}

// Gas: locals are charged per function in the release build, per declaration (as before)
// in the replay build.
#[test]
fn split_declarations_gas_charge_follows_the_mode() {
    // Per declaration: the unmetered allowance (100) is subtracted from each group,
    // max(50 - 100, 0) + (150 - 100) + max(50 - 100, 0) = 50 metered locals = 500 gas.
    // Per function: (250 - 100) * 10 = 1500. Either result also proves the three groups
    // reached the metering middleware as separate declarations.
    let (split, small_groups, two_large_groups) = if REPLAY_RECORD_MODE {
        (500, 0, 3000)
    } else {
        (1500, 2000, 4000)
    };
    assert_eq!(points_used_running_main(&[&[50, 150, 50]]), split);

    // Groups at or below the allowance: free per declaration, (300 - 100) * 10 per function.
    assert_eq!(points_used_running_main(&[&[100, 100, 100]]), small_groups);

    // Split groups each above the allowance: (200 - 100) + (300 - 100) = 300 metered per
    // declaration, (500 - 100) = 400 per function.
    assert_eq!(points_used_running_main(&[&[200, 300]]), two_large_groups);
}

// Both modes: one declaration costs the same under both formulas.
#[test]
fn a_single_declaration_has_the_same_gas_charge_in_both_modes() {
    // (300 - 100) * 10 = 2000.
    assert_eq!(points_used_running_main(&[&[300]]), 2000);
    // At or below the allowance: free.
    assert_eq!(points_used_running_main(&[&[100]]), 0);
}
