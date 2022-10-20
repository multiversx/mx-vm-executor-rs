#[derive(Default)]
pub struct WasmerContext {
    pub(crate) execution_info: String,
}

impl WasmerContext {
    pub fn push_execution_info(&mut self, info: &str) {
        self.execution_info.push_str(info);
        self.execution_info.push('\n');
    }
}
