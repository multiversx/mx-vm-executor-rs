use elrond_exec_service::ExecutorLastError;
use elrond_exec_service::ExecutorService;

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

    fn push_execution_info(&mut self, info: &str) {
        self.execution_info.push_str(info);
    }

    fn get_execution_info(&self) -> String {
        self.execution_info.clone()
    }

    fn set_imports(&mut self, _imports: Vec<elrond_exec_service::WasmerImportData>) {}
}

impl ExecutorService for BasicExecutorService {}
