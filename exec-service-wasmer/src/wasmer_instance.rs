use std::{cell::RefCell, rc::Rc};

use elrond_exec_service::{ExecutorService, Instance};

use crate::BasicExecutorService;

pub struct WasmerInstance {
    // pub(crate) service_ref: Rc<RefCell<Box<dyn ExecutorService>>>,
}

impl Instance for WasmerInstance {
    fn call(&self, func_name: &str) -> Result<(), String> {
        Ok(())
    }
}
