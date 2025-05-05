use std::{error::Error, fmt};

use crate::ExecutorError;

/// Useful for validating wasm bytes coming from tests.
///
/// Allows executors to provide an adequate error.
pub fn check_missing_wasm(wasm_bytes: &[u8]) -> Result<(), ExecutorError> {
    if wasm_bytes.starts_with("MISSING:".as_bytes()) {
        Err(Box::new(MissingWasmError(
            String::from_utf8_lossy(wasm_bytes).to_string(),
        )))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub struct MissingWasmError(String);

impl fmt::Display for MissingWasmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Contract {}", &self.0)
    }
}

impl Error for MissingWasmError {}
