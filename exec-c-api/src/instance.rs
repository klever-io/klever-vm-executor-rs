//! Instantiate a module, call functions, and read exports.

use crate::{
    service_singleton::with_service,
    string_copy,
    string_length,
    vm_exec_byte_array,
    vm_exec_byte_array_list,
    // error::{update_last_error, CApiError},
    // export::{wasmer_exports_t, NamedExport, NamedExports},
    // import::GLOBAL_IMPORT_OBJECT,
    // memory::wasmer_memory_t,
    // value::{wasmer_value, wasmer_value_t},
    vm_exec_result_t,
};
use elrond_exec_service::{CompilationOptions, ServiceInstance};
use libc::{c_char, c_int, c_void};
use std::{ffi::CStr, ptr, slice};
// use wasmer_runtime::{Ctx, Instance, Memory, Value};
// use wasmer_runtime_core::import::ImportObject;

// use wasmer_middleware_common::metering;
// use wasmer_middleware_common::opcode_control;
// use wasmer_middleware_common::opcode_trace;
// use wasmer_middleware_common::runtime_breakpoints;
// use wasmer_runtime_core::backend::Compiler;
// use wasmer_runtime_core::codegen::{MiddlewareChain, StreamingCompiler};

/// Opaque pointer to a `wasmer_runtime::Instance` value in Rust.
///
/// A `wasmer_runtime::Instance` represents a WebAssembly instance. It
/// is generally generated by the `wasmer_instantiate()` function, or by
/// the `wasmer_module_instantiate()` function for the most common paths.
#[repr(C)]
pub struct vm_exec_instance_t;

/// Opaque pointer to a `wasmer_runtime::Ctx` value in Rust.
///
/// An instance context is passed to any host function (aka imported
/// function) as the first argument. It is necessary to read the
/// instance data or the memory, respectively with the
/// `wasmer_instance_context_data_get()` function, and the
/// `wasmer_instance_context_memory()` function.
///
/// It is also possible to get the instance context outside a host
/// function by using the `wasmer_instance_context_get()`
/// function. See also `wasmer_instance_context_data_set()` to set the
/// instance context data.
///
/// Example:
///
/// ```c
/// // A host function that prints data from the WebAssembly memory to
/// // the standard output.
/// void print(wasmer_instance_context_t *context, int32_t pointer, int32_t length) {
///     // Use `wasmer_instance_context` to get back the first instance memory.
///     const wasmer_memory_t *memory = wasmer_instance_context_memory(context, 0);
///
///     // Continue…
/// }
/// ```
// #[repr(C)]
// pub struct wasmer_instance_context_t;

#[repr(C)]
pub struct vm_exec_compilation_options_t;

