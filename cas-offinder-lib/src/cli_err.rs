use std::io;
use std::num;
use std::string;
use std::sync::mpsc::SendError;

#[derive(Debug)]
pub enum CliError {
    IoError(io::Error),
    BadFileFormat(&'static str),
    ParseIntError(num::ParseIntError),
    FromUtf8Error(string::FromUtf8Error),
    SendError(String),
    ArgumentError(&'static str),
}
pub type Result<T> = std::result::Result<T, CliError>;

impl From<io::Error> for CliError {
    fn from(error: io::Error) -> Self {
        CliError::IoError(error)
    }
}

impl From<num::ParseIntError> for CliError {
    fn from(error: num::ParseIntError) -> Self {
        CliError::ParseIntError(error)
    }
}

impl From<string::FromUtf8Error> for CliError {
    fn from(error: string::FromUtf8Error) -> Self {
        CliError::FromUtf8Error(error)
    }
}
impl<T> From<SendError<T>> for CliError {
    fn from(error: SendError<T>) -> Self {
        CliError::SendError(error.to_string())
    }
}
