use crate::utils::Rule;
use colored::*;
use pest::error::Error as PestError;
use serde_hashkey::Error as HashKeyError;
use serde_json::error::{Category, Error as SerdeError};
use std::{error::Error, fmt, path::Path};

/// An error that occurred during parsing or hydrating a filmReel file
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum FrError {
    FrameParse(&'static str),
    FrameParsef(&'static str, String),
    ReelParsef(&'static str, String),
    ReadInstruction(&'static str),
    WriteInstruction(&'static str),
    ReadInstructionf(&'static str, String),
    ReelParse(&'static str),
    Serde(String),
    Parse(String),
    File(String, String),
    Pest(PestError<Rule>),
}

impl Error for FrError {
    fn description(&self) -> &str {
        "Error related to filmReel"
    }
}

impl From<SerdeError> for FrError {
    fn from(err: SerdeError) -> FrError {
        match err.classify() {
            Category::Io => unreachable!(),
            Category::Syntax | Category::Data | Category::Eof => FrError::Serde(err.to_string()),
        }
    }
}

impl From<PestError<Rule>> for FrError {
    fn from(err: PestError<Rule>) -> FrError {
        Self::Pest(err)
    }
}

impl From<HashKeyError> for FrError {
    fn from(err: HashKeyError) -> FrError {
        Self::Parse(err.to_string())
    }
}

macro_rules! errorf {
    ($fmt: expr, $err_name:expr, $err_msg:expr, $item: expr) => {
        writeln!($fmt, "\n{}", "=======================".red())?;
        writeln!($fmt, "{}: {}", $err_name.yellow(), $err_msg)?;
        writeln!($fmt, "{} {}", "-->".bright_black(), $item)?;
        writeln!($fmt, "{}", "=======================".red())?;
    };
}
impl fmt::Display for FrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FrError::FrameParse(msg) => write!(f, "FrameParseError: {}", msg),
            FrError::ReelParse(msg) => write!(f, "ReelParseError: {}", msg),
            FrError::WriteInstruction(msg) => write!(f, "WriteInstructionError: {}", msg),
            FrError::ReadInstruction(msg) => write!(f, "ReadInstructionError: {}", msg),
            FrError::FrameParsef(msg, item) => {
                errorf!(f, "FrameParseError", msg, item);
                Ok(())
            }
            FrError::ReelParsef(msg, item) => {
                errorf!(f, "ReelParseError", msg, item);
                Ok(())
            }
            FrError::ReadInstructionf(msg, item) => {
                errorf!(f, "ReadInstructionError", msg, item);
                Ok(())
            }
            FrError::Serde(msg) => {
                writeln!(f, "SerdeError {} {}", "-->".red(), msg)?;
                Ok(())
            }
            FrError::Parse(msg) => {
                writeln!(f, "ParseError {} {}", "-->".red(), msg)?;
                Ok(())
            }
            FrError::Pest(msg) => {
                writeln!(f, "PestError {} {}", "-->".red(), msg)?;
                Ok(())
            }
            FrError::File(msg, item) => {
                errorf!(f, "FileError", msg, item);
                Ok(())
            }
        }
    }
}

/// adds the given filepath as context for [`FrError::File`]
pub trait WithPath<T, P> {
    fn with_path(self, path: P) -> Result<T, FrError>;
}

impl<T, E, P> WithPath<T, P> for Result<T, E>
where
    E: Error,
    P: AsRef<Path>,
{
    fn with_path(self, path: P) -> Result<T, FrError> {
        self.map_err(|e| FrError::File(path.as_ref().to_string_lossy().to_string(), e.to_string()))
    }
}