pub struct CapiInstance {
    content: Box<dyn ServiceInstance>,
}

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_new_instance(
    instance: *mut *mut vm_exec_instance_t,
    wasm_bytes_ptr: *mut u8,
    wasm_bytes_len: u32,
    options_ptr: *const vm_exec_compilation_options_t,
) -> vm_exec_result_t {
    if wasm_bytes_ptr.is_null() {
        with_service(|service| service.update_last_error_str("wasm bytes ptr is null".to_string()));
        return vm_exec_result_t::VM_EXEC_ERROR;
    }

    let wasm_bytes: &[u8] = slice::from_raw_parts_mut(wasm_bytes_ptr, wasm_bytes_len as usize);
    let compilation_options: &CompilationOptions = &*(options_ptr as *const CompilationOptions);
    let instance_result =
        with_service(|service| service.new_instance(wasm_bytes, compilation_options));
    match instance_result {
        Ok(instance_box) => {
            let capi_instance = CapiInstance {
                content: instance_box,
            };
            *instance = Box::into_raw(Box::new(capi_instance)) as *mut vm_exec_instance_t;
            vm_exec_result_t::VM_EXEC_OK
        }
        Err(message) => {
            with_service(|service| service.update_last_error_str(message.to_string()));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

// /// Returns the instance context. Learn more by looking at the
// /// `wasmer_instance_context_t` struct.
// ///
// /// This function returns `null` if `instance` is a null pointer.
// ///
// /// Example:
// ///
// /// ```c
// /// const wasmer_instance_context_get *context = wasmer_instance_context_get(instance);
// /// my_data *data = (my_data *) wasmer_instance_context_data_get(context);
// /// // Do something with `my_data`.
// /// ```
// ///
// /// It is often useful with `wasmer_instance_context_data_set()`.
// #[allow(clippy::cast_ptr_alignment)]
// #[no_mangle]
// pub extern "C" fn wasmer_instance_context_get(
//     instance: *mut vm_exec_instance_t,
// ) -> *const wasmer_instance_context_t {
//     if instance.is_null() {
//         return ptr::null() as _;
//     }

//     let instance = unsafe { &*(instance as *const Instance) };
//     let context: *const Ctx = instance.context() as *const _;

//     context as *const wasmer_instance_context_t
// }

// /// Verifies whether the specified function name is imported by the given instance.
// #[allow(clippy::cast_ptr_alignment)]
// #[no_mangle]
// pub unsafe extern "C" fn wasmer_instance_is_function_imported(
//     instance: *mut vm_exec_instance_t,
//     name: *const c_char,
// ) -> bool {
//     if instance.is_null() {
//         return false;
//     }

//     if name.is_null() {
//         return false;
//     }

//     let instance = &*(instance as *const Instance);

//     let func_name_c = CStr::from_ptr(name);
//     let func_name_r = func_name_c.to_str().unwrap();

//     let module = instance.module();

//     let functions = module.info().name_table.to_vec();

//     functions.contains(&func_name_r)
// }

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
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_call(
    instance: *mut vm_exec_instance_t,
    func_name_ptr: *const c_char,
) -> vm_exec_result_t {
    // unpack the instance object
    if instance.is_null() {
        with_service(|service| service.update_last_error_str("instance ptr is null".to_string()));
        return vm_exec_result_t::VM_EXEC_ERROR;
    }
    let capi_instance = &mut *(instance as *mut CapiInstance);

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

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_check_signatures(
    instance: *mut vm_exec_instance_t,
) -> vm_exec_result_t {
    vm_exec_result_t::VM_EXEC_OK
}

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_has_function(
    instance: *mut vm_exec_instance_t,
    func_name_ptr: *const c_char,
) -> c_int {
    // unpack the instance object
    if instance.is_null() {
        with_service(|service| service.update_last_error_str("instance ptr is null".to_string()));
        return -1;
    }
    let capi_instance = &mut *(instance as *mut CapiInstance);

    // unpack the function name
    if func_name_ptr.is_null() {
        with_service(|service| service.update_last_error_str("name ptr is null".to_string()));
        return -1;
    }
    let func_name_c = CStr::from_ptr(func_name_ptr);
    let func_name_r = func_name_c.to_str().unwrap();

    if capi_instance.content.has_function(func_name_r) {
        1
    } else {
        0
    }
}

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exported_function_names_length(
    instance: *mut vm_exec_instance_t,
) -> c_int {
    // unpack the instance object
    if instance.is_null() {
        with_service(|service| service.update_last_error_str("instance ptr is null".to_string()));
        return 0;
    }
    let capi_instance = &mut *(instance as *mut CapiInstance);

    let func_names = capi_instance.content.get_exported_function_names();
    if func_names.is_empty() {
        0
    } else {
        let len_sum: usize = func_names.iter().map(|func_name| func_name.len()).sum();
        (len_sum + func_names.len()) as c_int
    }
}

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exported_function_names(
    instance: *mut vm_exec_instance_t,
    dest_buffer: *mut c_char,
    dest_buffer_len: c_int,
) -> c_int {
    // unpack the instance object
    if instance.is_null() {
        with_service(|service| service.update_last_error_str("instance ptr is null".to_string()));
        return 0;
    }
    let capi_instance = &mut *(instance as *mut CapiInstance);

    let func_names = capi_instance.content.get_exported_function_names();
    let concat = func_names.join("|");
    string_copy(concat, dest_buffer, dest_buffer_len)
}

// /// Gets all the exports of the given WebAssembly instance.
// ///

// /// This function stores a Rust vector of exports into `exports` as an
// /// opaque pointer of kind `wasmer_exports_t`.
// ///
// /// As is, you can do anything with `exports` except using the
// /// companion functions, like `wasmer_exports_len()`,
// /// `wasmer_exports_get()` or `wasmer_export_kind()`. See the example below.
// ///
// /// **Warning**: The caller owns the object and should call
// /// `wasmer_exports_destroy()` to free it.
// ///
// /// Example:
// ///
// /// ```c
// /// // Get the exports.
// /// wasmer_exports_t *exports = NULL;
// /// wasmer_instance_exports(instance, &exports);
// ///
// /// // Get the number of exports.
// /// int exports_length = wasmer_exports_len(exports);
// /// printf("Number of exports: %d\n", exports_length);
// ///
// /// // Read the first export.
// /// wasmer_export_t *export = wasmer_exports_get(exports, 0);
// ///
// /// // Get the kind of the export.
// /// wasmer_import_export_kind export_kind = wasmer_export_kind(export);
// ///
// /// // Assert it is a function (why not).
// /// assert(export_kind == WASM_FUNCTION);
// ///
// /// // Read the export name.
// /// wasmer_byte_array name_bytes = wasmer_export_name(export);
// ///
// /// assert(name_bytes.bytes_len == sizeof("sum") - 1);
// /// assert(memcmp(name_bytes.bytes, "sum", sizeof("sum") - 1) == 0);
// ///
// /// // Destroy the exports.
// /// wasmer_exports_destroy(exports);
// /// ```
// // #[allow(clippy::cast_ptr_alignment)]
// // #[no_mangle]
// // pub unsafe extern "C" fn wasmer_instance_exports(
// //     instance: *mut vm_exec_instance_t,
// //     exports: *mut *mut wasmer_exports_t,
// // ) {
// //     if instance.is_null() {
// //         return;
// //     }

// //     let instance_ref = &mut *(instance as *mut Instance);
// //     let mut exports_vec: Vec<NamedExport> = Vec::with_capacity(instance_ref.exports().count());

// //     for (name, export) in instance_ref.exports() {
// //         exports_vec.push(NamedExport {
// //             name: name.clone(),
// //             export: export.clone(),
// //             instance: instance as *mut Instance,
// //         });
// //     }

// //     let named_exports: Box<NamedExports> = Box::new(NamedExports(exports_vec));

// //     *exports = Box::into_raw(named_exports) as *mut wasmer_exports_t;
// // }

// /// Sets the data that can be hold by an instance context.
// ///
// /// An instance context (represented by the opaque
// /// `wasmer_instance_context_t` structure) can hold user-defined
// /// data. This function sets the data. This function is complementary
// /// of `wasmer_instance_context_data_get()`.
// ///
// /// This function does nothing if `instance` is a null pointer.
// ///
// /// Example:
// ///
// /// ```c
// /// // Define your own data.
// /// typedef struct {
// ///     // …
// /// } my_data;
// ///
// /// // Allocate them and set them on the given instance.
// /// my_data *data = malloc(sizeof(my_data));
// /// data->… = …;
// /// wasmer_instance_context_data_set(instance, (void*) my_data);
// ///
// /// // You can read your data.
// /// {
// ///     my_data *data = (my_data*) wasmer_instance_context_data_get(wasmer_instance_context_get(instance));
// ///     // …
// /// }
// /// ```
// #[allow(clippy::cast_ptr_alignment)]
// #[no_mangle]
// pub extern "C" fn wasmer_instance_context_data_set(
//     instance: *mut vm_exec_instance_t,
//     data_ptr: *mut c_void,
// ) {
//     if instance.is_null() {
//         return;
//     }

//     let instance = unsafe { &mut *(instance as *mut Instance) };

//     instance.context_mut().data = data_ptr;
// }

// /// Gets the `memory_idx`th memory of the instance.
// ///
// /// Note that the index is always `0` until multiple memories are supported.
// ///
// /// This function is mostly used inside host functions (aka imported
// /// functions) to read the instance memory.
// ///
// /// Example of a _host function_ that reads and prints a string based on a pointer and a length:
// ///
// /// ```c
// /// void print_string(const wasmer_instance_context_t *context, int32_t pointer, int32_t length) {
// ///     // Get the 0th memory.
// ///     const wasmer_memory_t *memory = wasmer_instance_context_memory(context, 0);
// ///
// ///     // Get the memory data as a pointer.
// ///     uint8_t *memory_bytes = wasmer_memory_data(memory);
// ///
// ///     // Print what we assumed to be a string!
// ///     printf("%.*s", length, memory_bytes + pointer);
// /// }
// /// ```
// // #[allow(clippy::cast_ptr_alignment)]
// // #[no_mangle]
// // pub extern "C" fn wasmer_instance_context_memory(
// //     ctx: *const wasmer_instance_context_t,
// //     _memory_idx: u32,
// // ) -> *const wasmer_memory_t {
// //     let ctx = unsafe { &*(ctx as *const Ctx) };
// //     let memory = ctx.memory(0);
// //     memory as *const Memory as *const wasmer_memory_t
// // }

// /// Gets the data that can be hold by an instance.
// ///
// /// This function is complementary of
// /// `wasmer_instance_context_data_set()`. Please read its
// /// documentation. You can also read the documentation of
// /// `wasmer_instance_context_t` to get other examples.
// ///
// /// This function returns nothing if `ctx` is a null pointer.
// #[allow(clippy::cast_ptr_alignment)]
// #[no_mangle]
// pub extern "C" fn wasmer_instance_context_data_get(
//     ctx: *const wasmer_instance_context_t,
// ) -> *mut c_void {
//     if ctx.is_null() {
//         return ptr::null_mut() as _;
//     }

//     let ctx = unsafe { &*(ctx as *const Ctx) };

//     ctx.data
// }

/// Frees memory for the given `vm_exec_instance_t`.
///
/// Check the `wasmer_instantiate()` function to get a complete
/// example.
///
/// If `instance` is a null pointer, this function does nothing.
///
/// Example:
///
/// ```c
/// // Get an instance.
/// vm_exec_instance_t *instance = NULL;
/// wasmer_instantiate(&instance, bytes, bytes_length, imports, 0);
///
/// // Destroy the instance.
/// wasmer_instance_destroy(instance);
/// ```
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn vm_exec_instance_destroy(instance: *mut vm_exec_instance_t) {
    if !instance.is_null() {
        unsafe {
            std::ptr::drop_in_place(instance);
        }
        // unsafe { Box::from_raw(instance as *mut Instance) };
    }
}
