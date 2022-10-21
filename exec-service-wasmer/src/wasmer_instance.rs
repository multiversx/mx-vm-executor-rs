use std::rc::Rc;

use elrond_exec_service::Instance;

use wasmer::Extern;

use crate::WasmerExecutorData;

pub struct WasmerInstance {
    pub executor_data: Rc<WasmerExecutorData>,
    pub wasmer_instance: wasmer::Instance,
}

impl Instance for WasmerInstance {
    fn call(&self, func_name: &str) -> Result<(), String> {
        println!("Rust instance call: {}", func_name);

        let func = self
            .wasmer_instance
            .exports
            .get_function(func_name)
            .map_err(|_| "function not found".to_string())?;

        let _ = func.call(&[]);

        Ok(())
    }

    fn check_signatures(&self) -> bool {
        true
    }

    fn has_function(&self, func_name: &str) -> bool {
        self.wasmer_instance.exports.get_function(func_name).is_ok()
    }

    fn get_exported_function_names(&self) -> Vec<String> {
        self.wasmer_instance
            .exports
            .iter()
            .filter_map(|(name, export)| match export {
                Extern::Function(_) => Some(name),
                _ => None,
            })
            .cloned()
            .collect()
    }
}
