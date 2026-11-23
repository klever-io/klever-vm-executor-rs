//! KLC-2789: calibrates `SLOT_BYTES` against the native stack singlepass really uses.
//!
//! Every shape is a recursive function `$rec` with a given frame layout (params, locals,
//! operand stack, blocks, call arguments) that recurses while its operand stack is at its
//! highest. For two recursion depths, a binary search over `std::thread::Builder`
//! stack sizes finds the smallest native stack the call succeeds on. The difference
//! between the two, divided by the number of extra frames, is the native size of one
//! `$rec` frame; dividing it by the frame's weight gives the bytes per weight slot.
//!
//! The test fails if a frame takes more than `SLOT_BYTES` per slot, or if a whole call
//! takes more than `SLOT_BYTES * weight + CALL_OVERHEAD_BYTES`. Run it with `--nocapture`
//! to print the measurements.
//!
//! The search relies on the stack running out at the guard page below the thread's
//! stack. On aarch64, singlepass moves the stack pointer by 16 bytes per spilled value but
//! writes only 8 of them, so a frame leaves an untouched gap of 8 bytes per spilled value
//! below its values. Once that gap is as large as the guard page (16 KiB on macOS, 4 KiB
//! on Linux), the next frame can write past the guard without faulting and the search
//! under-measures. The operand stack shapes therefore hold at most 400 values (a 3.2 KiB
//! gap) on aarch64; larger ones, up to the 40,000-value review demo, are measured on
//! x86_64 only, where nothing is skipped.
//!
//! On Linux, glibc reuses cached thread stacks up to four times larger than the requested
//! size, which makes every frame look smaller. Run the test there with
//! `GLIBC_TUNABLES=glibc.pthread.stack_cache_size=0`; the test fails if a local measures
//! less than the 8 bytes it is known to take.
//!
//! Measured (debug build, bytes per weight slot / native bytes below the first frame):
//! - aarch64 macOS: 15.70 max with 400-value operand stacks (15.92 with 1000-1500 values,
//!   which the 16 KiB macOS guard page still catches; 16 bytes per spilled value is the
//!   limit), locals 8.03, 49 params 14.06, empty frame 7.27; intercepts at or below 0,
//!   within the 16 KiB page rounding.
//! - x86_64 Linux (Docker, linux/amd64): 10.81 max (49 params; operand stack values and
//!   locals 8.00, empty frame 5.82); intercepts 11-15 KiB.

use klever_chain_vm_executor::{CompilationOptions, ExecutorService, Instance, VMHooksDefault};
use klever_chain_vm_executor_wasmer::{
    last_call_stack_depth_stats, override_stack_weight_budget_for_tests, BasicExecutorService,
    FRAME_OVERHEAD_SLOTS, SLOT_BYTES,
};
use wasmer::wat2wasm;

mod common;

/// Native bytes below the first wasm frame: the probe thread's own frames and the host
/// frames of `Instance::call` (debug build). Measured at most 15.2 KiB (x86_64 Linux).
const CALL_OVERHEAD_BYTES: i64 = 20 << 10;

/// Smallest stack the search probes: large enough for the host frames, so the stack
/// always runs out in wasm code, where wasmer turns it into a trap.
const SEARCH_FLOOR: usize = 128 << 10;

/// The stack size search stops at this resolution; thread stacks are also rounded up to
/// whole pages (16 KiB on aarch64 macOS).
const SEARCH_STEP: usize = 4 << 10;

/// Extra frames between the two samples are chosen to span at least this many bytes, so
/// page rounding stays below 0.2% of the per-frame size.
const SAMPLE_SPAN_BYTES: i64 = 8 << 20;

const GAS_LIMIT: u64 = u64::MAX / 2;

fn weight(params: i64, locals: i64, height: i64) -> i64 {
    params + locals + height + FRAME_OVERHEAD_SLOTS
}

struct Shape {
    name: String,
    /// Types of the params after `$n`. The recursive call passes them all on.
    params: Vec<&'static str>,
    locals: Vec<&'static str>,
    /// Instructions run before the recursive call, building the operand stack.
    prefix: String,
    /// Instructions run after it, emptying the operand stack.
    suffix: String,
    /// Height of the operand stack `prefix` leaves.
    prefix_height: i64,
    indirect: bool,
}

