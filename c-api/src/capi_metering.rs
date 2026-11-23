use crate::capi_executor::{vm_exec_executor_t, CapiExecutor};
use crate::capi_instance::{vm_exec_instance_t, CapiInstance};
use crate::service_singleton::with_service;
use crate::vm_exec_result_t;
use klever_chain_vm_executor::OpcodeCost;

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct vm_exec_opcode_cost_t;

/// Sets the opcode costs for the given executor.
///
/// This function returns `vm_exec_result_t::WASMER_OK` upon success,
/// `vm_exec_result_t::WASMER_ERROR` otherwise. You can use
/// `wasmer_last_error_message()` to get the generated error message.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_set_opcode_costs(
    executor_ptr: *mut vm_exec_executor_t,
    opcode_cost_ptr: *const vm_exec_opcode_cost_t,
) -> vm_exec_result_t {
    let capi_executor = cast_input_ptr!(executor_ptr, CapiExecutor, "executor ptr is null");
    let opcode_costs: &OpcodeCost = &*(opcode_cost_ptr as *const OpcodeCost);

    let result = capi_executor.content.set_opcode_cost(opcode_costs);
    match result {
        Ok(()) => vm_exec_result_t::VM_EXEC_OK,
        Err(message) => {
            with_service(|service| service.update_last_error_str(message.to_string()));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

/// Returns the size in bytes of the opcode cost struct that
/// `vm_exec_set_opcode_costs` reads.
///
/// klever-go compares it with the size of its own opcode cost struct at
/// startup: the struct is passed by pointer, so a field missing on either side
/// would shift every later cost without any error.
#[no_mangle]
pub extern "C" fn vm_exec_opcode_cost_size() -> u32 {
    std::mem::size_of::<OpcodeCost>() as u32
}

/// Sets the number of points(gas) limit for the given instance.
///
/// This function returns `vm_exec_result_t::WASMER_OK` upon success,
/// `vm_exec_result_t::WASMER_ERROR` otherwise. You can use
/// `wasmer_last_error_message()` to get the generated error message.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_set_points_limit(
    instance_ptr: *const vm_exec_instance_t,
    limit: u64,
) -> vm_exec_result_t {
    let capi_instance = cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    let result = capi_instance.content.set_points_limit(limit);
    match result {
        Ok(()) => vm_exec_result_t::VM_EXEC_OK,
        Err(message) => {
            with_service(|service| service.update_last_error_str(message));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

/// Sets the number of points(gas) for the given instance.
///
/// This function returns `vm_exec_result_t::WASMER_OK` upon success,
/// `vm_exec_result_t::WASMER_ERROR` otherwise. You can use
/// `wasmer_last_error_message()` to get the generated error message.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_set_points_used(
    instance_ptr: *const vm_exec_instance_t,
    points: u64,
) -> vm_exec_result_t {
    let capi_instance = cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    let result = capi_instance.content.set_points_used(points);
    match result {
        Ok(()) => vm_exec_result_t::VM_EXEC_OK,
        Err(message) => {
            with_service(|service| service.update_last_error_str(message));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

/// Returns the number of points(gas) used by the given instance.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_get_points_used(
    instance_ptr: *const vm_exec_instance_t,
) -> u64 {
    let capi_instance =
        cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null", 0);
    let result = capi_instance.content.get_points_used();
    match result {
        Ok(points) => points,
        Err(message) => {
            with_service(|service| service.update_last_error_str(message));
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fields in `OpcodeCost`, all `u32`. klever-go's opcode cost struct must
    /// have the same fields in the same order.
    const OPCODE_COST_FIELD_COUNT: usize = 123;

    #[test]
    fn opcode_cost_size_is_the_struct_size() {
        assert_eq!(
            vm_exec_opcode_cost_size() as usize,
            std::mem::size_of::<OpcodeCost>()
        );
    }

    #[test]
    fn opcode_cost_has_the_expected_field_count() {
        assert_eq!(
            std::mem::size_of::<OpcodeCost>(),
            OPCODE_COST_FIELD_COUNT * std::mem::size_of::<u32>()
        );
        println!("vm_exec_opcode_cost_size() = {}", vm_exec_opcode_cost_size());
    }
}
