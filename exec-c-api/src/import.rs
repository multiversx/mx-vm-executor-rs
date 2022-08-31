//! Create, read, destroy import definitions (function, global, memory
//! and table) on an instance.

use crate::{
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
use elrond_exec_service::{FuncPointer, FuncSig, Function, Type};

pub enum ImportError {
    ModuleNameError,
    ImportNameError,
}

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

pub struct WasmerImportData {
    pub module_name: String,
    pub import_name: String,
    pub import_func: Function,
}

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_import_object_cache_from_imports_never(
    imports: *mut vm_exec_import_t,
    imports_len: c_uint,
) -> vm_exec_result_t {
    vm_exec_result_t::VM_EXEC_OK
}

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_import_object_cache_from_imports(
    // imports: *mut wasmer_import_t,
    imports_len: c_uint,
) -> vm_exec_result_t {
    // let imports_result = wasmer_create_import_object_from_imports(imports, imports_len);
    // let import_object = match imports_result {
    //     Err(ImportError::ModuleNameError) => {
    //         update_last_error(CApiError {
    //             msg: "error converting module name to string".to_string(),
    //         });
    //         return wasmer_result_t::WASMER_ERROR;
    //     }
    //     Err(ImportError::ImportNameError) => {
    //         update_last_error(CApiError {
    //             msg: "error converting import_name to string".to_string(),
    //         });
    //         return wasmer_result_t::WASMER_ERROR;
    //     }
    //     Ok(created_imports_object) => created_imports_object,
    // };

    // if GLOBAL_IMPORT_OBJECT != (0 as *mut ImportObject) {
    //     let _ = Box::from_raw(GLOBAL_IMPORT_OBJECT); // deallocate previous GLOBAL_IMPORT_OBJECT
    // }

    // GLOBAL_IMPORT_OBJECT = Box::into_raw(Box::new(import_object));
    vm_exec_result_t::VM_EXEC_OK
}

/// Assembles an ImportObject from a list of imports received on the C API
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe fn create_import_object_from_imports(
    imports: *mut vm_exec_import_t,
    imports_len: c_uint,
) -> Result<Vec<WasmerImportData>, ImportError> {
    let imports: &[vm_exec_import_t] = slice::from_raw_parts(imports, imports_len as usize);
    // let mut import_object = ImportObject::new();
    // let mut namespaces = HashMap::new();
    let mut result = Vec::with_capacity(imports_len as usize);

    for import in imports {
        let module_name = slice::from_raw_parts(
            import.module_name.bytes,
            import.module_name.bytes_len as usize,
        );
        let module_name = if let Ok(s) = std::str::from_utf8(module_name) {
            s
        } else {
            return Err(ImportError::ModuleNameError);
        };
        let import_name = slice::from_raw_parts(
            import.import_name.bytes,
            import.import_name.bytes_len as usize,
        );
        let import_name = if let Ok(s) = std::str::from_utf8(import_name) {
            s
        } else {
            return Err(ImportError::ImportNameError);
        };

        let func_export = import.import_func as *mut Function;
        let import_func = (&*func_export).clone();

        println!(
            "Rust: {}, {}, {}",
            module_name,
            import_name,
            import_func.signature.params().len()
        );

        result.push(WasmerImportData {
            module_name: module_name.to_string(),
            import_name: import_name.to_string(),
            import_func,
        });

        // let namespace = namespaces.entry(module_name).or_insert_with(Namespace::new);

        // let export = match import.tag {
        //     wasmer_import_export_kind::WASM_MEMORY => {
        //         let mem = import.value.memory as *mut Memory;
        //         Export::Memory((&*mem).clone())
        //     }
        //     wasmer_import_export_kind::WASM_FUNCTION => {
        //         let func_export = import.value.func as *mut Export;
        //         (&*func_export).clone()
        //     }
        //     wasmer_import_export_kind::WASM_GLOBAL => {
        //         let global = import.value.global as *mut Global;
        //         Export::Global((&*global).clone())
        //     }
        //     wasmer_import_export_kind::WASM_TABLE => {
        //         let table = import.value.table as *mut Table;
        //         Export::Table((&*table).clone())
        //     }
        // };
        // namespace.insert(import_name, export);
    }

    // for (module_name, namespace) in namespaces.into_iter() {
    //     import_object.register(module_name, namespace);
    // }

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
