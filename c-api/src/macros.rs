macro_rules! return_if_ptr_null {
    ($ptr_var:ident, $err_msg:expr, $err_return_val:expr) => {
        if $ptr_var.is_null() {
            with_service(|service| service.update_last_error_str($err_msg.to_string()));
            return $err_return_val;
        }
    };
    ($ptr_var:ident, $err_msg:expr) => {
        return_if_ptr_null!($ptr_var, $err_msg, vm_exec_result_t::VM_EXEC_ERROR)
    };
}

macro_rules! cast_input_ptr {
    ($ptr_var:ident, $expected_ty:ty, $err_msg:expr, $err_return_val:expr) => {
        if $ptr_var.is_null() {
            with_service(|service| service.update_last_error_str($err_msg.to_string()));
            return $err_return_val;
        } else {
            &mut *($ptr_var as *mut $expected_ty)
        }
    };
    ($ptr_var:ident, $expected_ty:ty, $err_msg:expr) => {
        cast_input_ptr!(
            $ptr_var,
            $expected_ty,
            $err_msg,
            vm_exec_result_t::VM_EXEC_ERROR
        )
    };
}

macro_rules! cast_input_const_ptr {
    ($ptr_var:ident, $expected_ty:ty, $err_msg:expr, $err_return_val:expr) => {
        if $ptr_var.is_null() {
            with_service(|service| service.update_last_error_str($err_msg.to_string()));
            return $err_return_val;
        } else {
            &*($ptr_var as *const $expected_ty)
        }
    };
    ($ptr_var:ident, $expected_ty:ty, $err_msg:expr) => {
        cast_input_const_ptr!(
            $ptr_var,
            $expected_ty,
            $err_msg,
            vm_exec_result_t::VM_EXEC_ERROR
        )
    };
}
