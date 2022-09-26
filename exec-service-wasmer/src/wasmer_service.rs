use std::cell::RefCell;
use std::rc::Rc;

use elrond_exec_service::CompilationOptions;
use elrond_exec_service::ExecutorError;
use elrond_exec_service::ExecutorLastError;
use elrond_exec_service::ExecutorService;
use elrond_exec_service::ServiceInstance;

use crate::WasmerContext;
use crate::WasmerInstance;

#[derive(Default)]
pub struct BasicExecutorService {
    pub last_error: String,
    pub context_rc: Rc<RefCell<WasmerContext>>,
}

impl BasicExecutorService {
    pub fn new() -> Self {
        Self {
            last_error: String::new(),
            context_rc: Rc::new(RefCell::new(WasmerContext::default())),
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
        let mut context = self.context_rc.borrow_mut();
        context.push_execution_info(info);
    }

    fn get_execution_info(&self) -> String {
        let context = self.context_rc.borrow();
        context.execution_info.clone()
    }

    fn clear_execution_info(&mut self) {
        let mut context = self.context_rc.borrow_mut();
        context.execution_info.clear();
    }

    fn set_imports(&mut self, imports: Vec<elrond_exec_service::WasmerImportData>) {
        let mut context = self.context_rc.borrow_mut();
        context.imports = imports;
    }

    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        _compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn ServiceInstance>, ExecutorError> {
        Ok(Box::new(WasmerInstance::new(
            self.context_rc.clone(),
            wasm_bytes,
        )?))
    }
}
