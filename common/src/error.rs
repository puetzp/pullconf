use std::{fmt, process::ExitCode};

#[derive(Debug)]
pub struct Terminate;

impl std::error::Error for Terminate {}

impl fmt::Display for Terminate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("")
    }
}

impl From<Terminate> for ExitCode {
    fn from(_: Terminate) -> Self {
        Self::FAILURE
    }
}
