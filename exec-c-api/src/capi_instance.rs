//! Instantiate a module, call functions, and read exports.

use crate::{
    capi_executor::{vm_exec_executor_t, CapiExecutor},
    service_singleton::with_service,
    string_copy, vm_exec_result_t,
};
use elrond_exec_service::{CompilationOptions, Instance};
use libc::{c_char, c_int};
use std::{ffi::CStr, slice};

/// Opaque pointer to a `wasmer_runtime::Instance` value in Rust.
///
/// A `wasmer_runtime::Instance` represents a WebAssembly instance. It
/// is generally generated by the `wasmer_instantiate()` function, or by
/// the `wasmer_module_instantiate()` function for the most common paths.
#[repr(C)]
pub struct vm_exec_instance_t;

#[repr(C)]
pub struct vm_exec_compilation_options_t;

pub struct CapiInstance {
    pub(crate) content: Box<dyn Instance>,
}

/// Creates a new VM executor instance.
///
/// All of the context comes from the provided VM executor.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_new_instance(
    executor_ptr: *mut vm_exec_executor_t,
    instance_ptr_ptr: *mut *mut vm_exec_instance_t,
    wasm_bytes_ptr: *mut u8,
    wasm_bytes_len: u32,
    options_ptr: *const vm_exec_compilation_options_t,
) -> vm_exec_result_t {
    let capi_executor = cast_input_ptr!(executor_ptr, CapiExecutor, "executor ptr is null");

    if wasm_bytes_ptr.is_null() {
        with_service(|service| service.update_last_error_str("wasm bytes ptr is null".to_string()));
        return vm_exec_result_t::VM_EXEC_ERROR;
    }

    let wasm_bytes: &[u8] = slice::from_raw_parts_mut(wasm_bytes_ptr, wasm_bytes_len as usize);
    let compilation_options: &CompilationOptions = &*(options_ptr as *const CompilationOptions);
    let instance_result = capi_executor
        .content
        .new_instance(wasm_bytes, compilation_options);
    match instance_result {
        Ok(instance_box) => {
            let capi_instance = CapiInstance {
                content: instance_box,
            };
            *instance_ptr_ptr = Box::into_raw(Box::new(capi_instance)) as *mut vm_exec_instance_t;
            vm_exec_result_t::VM_EXEC_OK
        }
        Err(message) => {
            with_service(|service| service.update_last_error_str(message.to_string()));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

/// Calls an exported function of a WebAssembly instance by `name`
/// with the provided parameters. The exported function results are
/// stored on the provided `results` pointer.
///
/// This function returns `vm_exec_result_t::WASMER_OK` upon success,
/// `vm_exec_result_t::WASMER_ERROR` otherwise. You can use
/// `wasmer_last_error_message()` to get the generated error message.
///
/// Potential errors are the following:
///
///   * `instance` is a null pointer,
///   * `name` is a null pointer,
///   * `params` is a null pointer.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_call(
    instance_ptr: *mut vm_exec_instance_t,
    func_name_ptr: *const c_char,
) -> vm_exec_result_t {
    let capi_instance = cast_input_ptr!(instance_ptr, CapiInstance, "instance ptr is null");

    // unpack the function name
    if func_name_ptr.is_null() {
        with_service(|service| service.update_last_error_str("name ptr is null".to_string()));
        return vm_exec_result_t::VM_EXEC_ERROR;
    }
    let func_name_c = CStr::from_ptr(func_name_ptr);
    let func_name_r = func_name_c.to_str().unwrap();

    let result = capi_instance.content.call(func_name_r);
    match result {
        Ok(()) => vm_exec_result_t::VM_EXEC_OK,
        Err(message) => {
            with_service(|service| service.update_last_error_str(message));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

/// Checks that all public module functions (SC endpoints) have no arguments or results.
///
/// Still in the works.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_check_signatures(
    _instance: *mut vm_exec_instance_t,
) -> vm_exec_result_t {
    vm_exec_result_t::VM_EXEC_OK
}

/// Checks whether SC has an endpoint with given name.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_has_function(
    instance_ptr: *mut vm_exec_instance_t,
    func_name_ptr: *const c_char,
) -> c_int {
    let capi_instance = cast_input_ptr!(instance_ptr, CapiInstance, "instance ptr is null", -1);

    // unpack the function name
    return_if_ptr_null!(func_name_ptr, "function name ptr is null", -1);
    let func_name_c = CStr::from_ptr(func_name_ptr);
    let func_name_r = func_name_c.to_str().unwrap();

    if capi_instance.content.has_function(func_name_r) {
        1
    } else {
        0
    }
}

/// Required to be able to extract all SC endpoint names. See `vm_exported_function_names`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exported_function_names_length(
    instance_ptr: *mut vm_exec_instance_t,
) -> c_int {
    let capi_instance = cast_input_ptr!(instance_ptr, CapiInstance, "instance ptr is null", 0);

    let func_names = capi_instance.content.get_exported_function_names();
    if func_names.is_empty() {
        0
    } else {
        let len_sum: usize = func_names.iter().map(|func_name| func_name.len()).sum();
        (len_sum + func_names.len()) as c_int
    }
}

/// Returns all SC endpoint names, separated by pipes.
///
/// e.g. `"init|endpoint1|endpoint2"`
///
/// No endpoint order is assumed.
///
/// It is necessary to first call `vm_exported_function_names_length` and pre-allocate a buffer of this length.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exported_function_names(
    instance_ptr: *mut vm_exec_instance_t,
    dest_buffer: *mut c_char,
    dest_buffer_len: c_int,
) -> c_int {
    let capi_instance = cast_input_ptr!(instance_ptr, CapiInstance, "instance ptr is null", 0);

    let func_names = capi_instance.content.get_exported_function_names();
    let concat = func_names.join("|");
    string_copy(concat, dest_buffer, dest_buffer_len)
}

/// Frees memory for the given `vm_exec_instance_t`.
///
/// Check the `wasmer_instantiate()` function to get a complete
/// example.
///
/// If `instance` is a null pointer, this function does nothing.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_destroy(instance: *mut vm_exec_instance_t) {
    if !instance.is_null() {
        // unsafe {
        std::ptr::drop_in_place(instance);
        // }
        // unsafe { Box::from_raw(instance as *mut Instance) };
    }
}
