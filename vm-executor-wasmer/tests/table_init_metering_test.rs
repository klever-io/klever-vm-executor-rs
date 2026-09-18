use klever_chain_vm_executor::CompilationOptions;

mod common;

fn table_init_wat(segment_len: usize, table_size: usize, copied_elements: usize) -> String {
    let refs = vec!["0"; segment_len].join(" ");
    format!(
        r#"
(module
    (type (;0;) (func))
    (func (;0;) (type 0))
    (func (;1;) (type 0)
        i32.const 0
        i32.const 0
        i32.const {copied_elements}
        table.init 0 0)
    (table (;0;) {table_size} {table_size} funcref)
    (elem func {refs})
    (memory (;0;) 1)
    (export "memory" (memory 0))
    (export "main" (func 1)))
"#
    )
}

fn compilation_options_with_per_element_cost(cost: u64, gas_limit: u64) -> CompilationOptions {
    CompilationOptions {
        gas_limit,
        table_init_per_element_cost: cost,
        ..common::DUMMY_COMPILATION_OPTIONS
    }
}

fn points_used_running_table_init(
    copied_elements: usize,
    per_element_cost: u64,
    gas_limit: u64,
) -> Result<u64, String> {
    let wat = table_init_wat(1000, 1000, copied_elements);
    let options = compilation_options_with_per_element_cost(per_element_cost, gas_limit);
    let instance = common::test_instance_with_options(&wat, &options).unwrap();
    instance.set_points_limit(gas_limit)?;
    instance.set_points_used(0)?;
    instance.call("main")?;
    instance.get_points_used()
}

#[test]
fn table_init_charge_is_proportional_to_copied_element_count() {
    let per_element_cost = 10;

    for copied_elements in [1usize, 10, 100, 1000] {
        let points = points_used_running_table_init(copied_elements, per_element_cost, 1_000_000_000)
            .unwrap();

        assert_eq!(
            points,
            copied_elements as u64 * per_element_cost,
            "copying {copied_elements} elements must cost {per_element_cost} gas each"
        );
    }
}

#[test]
fn table_init_charge_scales_with_the_configured_per_element_cost() {
    let single = points_used_running_table_init(100, 1, 1_000_000_000).unwrap();
    let double = points_used_running_table_init(100, 2, 1_000_000_000).unwrap();

    assert_eq!(single, 100);
    assert_eq!(double, 200);
}

#[test]
fn table_init_keeps_flat_metering_when_per_element_cost_is_zero() {
    // Zero reproduces the pre-fork schedule: the element count must not be charged at
    // all, so a one-element and a thousand-element copy stay indistinguishable.
    let one = points_used_running_table_init(1, 0, 1_000_000_000).unwrap();
    let many = points_used_running_table_init(1000, 0, 1_000_000_000).unwrap();

    assert_eq!(one, many);
    assert_eq!(many, 0);
}

#[test]
fn table_init_fails_when_gas_cannot_cover_the_copied_elements() {
    let per_element_cost = 10;
    let copied_elements = 1000;
    let required = copied_elements as u64 * per_element_cost;

    // The inherited out-of-gas check trips on points_used >= points_limit, so covering
    // the charge exactly is already too little: the limit has to exceed it.
    let sufficient =
        points_used_running_table_init(copied_elements, per_element_cost, required + 1);
    assert_eq!(sufficient, Ok(required));

    let insufficient = points_used_running_table_init(copied_elements, per_element_cost, required);
    assert!(
        insufficient.is_err(),
        "a copy the caller cannot pay for must not run, got {insufficient:?}"
    );
}
