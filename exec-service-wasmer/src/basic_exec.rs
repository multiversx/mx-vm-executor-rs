use elrond_exec_service::ExecutorLastError;
use elrond_exec_service::ExecutorService;

#[derive(Default)]
pub struct BasicExecutorService {
    pub last_error: String,
}

impl BasicExecutorService {
    pub fn new() -> Self {
        Self {
            last_error: "INITIAL ERROR".to_string(),
        }
    }
}

impl ExecutorLastError for BasicExecutorService {
    fn update_last_error_str(&mut self, err_str: &str) {
        self.last_error = err_str.to_string();
    }

    fn get_last_error_string(&self) -> String {
        self.last_error.clone()
    }
}

impl ExecutorService for BasicExecutorService {}
