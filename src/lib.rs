use argh::FromArgs;
use std::error::Error;
use std::path::PathBuf;

pub mod grpc;
pub mod params;
pub mod record;
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
    Record(Record),
}

/// Takes a single frame, sends the request and compares the returned response
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "take")]
pub struct Take {
    /// frame to process
    #[argh(positional)]
    frame: PathBuf,

    /// address passed to grpcurl
    #[argh(positional)]
    addr: Option<String>,

    /// filepath of cut file
    #[argh(option, short = 'c')]
    cut: PathBuf,

    /// args passed to grpcurl
    #[argh(option, short = 'H')]
    header: Option<String>,

    /// output of take file
    #[argh(option, short = 'o')]
    output: Option<PathBuf>,
}

/// Attemps to play through an entire Reel sequence
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "record")]
pub struct Record {
    /// directory where frames and cut register is located
    #[argh(positional)]
    path: PathBuf,

    /// name of the reel
    #[argh(positional)]
    name: String,

    /// header string passed to grpcurl
    #[argh(option, short = 'H')]
    header: Option<String>,

    /// address passed to grpcurl
    #[argh(option, short = 'a')]
    addr: Option<String>,

    /// filepath of output cut file
    #[argh(option, short = 'c')]
    cut: Option<PathBuf>,

    /// output directory for successful takes
    #[argh(option, short = 'o')]
    output: Option<PathBuf>,

    /// interactive frame sequence transitions
    #[argh(switch, short = 'i')]
    interactive: bool,
}

impl Take {
    pub fn validate(&self) -> Result<(), &str> {
        if !self.frame.is_file() {
            return Err("<frame> must be a valid file");
        }
        if !self.cut.is_file() {
            return Err("<cut> must be a valid file");
        }
        Ok(())
    }
}
impl Record {
    pub fn validate(&self) -> Result<(), &str> {
        if !self.path.is_dir() {
            return Err("<path> must be a valid directory");
        }

        if let Some(cut) = &self.cut {
            if !cut.is_file() {
                return Err("<cut> must be a valid file");
            }
        } else {
            // check existence of implicit cut file in the same directory
            if !self.get_cut_file().is_file() {
                return Err("unable to find a matching cut file in the given directory");
            }
        }

        if let Some(output) = &self.output {
            if !output.is_dir() {
                return Err("<output> must be a valid directory");
            }
        }
        Ok(())
    }

    /// Returns expected cut filename in the given directory with the provided reel name
    pub fn get_cut_file(&self) -> PathBuf {
        self.path.join(format!("{}.cut.json", self.name))
    }

    /// Checks for the existence of a copy cut file in the given directory with the provided reel name
    pub fn get_cut_copy(&self) -> PathBuf {
        self.path.join(format!(".{}.cut.json", self.name))
    }
}
