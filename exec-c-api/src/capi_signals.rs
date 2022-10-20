use crate::service_singleton::with_service;

#[no_mangle]
pub unsafe extern "C" fn vm_exec_set_sigsegv_passthrough() {
    with_service(|service| {
        service.update_last_error_str("Rust: wasmer_set_sigsegv_passthrough called!".to_string());
    });
}
