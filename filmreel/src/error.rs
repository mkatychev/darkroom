use colored::*;
use serde_json::error::Error as SerdeError;
use std::error::Error;
use std::fmt;

/// An error that occurred during parsing or hydrating a filmReel file
#[derive(Debug, PartialEq)]
pub enum FrError {
    FrameParse(&'static str),
    FrameParsef(&'static str, String),
    ReadInstruction(&'static str),
    WriteInstruction(&'static str),
    ReadInstructionf(&'static str, String),
    Serde(String),
}

impl Error for FrError {
    fn description(&self) -> &str {
        "Error related to filmReel"
    }
}

macro_rules! errorf {
    ($fmt: expr, $err_name:expr, $err_msg:expr, $item: expr) => {
        writeln!($fmt, "=======================")?;
        writeln!($fmt, "{}: {}", $err_name, $err_msg)?;
        writeln!($fmt, "{} {}", "-->".red(), $item)?;
        writeln!($fmt, "=======================")?;
    };
}

impl From<SerdeError> for FrError {
    fn from(err: SerdeError) -> FrError {
        use serde_json::error::Category;
        match err.classify() {
            Category::Io => unreachable!(),
            Category::Syntax | Category::Data | Category::Eof => FrError::Serde(err.to_string()),
        }
    }
}

impl fmt::Display for FrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FrError::FrameParse(msg) => write!(f, "FrameParseError: {}", msg),
            FrError::WriteInstruction(msg) => write!(f, "WriteInstructionError: {}", msg),
            FrError::ReadInstruction(msg) => write!(f, "ReadInstructionError: {}", msg),
            FrError::FrameParsef(msg, item) => {
                errorf!(f, "FrameParseError", msg, item);
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
        }
    }
}
