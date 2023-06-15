use multiversx_chain_vm_executor_wasmer::{set_log_level, u64_to_log_level};

use crate::{basic_types::vm_exec_result_t, service_singleton::with_service};

/// Sets the log level.
#[no_mangle]
pub extern "C" fn vm_exec_set_log_level(value: u64) -> vm_exec_result_t {
    let result = u64_to_log_level(value);

    match result {
        Ok(level) => {
            set_log_level(level);
            vm_exec_result_t::VM_EXEC_OK
        }
        Err(message) => {
            with_service(|service| service.update_last_error_str(message.to_string()));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}
