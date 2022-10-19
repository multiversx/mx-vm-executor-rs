use std::fmt::{self, Display};
use std::{error::Error, fmt::Formatter};

#[derive(Debug)]
pub struct ServiceError {
    message: &'static str,
}

impl ServiceError {
    pub fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl Display for ServiceError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl Error for ServiceError {}