impl Shape {
    fn new(name: &str) -> Self {
        Shape {
            name: name.to_string(),
            params: vec![],
            locals: vec![],
            prefix: String::new(),
            suffix: String::new(),
            prefix_height: 0,
            indirect: false,
        }
    }

    /// The highest the operand stack gets in `$rec`, counted by hand from `wat`: the
    /// prefix, then inside the `if` the new `$n` (2 values while it is computed) and the
    /// call's other arguments, plus the table index for `call_indirect`.
    fn height(&self) -> i64 {
        let call_operands = 1 + self.params.len() as i64 + self.indirect as i64;
        self.prefix_height + call_operands.max(2)
    }

    fn rec_weight(&self) -> i64 {
        weight(
            1 + self.params.len() as i64,
            self.locals.len() as i64,
            self.height(),
        )
    }

    /// `$main` pushes `$n` and its own zeroed locals as the other arguments (the metering
    /// middleware rejects float constants), then calls `$rec`.
    fn main_weight(&self) -> i64 {
        let params = self.params.len() as i64;
        weight(0, params, 1 + params)
    }

    /// Weight of `main` calling `$rec` at `depth`: `depth + 1` frames of `$rec`.
    fn call_weight(&self, depth: i64) -> i64 {
        self.main_weight() + (depth + 1) * self.rec_weight()
    }

    fn wat(&self, depth: i64) -> String {
        let params = self.params.join(" ");
        let locals = if self.locals.is_empty() {
            String::new()
        } else {
            format!("(local {})", self.locals.join(" "))
        };
        let args: String = (1..=self.params.len())
            .map(|index| format!("local.get {index}\n"))
            .collect();
        let main_locals = if self.params.is_empty() {
            String::new()
        } else {
            format!("(local {})", params)
        };
        let main_args: String = (0..self.params.len())
            .map(|index| format!("local.get {index}\n"))
            .collect();
        let call = if self.indirect {
            "i32.const 0\ncall_indirect (type $rec_t)"
        } else {
            "call $rec"
        };
        let Shape { prefix, suffix, .. } = self;
        format!(
            r#"
(module
    (type $rec_t (func (param i32 {params})))
    (func $rec (type $rec_t) {locals}
{prefix}
        local.get 0
        if
            local.get 0
            i32.const 1
            i32.sub
{args}
            {call}
        end
{suffix})
    (func $main {main_locals}
        i32.const {depth}
{main_args}
        call $rec)
    (table 1 1 funcref)
    (elem (i32.const 0) $rec)
    (memory 1)
    (export "memory" (memory 0))
    (export "main" (func $main)))
"#
        )
    }
}

