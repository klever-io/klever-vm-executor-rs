//! Table instructions have no price in the opcode cost schedule, so the metering
//! middleware rejects every one of them at compile time: a contract can't change a
//! table after instantiation. Tables filled by active element segments and called
//! through `call_indirect` keep working.

use klever_chain_vm_executor::CompilationOptions;

mod common;

const COMPILATION_OPTIONS: CompilationOptions = CompilationOptions {
    gas_limit: 1_000_000_000,
    ..common::DUMMY_COMPILATION_OPTIONS
};

const ONE_FUNCTION_TABLE: &str = "(table 1 1 funcref) (elem (i32.const 0) func 0)";

fn module_wat(tables_and_elems: &str, body: &str) -> String {
    format!(
        r#"
(module
    (type (;0;) (func))
    (func (;0;) (type 0))
    (func (;1;) (type 0)
        {body})
    {tables_and_elems}
    (memory (;0;) 1)
    (export "memory" (memory 0))
    (export "main" (func 1)))
"#
    )
}

fn assert_rejected(instruction: &str, tables_and_elems: &str, body: &str) {
    let wat = module_wat(tables_and_elems, body);
    let err = match common::test_instance(&wat) {
        Ok(_) => panic!("{instruction} must not compile"),
        Err(err) => err.to_string(),
    };
    assert!(
        err.contains("Unsupported operator"),
        "{instruction}: unexpected error {err:?}"
    );
}

#[test]
fn table_get_is_rejected() {
    assert_rejected(
        "table.get",
        ONE_FUNCTION_TABLE,
        "i32.const 0 table.get 0 drop",
    );
}

#[test]
fn table_set_is_rejected() {
    assert_rejected(
        "table.set",
        ONE_FUNCTION_TABLE,
        "i32.const 0 ref.null func table.set 0",
    );
}

#[test]
fn table_grow_is_rejected() {
    assert_rejected(
        "table.grow",
        "(table 1 2 funcref)",
        "ref.null func i32.const 1 table.grow 0 drop",
    );
}

#[test]
fn table_size_is_rejected() {
    assert_rejected("table.size", ONE_FUNCTION_TABLE, "table.size 0 drop");
}

#[test]
fn table_init_is_rejected() {
    assert_rejected(
        "table.init",
        "(table 1 1 funcref) (elem func 0)",
        "i32.const 0 i32.const 0 i32.const 1 table.init 0 0",
    );
}

#[test]
fn table_fill_is_rejected() {
    assert_rejected(
        "table.fill",
        ONE_FUNCTION_TABLE,
        "i32.const 0 ref.null func i32.const 1 table.fill 0",
    );
}

#[test]
fn table_copy_is_rejected() {
    assert_rejected(
        "table.copy",
        "(table 2 2 funcref) (elem (i32.const 0) func 0)",
        "i32.const 1 i32.const 0 i32.const 1 table.copy 0 0",
    );
}

#[test]
fn elem_drop_is_rejected() {
    assert_rejected(
        "elem.drop",
        "(table 1 1 funcref) (elem func 0)",
        "elem.drop 0",
    );
}

#[test]
fn call_indirect_through_an_active_element_segment_still_works() {
    let wat = r#"
(module
    (type (;0;) (func))
    (func (;0;) (type 0)
        i32.const 0
        i32.const 0x5eed5eed
        i32.store)
    (func (;1;) (type 0)
        i32.const 0
        call_indirect (type 0))
    (table 1 1 funcref)
    (elem (i32.const 0) func 0)
    (memory (;0;) 1)
    (export "memory" (memory 0))
    (export "main" (func 1)))
"#;
    let instance = common::test_instance_with_options(wat, &COMPILATION_OPTIONS).unwrap();
    instance.call("main").unwrap();
    assert_eq!(
        instance.memory_load(0, 4).unwrap()[..4],
        0x5eed5eedu32.to_le_bytes()
    );
}
