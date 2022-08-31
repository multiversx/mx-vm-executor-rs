pub trait ExecutorService: ExecutorLastError {}

pub trait ExecutorLastError {
    fn update_last_error_str(&mut self, err_str: &str);

    fn get_last_error_string(&self) -> String;
}
