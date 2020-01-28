use std::fmt;

/// An error that occurred during parsing or hydrating a filmReel file
#[derive(Debug, PartialEq)]
pub enum Error {
    FrameParse(&'static str),
    FrameParsef(&'static str, String),
    ReadInstruction(&'static str),
    ReadInstructionf(&'static str, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::FrameParse(err) => write!(f, "FrameParseError: {}", err),
            Error::FrameParsef(err, item) => {
                writeln!(f, "FrameParseError: {}", err)?;
                writeln!(f, " --> {}", item)?;
                Ok(())
            }
            Error::ReadInstruction(err) => write!(f, "ReadInstructionError: {}", err),
            Error::ReadInstructionf(err, item) => {
                writeln!(f, "ReadInstructionError: {}", err)?;
                writeln!(f, " --> {}", item)?;
                Ok(())
            }
        }
    }
}
