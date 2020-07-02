use crate::params::BaseParams;
use anyhow::{anyhow, Error};
use argh::FromArgs;
use colored_json::{prelude::*, Colour, Styler};
use serde::Serialize;
use std::path::PathBuf;

pub mod grpc;
pub mod http;
pub mod params;
pub mod record;
pub mod take;

pub use filmreel::{
    cut::Register,
    frame::*,
    reel::{MetaFrame, Reel},
    FrError, ToStringHidden, ToStringPretty,
};

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

    /// enable TLS (automatically inferred for HTTP/S)
    #[argh(switch)]
    tls: bool,

    /// pass proto files used for payload forming
    #[argh(option, short = 'p')]
    proto: Vec<PathBuf>,

    /// fallback address passed to the specified protocol
    #[argh(positional, short = 'a')]
    address: Option<String>,

    /// fallback header passed to the specified protocol
    #[argh(option, short = 'H')]
    header: Option<String>,

    /// output of final cut file
    #[argh(option, short = 'C')]
    cut_out: Option<PathBuf>,

    /// interactive frame sequence transitions
    #[argh(switch, short = 'i')]
    interactive: bool,

    #[argh(subcommand)]
    pub nested: SubCommand,
}

impl Command {
    pub fn base_params(&self) -> BaseParams {
        BaseParams {
            tls: self.tls,
            header: self.header.clone(),
            address: self.address.clone(),
            proto: self.proto.clone(),
            cut_out: self.cut_out.clone(),
            interactive: self.interactive,
            verbose: self.verbose,
        }
    }

    pub fn get_nested(self) -> SubCommand {
        self.nested
    }
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
    Version(Version),
    Take(Take),
    Record(Record),
}

/// Returns CARGO_PKG_VERSION
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "version")]
pub struct Version {
    /// returns cargo package version, this is a temporary argh workaround
    #[argh(switch)]
    version: bool,
}

/// argh version workaround
pub fn version() -> String {
    option_env!("CARGO_PKG_VERSION")
        .unwrap_or("unknown")
        .to_string()
}

/// Takes a single frame, emitting the request then validating the returned response
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "take")]
pub struct Take {
    /// path of the frame to process
    #[argh(positional)]
    frame: PathBuf,

    /// filepath of input cut file
    #[argh(option, short = 'c')]
    cut: PathBuf,

    /// output of take file
    #[argh(option, short = 'o')]
    take_out: Option<PathBuf>,
}

/// Attempts to play through an entire Reel sequence running a take for every frame in the sequence
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "record")]
pub struct Record {
    /// directory path where frames and (if no explicit cut is provided) the cut are to be found
    #[argh(positional)]
    reel_path: PathBuf,

    /// name of the reel, used to find corresponding frames for the path provided
    #[argh(positional)]
    reel_name: String,

    /// filepath of input cut file
    #[argh(option, short = 'c')]
    cut: Option<PathBuf>,

    /// repeatable component reel pattern using an ampersand separator: `--component "<dir>&<reel_name>"`
    #[argh(option, short = 'b')]
    component: Vec<String>,

    /// filepath of merge cuts
    #[argh(positional)]
    merge_cuts: Vec<PathBuf>,

    /// output directory for successful takes
    #[argh(option, short = 'o')]
    take_out: Option<PathBuf>,
}

impl Take {
    /// validate ensures the frame and cut filepaths provided point to valid files
    pub fn validate(&self) -> Result<(), Error> {
        if !self.frame.is_file() {
            return Err(anyhow!("<frame> must be a valid file"));
        }

        // TODO for now remove file requirement
        //
        // this permits describable zsh `=(thing)` or basic `<(thing)` FIFO syntax
        // https://superuser.com/questions/1059781/what-exactly-is-in-bash-and-in-zsh
        // if !self.cut.is_file() {
        //     return Err("<cut> must be a valid file");
        // }
        Ok(())
    }
}
impl Record {
    /// validate ensures the reels is a valid directory and ensures that the corresponding cut file
    /// exists
    pub fn validate(&self) -> Result<(), Error> {
        if !self.reel_path.is_dir() {
            return Err(anyhow!("<path> must be a valid directory"));
        }

        if let Some(cut) = &self.cut {
            if !cut.is_file() {
                return Err(anyhow!("<cut> must be a valid file"));
            }
        } else {
            // check existence of implicit cut file in the same directory
            if !self.get_cut_file().is_file() {
                return Err(anyhow!(
                    "unable to find a matching cut file in the given directory"
                ));
            }
        }

        if let Some(output) = &self.take_out {
            if !output.is_dir() {
                return Err(anyhow!("<output> must be a valid directory"));
            }
        }
        Ok(())
    }

    /// Returns expected cut filename in the given directory with the provided reel name
    pub fn get_cut_file(&self) -> PathBuf {
        if let Some(cut) = &self.cut {
            return cut.clone();
        }

        self.reel_path.join(format!("{}.cut.json", self.reel_name))
    }

    /// Returns a period  appended path of the current cut file attempting to reduce the likelihood
    /// that the original cut will be overwritten or for the output to be committed to version control
    pub fn get_cut_copy(&self) -> PathBuf {
        self.reel_path.join(format!(".{}.cut.json", self.reel_name))
    }
}

/// get_styler returns the custom syntax values for stdout json
fn get_styler() -> Styler {
    Styler {
        bool_value: Colour::Purple.normal(),
        float_value: Colour::RGB(255, 123, 0).normal(),
        integer_value: Colour::RGB(255, 123, 0).normal(),
        nil_value: Colour::Cyan.normal(),
        string_include_quotation: false,
        ..Default::default()
    }
}

trait ToTakeColouredJson {
    fn to_coloured_tk_json(&self) -> Result<String, FrError>;
}

impl<T> ToTakeColouredJson for T
where
    T: ?Sized + Serialize,
{
    fn to_coloured_tk_json(&self) -> Result<String, FrError> {
        Ok(self
            .to_string_pretty()?
            .to_colored_json_with_styler(ColorMode::default().eval(), get_styler())?)
    }
}

trait ToTakeHiddenColouredJson: ToTakeColouredJson {
    fn to_hidden_tk_json(&self) -> Result<String, FrError>;
}

impl<T> ToTakeHiddenColouredJson for T
where
    T: ?Sized + Serialize,
{
    fn to_hidden_tk_json(&self) -> Result<String, FrError> {
        Ok(self
            .to_string_hidden()?
            .to_colored_json_with_styler(ColorMode::default().eval(), get_styler())?)
    }
}
