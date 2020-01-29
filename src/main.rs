use crate::take::run_take;
use darkroom::*;
use structopt::StructOpt;

fn main() {
    let opt = Opt::from_args();

    match opt.cmd {
        Command::Take { frame, cut, output } => run_take(frame, cut, output),
        _ => println!("{:#?}", opt),
    }
}