fn cycle(types: &[&'static str], count: usize) -> Vec<&'static str> {
    types.iter().copied().cycle().take(count).collect()
}

/// `count` operand stack values read from the locals after `$n`, dropped one by one.
fn operand_shape(name: &str, local_types: &[&'static str], count: usize) -> Shape {
    let mut shape = Shape::new(name);
    shape.locals = local_types.to_vec();
    shape.prefix = (0..count)
        .map(|index| format!("local.get {}\n", 1 + index % local_types.len()))
        .collect();
    shape.suffix = "drop\n".repeat(count);
    shape.prefix_height = count as i64;
    shape
}

fn shapes() -> Vec<Shape> {
    let mut shapes = vec![Shape::new("empty")];

    // Values held on the operand stack across the call: see the module doc for the
    // aarch64 limit.
    let values = if cfg!(target_arch = "aarch64") {
        400usize
    } else {
        1_500
    };

    // The review demo shape: copies of one i64 local, summed after the call.
    let mut counts = vec![values];
    if cfg!(target_arch = "x86_64") {
        counts.extend([10_000, 40_000]);
    }
    for count in counts {
        let mut shape = Shape::new(&format!("operand_i64_add x{count}"));
        shape.locals = vec!["i64"];
        shape.prefix = "local.get 1\n".repeat(count);
        shape.suffix = format!("{}drop", "i64.add\n".repeat(count - 1));
        shape.prefix_height = count as i64;
        shapes.push(shape);
    }
    shapes.push(operand_shape(
        &format!("operand_mixed x{values}"),
        &["i32", "i64", "f32", "f64"],
        values,
    ));
    shapes.push(operand_shape(&format!("operand_f64 x{values}"), &["f64"], values));
    shapes.push(operand_shape(&format!("operand_v128 x{values}"), &["v128"], values));

    for (name, types) in [
        ("locals_i64 x3000", &["i64"][..]),
        ("locals_mixed x3000", &["i32", "i64", "f32", "f64"][..]),
        ("locals_v128 x3000", &["v128"][..]),
    ] {
        let mut shape = Shape::new(name);
        shape.locals = cycle(types, 3_000);
        shapes.push(shape);
    }

    for (name, types, indirect) in [
        // Wasmer accepts at most 50 params, `$n` included.
        ("params_i64 x49", &["i64"][..], false),
        ("params_mixed x49", &["i32", "i64", "f32", "f64"][..], false),
        (
            "params_mixed x49 call_indirect",
            &["i32", "i64", "f32", "f64"][..],
            true,
        ),
    ] {
        let mut shape = Shape::new(name);
        shape.params = cycle(types, 49);
        shape.indirect = indirect;
        shapes.push(shape);
    }

    // Nested blocks with results, one value per level held across the call.
    let levels = values;
    let mut shape = Shape::new(&format!("nested_blocks_result x{levels}"));
    shape.locals = vec!["i64"];
    shape.prefix = "block (result i64)\nlocal.get 1\n".repeat(levels);
    shape.suffix = format!("local.get 1\n{}drop", "i64.add\nend\n".repeat(levels));
    shape.prefix_height = levels as i64;
    shapes.push(shape);

    shapes
}

fn instance_in_record_mode(wat: &str) -> Box<dyn Instance> {
    let _budget = override_stack_weight_budget_for_tests(None);
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
    let service = BasicExecutorService::new();
    let executor = service.new_executor(Box::new(VMHooksDefault)).unwrap();
    let options = CompilationOptions {
        gas_limit: GAS_LIMIT,
        ..common::DUMMY_COMPILATION_OPTIONS
    };
    executor.new_instance(&wasm_bytes, &options).unwrap()
}

/// Holds an instance reference that is only used by one thread at a time: the spawning
/// thread blocks on `join` while the stack-size thread runs the call.
struct SendInstance<'a>(&'a dyn Instance);

// SAFETY: see `SendInstance`; the instance is never used from two threads at once.
unsafe impl Send for SendInstance<'_> {}

fn call_on_stack(instance: &dyn Instance, stack_size: usize) -> (Result<(), String>, i64) {
    let instance = SendInstance(instance);
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(stack_size)
            .spawn_scoped(scope, move || {
                let instance = instance;
                instance.0.set_points_used(0).unwrap();
                let result = instance.0.call("main");
                let weight_max = last_call_stack_depth_stats().unwrap().weight_max;
                (result, weight_max)
            })
            .unwrap()
            .join()
            .unwrap()
    })
}

/// Smallest stack size, to `SEARCH_STEP`, the call succeeds on, or `None` if the call
/// already succeeds just above `SEARCH_FLOOR`.
fn min_stack_size(instance: &dyn Instance, estimate: usize) -> Option<usize> {
    let mut high = estimate.max(SEARCH_FLOOR) * 2;
    while call_on_stack(instance, high).0.is_err() {
        high *= 2;
        assert!(high <= 1 << 30, "the call fails even on a 1 GiB stack");
    }
    let mut low = SEARCH_FLOOR;
    while high - low > SEARCH_STEP {
        // `low` is on the grid but `high` may not be: never round `mid` down to `low`,
        // so every probe shrinks the bracket.
        let mid = ((low + high) / 2 / SEARCH_STEP * SEARCH_STEP).max(low + SEARCH_STEP);
        match call_on_stack(instance, mid).0 {
            Ok(()) => high = mid,
            Err(err) => {
                assert!(
                    !err.contains("unreachable"),
                    "record mode must not trap on the budget: {err}"
                );
                low = mid;
            }
        }
    }
    (high > SEARCH_FLOOR + SEARCH_STEP).then_some(high)
}

struct Sample {
    depth: i64,
    min_stack: i64,
    weight: i64,
}

