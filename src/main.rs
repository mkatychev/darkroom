use crate::take::run_take;
use darkroom::*;
use structopt::StructOpt;

fn main() {
    let opt = Opt::from_args();

    match opt.cmd {
        Command::Take {
            frame,
            cut,
            header,
            dest,
            output,
        } => run_take(Command::Take {
            frame,
            cut,
            header,
            dest,
            output,
        }),
        _ => println!("{:#?}", opt),
    }
}
