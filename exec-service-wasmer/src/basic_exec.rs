use std::cell::RefCell;
use std::rc::Rc;

use elrond_exec_service::CompilationOptions;
use elrond_exec_service::ExecutorLastError;
use elrond_exec_service::ExecutorService;
use elrond_exec_service::Instance;

use crate::WasmerInstance;

#[derive(Default)]
pub struct BasicExecutorService {
    pub last_error: String,
    pub execution_info: String,
}

impl BasicExecutorService {
    pub fn new() -> Self {
        Self {
            last_error: String::new(),
            execution_info: String::new(),
        }
    }
}

impl ExecutorLastError for BasicExecutorService {
    fn update_last_error_str(&mut self, err_str: String) {
        self.last_error = err_str;
    }

    fn get_last_error_string(&self) -> String {
        self.last_error.clone()
    }
}

impl ExecutorService for BasicExecutorService {
    fn push_execution_info(&mut self, info: &str) {
        self.execution_info.push_str(info);
        self.execution_info.push('\n');
    }

    fn get_execution_info(&self) -> String {
        self.execution_info.clone()
    }

    fn set_imports(&mut self, _imports: Vec<elrond_exec_service::WasmerImportData>) {}

    fn new_instance(
        &self,
        _bytes: &[u8],
        _compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, String> {
        Ok(Box::new(WasmerInstance {}))
    }

    fn instance_call(&mut self, instance: &dyn Instance, func_name: &str) -> Result<(), String> {
        self.push_execution_info(format!("Instance call! {}", func_name).as_str());
        Ok(())
    }
}
