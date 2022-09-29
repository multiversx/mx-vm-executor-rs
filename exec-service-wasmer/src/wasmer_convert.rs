use elrond_exec_service::WasmerImportData;
use wasmer::{Exports, Function, ImportObject, Store};

// fn import_func_wrapper()

pub fn convert_imports(store: &Store, service_imports: &[WasmerImportData]) -> ImportObject {
    let mut namespace = Exports::new();

    for service_import in service_imports {
        namespace.insert(
            service_import.import_name.clone(),
            Function::new(
                store,
                convert_signature(service_import.import_func.signature.as_ref()),
                // service_import.import_func.func,
                |_args| Ok(vec![]), // TODO
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
