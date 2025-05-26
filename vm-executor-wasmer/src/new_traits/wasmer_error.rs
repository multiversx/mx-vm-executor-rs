use thiserror::Error;

#[derive(Error, Debug)]
pub enum WasmerExecutorError {
    #[error("bad wasmer instance pointer")]
    BadInstancePointer,

    #[error("get points used error: {0}")]
    GetPointsUsed(String),

    #[error("set points used error: {0}")]
    SetPointsUsed(String),

    #[error("set points limit error: {0}")]
    SetPointsLimit(String),

    #[error("instance call error: {0}")]
    InstanceCall(String),

    #[error("instance call error: {0}")]
    WrappedInstance(String),
}
