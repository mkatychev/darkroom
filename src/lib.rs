use crate::params::BaseParams;
use anyhow::{anyhow, Error};
use argh::FromArgs;
use colored_json::{prelude::*, Colour, Styler};
use serde::Serialize;
use std::{convert::TryFrom, fs, path::PathBuf};

#[cfg(feature = "man")]
use crate::man::Man;

pub mod grpc;
pub mod http;
pub mod params;
pub mod record;
pub mod take;

#[cfg(feature = "man")]
mod man;

pub use filmreel::{
    FrError, Frame, MetaFrame, Reel, Register, ToStringHidden, ToStringPretty, VirtualReel,
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

/// show version
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Darkroom: A contract testing tool built in Rust using the filmReel format.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(
    note = "Use `{command_name} man` for details on filmReel, the JSON format.",
    example = "Step through the httpbin test in [-i]nteractive mode:
$ {command_name} -i record ./test_data post
",
    example = "Echo the origin `${{IP}}` that gets written to the cut register from the httpbin.org POST request:
$ {command_name} --cut-out >(jq .IP) take ./test_data/post.01s.body.fr.json"
)]
pub struct Command {
    /// enable verbose output
    #[argh(switch, short = 'v')]
    verbose: bool,

    /// fallback address passed to the specified protocol
    #[argh(positional)]
    address: Option<String>,

    /// fallback header passed to the specified protocol
    #[argh(option, short = 'H')]
    header: Option<String>,

    /// output of final cut file
    #[argh(option, arg_name = "file")]
    cut_out: Option<PathBuf>,

    /// interactive frame sequence transitions
    #[argh(switch, short = 'i')]
    interactive: bool,

    /// enable TLS (automatically inferred for HTTP/S)
    #[argh(switch)]
    tls: bool,

    /// the path to a directory from which proto sources can be imported, for use with --proto flags.
    #[argh(option, arg_name = "dir")]
    proto_dir: Vec<PathBuf>,

    /// pass proto files used for payload forming
    #[argh(option, short = 'p', arg_name = "file")]
    proto: Vec<PathBuf>,

    #[argh(subcommand)]
    pub nested: SubCommand,
}

impl Command {
    pub fn base_params(&self) -> BaseParams {
        BaseParams {
            timeout:     30,
            timestamp:   false,
            tls:         self.tls,
            header:      self.header.clone(),
            address:     self.address.clone(),
            proto_path:  self.proto_dir.clone(),
            proto:       self.proto.clone(),
            cut_out:     self.cut_out.clone(),
            interactive: self.interactive,
            verbose:     self.verbose,
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
    #[cfg(feature = "man")]
    Man(Man),
    VirtualRecord(VirtualRecord),
}

/// Returns CARGO_PKG_VERSION
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "version")]
pub struct Version {
    /// returns cargo package version, this is a temporary argh workaround
    #[argh(switch)]
    version: bool,
}

/// Takes a single frame, emitting the request then validating the returned response
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "take")]
#[argh(
    example = "Echo the origin `${{IP}}` that gets written to the cut register from the httpbin.org POST request:
$ dark --cut-out >(jq .IP) take ./test_data/post.01s.body.fr.json"
)]
pub struct Take {
    /// path of the frame to process
    #[argh(positional)]
    frame: PathBuf,

    /// filepath of input cut file
    #[argh(option, short = 'c')]
    cut: Option<PathBuf>,

    /// ignore looking for a cut file when running take
    #[argh(switch, short = 'n')]
    no_cut: bool,

    /// output of take file
    #[argh(option, short = 'o', arg_name = "file")]
    take_out: Option<PathBuf>,

    /// filepath of merge cuts
    #[argh(positional)]
    merge_cuts: Vec<String>,
}

/// Attempts to play through an entire Reel sequence running a take for every frame in the sequence
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "record")]
#[argh(
    example = "Step through the httpbin test in [-i]nteractive mode:
$ dark -i record ./test_data post
",
    example = "Echo the origin `${{IP}}` that gets written to the cut register from the httpbin.org POST request:
