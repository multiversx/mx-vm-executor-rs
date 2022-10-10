use std::{
    ffi::c_void,
    sync::{Arc, Mutex}, borrow::Borrow,
};

use wasmer::WasmerEnv;

#[derive(Clone)]
pub struct ImportRuntimeContext {
    pub context_ptr: *mut c_void,
}

impl Default for ImportRuntimeContext {
    fn default() -> Self {
        ImportRuntimeContext {
            context_ptr: std::ptr::null_mut(),
        }
    }
}

#[derive(Clone)]
pub struct ImportRuntimeContextRef(pub Arc<Mutex<ImportRuntimeContext>>);

impl ImportRuntimeContextRef {
    pub fn new(context: ImportRuntimeContext) -> Self {
        Self(Arc::new(Mutex::new(context)))
    }

    pub fn get_context_ptr(&self) -> *mut c_void {
        let content = self.0.lock().unwrap();
        content.context_ptr
    }

    pub fn set_context_ptr(&self, context_ptr: *mut c_void) {
        let mut content = self.0.lock().unwrap();
        content.context_ptr = context_ptr;
    }
}

// #[derive(Clone)]
// pub struct ImportRuntimeContextRef(pub *mut ImportRuntimeContext);

unsafe impl Send for ImportRuntimeContext {}
unsafe impl Sync for ImportRuntimeContext {}

unsafe impl Send for ImportRuntimeContextRef {}
unsafe impl Sync for ImportRuntimeContextRef {}

impl WasmerEnv for ImportRuntimeContext {} // TEMP??
impl WasmerEnv for ImportRuntimeContextRef {}
