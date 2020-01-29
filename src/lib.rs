use std::path::PathBuf;
use structopt::StructOpt;
use take::run_take;

pub mod take;

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Performs a single operation on a single frame file
    Take {
        /// Frame to process
        #[structopt(name = "FRAME", parse(from_os_str))]
        frame: PathBuf,
        /// Filepath of cut file
        #[structopt(short = "c", long)]
        cut: PathBuf,
        /// Output file
        #[structopt(short, long, parse(from_os_str))]
        output: Option<PathBuf>,
    },
    Record,
}

// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "Darkroom")]
pub struct Opt {
    #[structopt(subcommand)]
    pub cmd: Command,
}
