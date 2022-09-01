//! Create, read, destroy import definitions (function, global, memory
//! and table) on an instance.

use crate::{
    error::CApiError,
    service_singleton::with_service,
    // export::{wasmer_import_export_kind, wasmer_import_export_value},
    value::vm_exec_value_tag,
    vm_exec_byte_array,
    vm_exec_result_t,
};
use libc::c_uint;
// use std::collections::HashMap;
use std::{ffi::c_void, result::Result, slice, sync::Arc};
// use wasmer_runtime::{Global, Memory, Table};
// use wasmer_runtime_core::{
//     export::{Context, Export, FuncPointer},
//     import::{ImportObject, Namespace},
//     types::{FuncSig, Type},
// };
use elrond_exec_service::{FuncPointer, FuncSig, Function, Type, WasmerImportData};

// pub static mut GLOBAL_IMPORT_OBJECT: *mut ImportObject = 0 as *mut ImportObject;

#[repr(C)]
pub struct vm_exec_import_t {
    pub module_name: vm_exec_byte_array,
    pub import_name: vm_exec_byte_array,
    pub import_func: *const vm_exec_import_func_t,
}

#[repr(C)]
#[derive(Clone)]
pub struct vm_exec_import_func_t;

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_set_imports(
    imports: *mut vm_exec_import_t,
    imports_len: c_uint,
) -> vm_exec_result_t {
    match process_raw_imports(imports, imports_len) {
        Ok(converted_imports) => {
            with_service(|service| service.set_imports(converted_imports));
            vm_exec_result_t::VM_EXEC_OK
        }
        Err(capi_error) => {
            with_service(|service| service.update_last_error_str(capi_error.to_string()));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

/// Assembles an ImportObject from a list of imports received on the C API
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe fn process_raw_imports(
    imports: *mut vm_exec_import_t,
    imports_len: c_uint,
) -> Result<Vec<WasmerImportData>, CApiError> {
    let imports: &[vm_exec_import_t] = slice::from_raw_parts(imports, imports_len as usize);
    let mut result = Vec::with_capacity(imports_len as usize);

    for import in imports {
        let module_name = slice::from_raw_parts(
            import.module_name.bytes,
            import.module_name.bytes_len as usize,
        );
        let module_name = std::str::from_utf8(module_name)
            .map_err(|_| CApiError::new("error converting module name to string"))?;

        let import_name = slice::from_raw_parts(
            import.import_name.bytes,
            import.import_name.bytes_len as usize,
        );
        let import_name = std::str::from_utf8(import_name)
            .map_err(|_| CApiError::new("error converting import name to string"))?;

        let func_export = import.import_func as *mut Function;
        let import_func = (&*func_export).clone();

        // println!(
        //     "Rust: {}, {}, {}",
        //     module_name,
        //     import_name,
        //     import_func.signature.params().len()
        // );

        result.push(WasmerImportData {
            module_name: module_name.to_string(),
            import_name: import_name.to_string(),
            import_func,
        });
    }

    Ok(result)
}

/// Creates new host function, aka imported function. `func` is a
/// function pointer, where the first argument is the famous `vm::Ctx`
/// (in Rust), or `wasmer_instance_context_t` (in C). All arguments
/// must be typed with compatible WebAssembly native types:
///
/// | WebAssembly type | C/C++ type |
/// | ---------------- | ---------- |
/// | `i32`            | `int32_t`  |
/// | `i64`            | `int64_t`  |
/// | `f32`            | `float`    |
/// | `f64`            | `double`   |
///
/// The function pointer must have a lifetime greater than the
/// WebAssembly instance lifetime.
///
/// The caller owns the object and should call
/// `wasmer_import_func_destroy` to free it.
#[no_mangle]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe extern "C" fn vm_exec_import_func_new(
    func: extern "C" fn(data: *mut c_void),
    params: *const vm_exec_value_tag,
    params_len: c_uint,
    returns: *const vm_exec_value_tag,
    returns_len: c_uint,
) -> *mut vm_exec_import_func_t {
    let params: &[vm_exec_value_tag] = slice::from_raw_parts(params, params_len as usize);
    let params: Vec<Type> = params.iter().cloned().map(|x| x.into()).collect();
    let returns: &[vm_exec_value_tag] = slice::from_raw_parts(returns, returns_len as usize);
    let returns: Vec<Type> = returns.iter().cloned().map(|x| x.into()).collect();

    let export = Box::new(Function {
        func: FuncPointer::new(func as _),
        signature: Arc::new(FuncSig::new(params, returns)),
    });
    Box::into_raw(export) as *mut vm_exec_import_func_t
}

/// Frees memory for the given Func
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn vm_exec_import_func_destroy(func: *mut vm_exec_import_func_t) {
    if !func.is_null() {
        unsafe { Box::from_raw(func as *mut Function) };
    }
}
