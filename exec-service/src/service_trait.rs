use crate::WasmerImportData;

pub trait ExecutorService: ExecutorLastError {}

pub trait ExecutorLastError {
    fn update_last_error_str(&mut self, err_str: String);

    fn get_last_error_string(&self) -> String;

    fn push_execution_info(&mut self, line: &str);

    fn get_execution_info(&self) -> String;

    fn set_imports(&mut self, imports: Vec<WasmerImportData>);
}
