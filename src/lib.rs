use argh::FromArgs;
use log;
use std::error::Error;
use std::path::PathBuf;

pub mod grpc;
pub mod take;

pub type BoxError = Box<dyn Error>;

pub struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!("{}", record.args());
        }
    }

    fn flush(&self) {}
}

/// Top-level command.
#[derive(FromArgs, PartialEq, Debug)]
pub struct Command {
    /// enable verbose output
    #[argh(switch, short = 'v')]
    verbose: bool,

    #[argh(subcommand)]
    pub nested: SubCommand,
}

/// Additional options such as verbosity
pub struct Opts {
    pub verbose: bool,
}

impl Opts {
    pub fn new(cmd: &Command) -> Self {
        Self {
            verbose: cmd.verbose,
        }
    }
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum SubCommand {
    Take(Take),
}

/// Generate and send a single Request and Response
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "take")]
pub struct Take {
    /// frame to process
    #[argh(positional)]
    frame: String,

    /// address passed to grpcurl
    #[argh(positional)]
    addr: String,

    /// filepath of cut file
    #[argh(option, short = 'c')]
    cut: PathBuf,

    /// args passed to grpcurl
    #[argh(option, short = 'H')]
    header: String,

    /// output file
    #[argh(option, short = 'o')]
    output: Option<PathBuf>,
}

/// Dark Record
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "record")]
pub struct Record {
    /// frame to process
    #[argh(positional)]
    frame: PathBuf,

    /// address passed to grpcurl
    #[argh(positional)]
    addr: String,

    /// filepath of cut file, assumed to be in the same directory as the
    /// frame argument
    #[argh(option, short = 'c')]
    cut: Option<PathBuf>,

    /// args passed to grpcurl
    #[argh(option, short = 'H')]
    header: String,

    /// output file
    #[argh(option, short = 'o')]
    output: Option<PathBuf>,
}

// A basic example
