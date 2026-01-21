use std::sync::Once;

use crate::{set_log_level, u64_to_log_level};
use meta::capi_safe_unwind;

use crate::{basic_types::vm_exec_result_t, service_singleton::with_service};

static PANIC_HANDLER: Once = Once::new();

pub fn set_panic_handler() {
    // Initialize the panic handler only once
    PANIC_HANDLER.call_once(|| {
        std::panic::set_hook(Box::new(|_| {}));
    });
}

/// Sets the log level.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_result_t::VM_EXEC_ERROR)]
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