$ dark --cut-out >(jq .IP) record ./test_data post"
)]
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

    /// repeatable component reel pattern using an ampersand separator: --component "<dir>&<reel_name>"
    #[argh(option, short = 'b')]
    component: Vec<String>,

    /// filepath of merge cuts
    #[argh(positional)]
    merge_cuts: Vec<String>,

    /// output directory for successful takes
    #[argh(option, short = 'o')]
    take_out: Option<PathBuf>,

    /// the range (inclusive) of frames that a record session will use, colon separated: --range <start>:<end> --range <start>:
    #[argh(option, short = 'r')]
    range: Option<String>,

    /// client request timeout in seconds, --timeout 0 disables request timeout [default: 30]
    #[argh(option, short = 't', default = "30")]
    timeout: u64,

    /// print timestamp at take start, error return, and reel completion
    #[argh(switch, short = 's')]
    timestamp: bool,

    /// print total time elapsed from record start to completion
    #[argh(switch, short = 'd')]
    duration: bool,
}

/// Attempts to play through an entire VirtualReel sequence running a take for every frame in the sequence
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "vrecord")]
#[argh(example = "Run the post reel in a v-reel setup:
$ {command_name} ./test_data/post.vr.json
$ {command_name} ./test_data/alt_post.vr.json")]
pub struct VirtualRecord {
    /// filepath or json string of VirtualReel
    #[argh(positional)]
    vreel: String,

    /// output directory for successful takes
    #[argh(option, short = 'o')]
    take_out: Option<PathBuf>,

    /// client request timeout in seconds, --timeout 0 disables request timeout [default: 30]
    #[argh(option, short = 't', default = "30")]
    timeout: u64,

    /// print timestamp at take start, error return, and reel completion
    #[argh(switch, short = 's')]
    timestamp: bool,

    /// print total time elapsed from record start to completion
    #[argh(switch, short = 'd')]
    duration: bool,
}

impl Take {
    /// validate ensures the frame and cut filepaths provided point to valid files
    pub fn validate(&self) -> Result<(), Error> {
        if !self.frame.is_file() {
            return Err(anyhow!("<frame> must be a valid file"));
        }

        // if there are merge cuts to use or --no-cut was specified
        // return early
        if !self.merge_cuts.is_empty() || self.no_cut {
            return Ok(());
        }

        let cut_file = self.get_cut_file()?;
        if !cut_file.is_file() {
            return Err(anyhow!(
                "{} must be a valid file",
                cut_file.to_string_lossy()
            ));
        }

        Ok(())
    }

    /// Returns expected cut filename in the given directory with the reel name derived from
    /// the provided frame file
    pub fn get_cut_file(&self) -> Result<PathBuf, Error> {
        if let Some(cut) = &self.cut {
            return Ok(cut.clone());
        }
        let metaframe = filmreel::reel::MetaFrame::try_from(&self.frame)?;
        let dir = fs::canonicalize(&self.frame)?;
        Ok(metaframe.get_cut_file(dir.parent().unwrap()))
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
            if !self.get_cut_file().is_file() && self.merge_cuts.is_empty() {
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

impl VirtualRecord {
    pub fn init(&self) -> Result<VirtualReel, Error> {
        let mut vreel = if guess_json_obj(&self.vreel) {
            serde_json::from_str(&self.vreel)?
        } else {
            let vreel_path = PathBuf::from(&self.vreel);
            let mut vreel_file = VirtualReel::try_from(vreel_path.clone())?;
            // default to parent directory of vreel file if path is not specified
            if vreel_file.path.is_none() {
                let parent_dir = fs::canonicalize(vreel_path.parent().unwrap().to_path_buf())?;
                vreel_file.path = Some(parent_dir);
            }
            vreel_file
        };
        vreel.join_path();

        Ok(vreel)
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

// try to see if a given string *might* be json
pub fn guess_json_obj<T: AsRef<str>>(input: T) -> bool {
    let obj = input
        .as_ref()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();

    obj.starts_with("{\"") && obj[2..].contains("\":") && obj.ends_with('}')
}
