use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExperimentalError {
    #[error("{0} global access error: {1}")]
    GlobalAccess(&'static str, wasmer::ExportError),

    #[error("{0} set global value error")]
    SetGlobalValue(&'static str, wasmer::RuntimeError),

    #[error("{0} error parsing global value: {0}")]
    ParseGlobalValue(&'static str, &'static str),

    #[error("uninitialized wasmer_inner weak pointer")]
    BadInstanceInnerPointer,

    #[error("error decoding breakpoint value: {0}")]
    UnknownBreakpointValue(u64),

    #[error("instance call error: {0}")]
    InstanceCall(wasmer::RuntimeError),
}
