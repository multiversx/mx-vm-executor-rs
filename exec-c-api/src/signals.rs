// use env_logger::{Builder, Target};
// use libc::c_uint;
// use std::env;

use crate::service_singleton::with_service;

#[no_mangle]
pub unsafe extern "C" fn vm_exec_set_sigsegv_passthrough() {
    with_service(|service| {
        service.update_last_error_str("Rust: wasmer_set_sigsegv_passthrough called!".to_string());
    });
    // log::error!("Rust: wasmer_set_sigsegv_passthrough");
}

// // #[allow(clippy::cast_ptr_alignment)]
// #[no_mangle]
// pub unsafe extern "C" fn do_something(x: c_uint) -> c_uint {
//     panic!("Rust: wasmer_set_sigsegv_passthrough");
//     // with_service(|service| {
//     //     service.update_last_error_str("Rust: wasmer_set_sigsegv_passthrough called!")
//     // });

//     x + 10
// }