/// `None` if the call at `depth` is too small to measure above `SEARCH_FLOOR`.
fn measure(shape: &Shape, depth: i64) -> Option<Sample> {
    let instance = instance_in_record_mode(&shape.wat(depth));
    let weight = shape.call_weight(depth);

    // The counters must agree with the hand-counted weight, so the bound below is
    // checked against the weight the middleware really enforces.
    let (result, weight_max) = call_on_stack(instance.as_ref(), 256 << 20);
    assert_eq!(result, Ok(()), "{} at depth {depth}", shape.name);
    assert_eq!(weight_max, weight, "{} at depth {depth}", shape.name);

    let estimate = (weight * SLOT_BYTES * 2) as usize;
    Some(Sample {
        depth,
        min_stack: min_stack_size(instance.as_ref(), estimate)? as i64,
        weight,
    })
}

#[test]
fn slot_bytes_bounds_every_measured_frame() {
    println!(
        "arch={} SLOT_BYTES={SLOT_BYTES} FRAME_OVERHEAD_SLOTS={FRAME_OVERHEAD_SLOTS}",
        std::env::consts::ARCH
    );
    println!(
        "{:<34} {:>7} {:>9} {:>9} {:>11} {:>10}",
        "shape", "weight", "frame_B", "B/slot", "intercept", "worst_over"
    );

    let mut max_bytes_per_slot = 0f64;
    let mut max_over = i64::MIN;
    let mut failures = vec![];
    for shape in shapes() {
        let rec_weight = shape.rec_weight();
        let estimate = rec_weight * SLOT_BYTES;
        // Frames can be smaller than `estimate`: deepen the first sample until it is
        // measurable above the search floor.
        let mut first_depth = ((512 << 10) / estimate).max(1);
        let first = loop {
            match measure(&shape, first_depth) {
                Some(sample) => break sample,
                None => first_depth *= 2,
            }
        };
        let extra_frames = (SAMPLE_SPAN_BYTES / estimate).max(2);
        let samples = [
            first,
            measure(&shape, first_depth + extra_frames).expect("the second sample is larger"),
        ];

        let frame_bytes = (samples[1].min_stack - samples[0].min_stack) as f64
            / (samples[1].depth - samples[0].depth) as f64;
        let bytes_per_slot = frame_bytes / rec_weight as f64;
        // Native bytes below the first `$rec` frame, by extrapolation.
        let intercept = samples[0].min_stack as f64 - (samples[0].depth + 1) as f64 * frame_bytes;
        // How far each sample is from the bound without the fixed call overhead.
        let over = samples
            .iter()
            .map(|sample| sample.min_stack - SLOT_BYTES * sample.weight)
            .max()
            .unwrap();
        println!(
            "{:<34} {:>7} {:>9.1} {:>9.2} {:>11.0} {:>10}",
            shape.name, rec_weight, frame_bytes, bytes_per_slot, intercept, over
        );

        max_bytes_per_slot = max_bytes_per_slot.max(bytes_per_slot);
        max_over = max_over.max(over);

        // One page of rounding is allowed across the extra frames.
        let page = 16 << 10;
        let extra = samples[1].depth - samples[0].depth;
        if samples[1].min_stack - samples[0].min_stack > SLOT_BYTES * rec_weight * extra + page {
            failures.push(format!(
                "{}: {frame_bytes:.1} bytes per frame of weight {rec_weight}",
                shape.name
            ));
        }
        // Every local is zero-initialised in its own 8-byte slot on both architectures.
        if shape.name.starts_with("locals_") && frame_bytes < 7.5 * shape.locals.len() as f64 {
            failures.push(format!(
                "{}: {frame_bytes:.1} bytes for {} locals: thread stacks are larger than \
                 requested (on Linux set GLIBC_TUNABLES=glibc.pthread.stack_cache_size=0)",
                shape.name,
                shape.locals.len()
            ));
        }
        if over > CALL_OVERHEAD_BYTES || intercept > CALL_OVERHEAD_BYTES as f64 {
            failures.push(format!(
                "{}: a call needs {over} bytes above SLOT_BYTES * weight",
                shape.name
            ));
        }
    }
    println!("max bytes per slot: {max_bytes_per_slot:.2}; max bytes above SLOT_BYTES * weight: {max_over}");

    assert!(failures.is_empty(), "{failures:#?}");
}
