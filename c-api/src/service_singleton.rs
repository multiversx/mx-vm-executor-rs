use multiversx_chain_vm_executor::ExecutorService;
use multiversx_chain_vm_executor_wasmer::BasicExecutorService;
use std::cell::RefCell;

thread_local! {
    static SERVICE: RefCell<Box<dyn ExecutorService>> = RefCell::new(Box::new(BasicExecutorService::new()));
}

pub fn with_service<R, F: FnOnce(&mut dyn ExecutorService) -> R>(f: F) -> R {
    SERVICE.with(|service| f(&mut **service.borrow_mut()))
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_singleton_last_error() {
        const SAMPLE_ERROR: &str = "sample error";
        with_service(|service| service.update_last_error_str(SAMPLE_ERROR.to_string()));
        let last_error = with_service(|service| service.get_last_error_string());
        assert_eq!(last_error, SAMPLE_ERROR);
    }
}
