use klever_chain_vm_executor::CompilationOptions;

mod common;

const UNBOUNDED_TABLE_WAT: &str = r#"
(module
    (table (;0;) 0 funcref)
    (memory (;0;) 1)
    (export "memory" (memory 0)))
"#;

const OVERSIZED_TABLE_WAT: &str = r#"
(module
    (table (;0;) 0 200 funcref)
    (memory (;0;) 1)
    (export "memory" (memory 0)))
"#;

fn compilation_options_with_table_cap(max_declared_table_size: usize) -> CompilationOptions {
    CompilationOptions {
        max_declared_table_size,
        ..common::DUMMY_COMPILATION_OPTIONS
    }
}

#[test]
fn max_declared_table_size_no_table() {
    let instance = common::test_instance(common::EMPTY_SC_WAT).unwrap();
    assert_eq!(instance.max_declared_table_size(), 0);
}

#[test]
fn max_declared_table_size_bounded() {
    let instance = common::test_instance(common::ADDER_WAT).unwrap();
    assert_eq!(instance.max_declared_table_size(), 1);
}

#[test]
fn max_declared_table_size_unbounded() {
    let instance = common::test_instance(UNBOUNDED_TABLE_WAT).unwrap();
    assert_eq!(instance.max_declared_table_size(), u32::MAX);
}

#[test]
fn instantiation_rejects_table_exceeding_cap() {
    let options = compilation_options_with_table_cap(100);
    let result = common::test_instance_with_options(OVERSIZED_TABLE_WAT, &options);
    assert!(result.is_err());
}

#[test]
fn instantiation_rejects_unbounded_table() {
    let options = compilation_options_with_table_cap(100);
    let result = common::test_instance_with_options(UNBOUNDED_TABLE_WAT, &options);
    assert!(result.is_err());
}

#[test]
fn instantiation_accepts_table_within_cap() {
    let options = compilation_options_with_table_cap(100);
    let result = common::test_instance_with_options(common::ADDER_WAT, &options);
    assert!(result.is_ok());
}

#[test]
fn instantiation_ignores_cap_when_disabled() {
    let options = compilation_options_with_table_cap(usize::MAX);
    let result = common::test_instance_with_options(UNBOUNDED_TABLE_WAT, &options);
    assert!(result.is_ok());
}

#[test]
fn instantiation_rejects_unbounded_table_even_when_cap_equals_u32_max() {
    let options = compilation_options_with_table_cap(u32::MAX as usize);
    let result = common::test_instance_with_options(UNBOUNDED_TABLE_WAT, &options);
    assert!(result.is_err());
}
