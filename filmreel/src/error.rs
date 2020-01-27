use std::fmt;

/// An error that occurred during parsing or hydrating a filmReel file
#[derive(Debug, PartialEq)]
pub enum Error {
    FrameParse(&'static str),
    ReadInstruction(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::FrameParse(err) => write!(f, "FrameParseError: {}", err),
            Error::ReadInstruction(err) => write!(f, "ReadInstructionError: {}", err),
        }
    }
}
