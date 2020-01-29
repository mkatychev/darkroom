use serde_json::error::Error as SerdeError;
use std::error::Error;
use std::fmt;

/// An error that occurred during parsing or hydrating a filmReel file
#[derive(Debug, PartialEq)]
pub enum FrError {
    FrameParse(&'static str),
    FrameParsef(&'static str, String),
    ReadInstruction(&'static str),
    ReadInstructionf(&'static str, String),
    Serde(String),
}

impl Error for FrError {
    fn description(&self) -> &str {
        "Error related to filmReel"
    }
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
            FrError::FrameParse(err) => write!(f, "FrameParseError: {}", err),
            FrError::FrameParsef(err, item) => {
                writeln!(f, "FrameParseError: {}", err)?;
                writeln!(f, " --> {}", item)?;
                Ok(())
            }
            FrError::ReadInstruction(err) => write!(f, "ReadInstructionError: {}", err),
            FrError::ReadInstructionf(err, item) => {
                writeln!(f, "ReadInstructionError: {}", err)?;
                writeln!(f, " --> {}", item)?;
                Ok(())
            }
            FrError::Serde(err) => {
                writeln!(f, "SerdeError:")?;
                writeln!(f, " --> {}", err)?;
                Ok(())
            }
        }
    }
}
