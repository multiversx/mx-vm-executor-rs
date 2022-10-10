use std::ffi::c_void;

use elrond_exec_service::{Type, WasmerImportData};
use wasmer::{Exports, Function, ImportObject, RuntimeError, Store, Value};

use crate::wasmer_env::{ImportRuntimeContext, ImportRuntimeContextRef};

pub fn convert_imports(
    store: &Store,
    env: ImportRuntimeContextRef,
    service_imports: &[WasmerImportData],
) -> ImportObject {
    let mut namespace = Exports::new();

    for service_import in service_imports {
        let import_name = service_import.import_name.clone();
        let func_pointer = service_import.import_func.func.clone();
        let no_returns = service_import.import_func.signature.returns().is_empty();
        let params = service_import.import_func.signature.params().to_vec();
        let args_len = params.len();
        namespace.insert(
            import_name.clone(),
            Function::new_with_env(
                store,
                convert_signature(service_import.import_func.signature.as_ref()),
                env.clone(),
                move |env, args| {
                    println!("import called: {}, num args: {}", &import_name, args_len);
                    let func_ptr = func_pointer.inner();
                    if no_returns {
                        match args_len {
                            0 => {
                                let func_fn: fn(*mut c_void) =
                                    unsafe { std::mem::transmute(func_ptr) };
                                func_fn(env.get_context_ptr());
                                Ok(vec![])
                            }
                            1 => {
                                let func_fn: fn(*mut c_void, i32) =
                                    unsafe { std::mem::transmute(func_ptr) };
                                func_fn(env.get_context_ptr(), args[0].unwrap_i32());
                                Ok(vec![])
                            }
                            2 => {
                                match params[1] {
                                    Type::I32 => {
                                        let func_fn: fn(*mut c_void, i32, i32) =
                                            unsafe { std::mem::transmute(func_ptr) };
                                        func_fn(
                                            env.get_context_ptr(),
                                            args[0].unwrap_i32(),
                                            args[1].unwrap_i32(),
                                        );
                                    }
                                    Type::I64 => {
                                        let func_fn: fn(*mut c_void, i32, i64) =
                                            unsafe { std::mem::transmute(func_ptr) };
                                        func_fn(
                                            env.get_context_ptr(),
                                            args[0].unwrap_i32(),
                                            args[1].unwrap_i64(),
                                        );
                                    }
                                }
                                Ok(vec![])
                            }
                            3 => {
                                let func_fn: fn(*mut c_void, i32, i32, i32) =
                                    unsafe { std::mem::transmute(func_ptr) };
                                    func_fn(
                                        env.get_context_ptr(),
                                        args[0].unwrap_i32(),
                                        args[1].unwrap_i32(),
                                        args[2].unwrap_i32(),
                                    );
                                Ok(vec![])
                            }
                            _ => {
                                println!("unsupported import arity (no results)");
                                Ok(vec![])
                                // Err(RuntimeError::new("unsupported import arity")),
                            }
                        }
                    } else {
                        match args_len {
                            0 => {
                                let func_fn: fn(*mut c_void) -> i32 =
                                    unsafe { std::mem::transmute(func_ptr) };
                                let result = func_fn(env.get_context_ptr());
                                Ok(vec![Value::I32(result)])
                            }
                            1 => {
                                let func_fn: fn(*mut c_void, i32) -> i32 =
                                    unsafe { std::mem::transmute(func_ptr) };
                                let result = func_fn(env.get_context_ptr(), args[0].unwrap_i32());
                                Ok(vec![Value::I32(result)])
                            }
                            2 => {
                                let func_fn: fn(*mut c_void, i32, i32) -> i32 =
                                    unsafe { std::mem::transmute(func_ptr) };
                                let result = func_fn(
                                    env.get_context_ptr(),
                                    args[0].unwrap_i32(),
                                    args[1].unwrap_i32(),
                                );
                                Ok(vec![Value::I32(result)])
                            }
                            3 => {
                                let func_fn: fn(*mut c_void, i32, i32, i32) -> i32 =
                                    unsafe { std::mem::transmute(func_ptr) };
                                let result = func_fn(
                                    env.get_context_ptr(),
                                    args[0].unwrap_i32(),
                                    args[1].unwrap_i32(),
                                    args[2].unwrap_i32(),
                                );
                                Ok(vec![Value::I32(result)])
                            }
                            _ => {
                                println!("unsupported import arity (with result)");
                                Ok(vec![])
                                // Err(RuntimeError::new("unsupported import arity")),
                            }
                        }
                    }
                }, // TODO
            ),
        )
    }

    let mut import_object = ImportObject::new();
    import_object.register("env", namespace);
    import_object
}

pub fn convert_type(service_ty: &elrond_exec_service::Type) -> wasmer::Type {
    match service_ty {
        elrond_exec_service::Type::I32 => wasmer::Type::I32,
        elrond_exec_service::Type::I64 => wasmer::Type::I64,
    }
}

pub fn convert_signature(service_sig: &elrond_exec_service::FuncSig) -> wasmer::FunctionType {
    let params: Vec<wasmer::Type> = service_sig.params().iter().map(convert_type).collect();
    let returns: Vec<wasmer::Type> = service_sig.returns().iter().map(convert_type).collect();
    wasmer::FunctionType::new(params, returns)
}
