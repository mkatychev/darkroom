use crate::take::run_take;
use colored::*;
use darkroom::*;
// use structopt::StructOpt;

fn main() {
    let args: Command = argh::from_env();
    // println!("{:#?}", args);

    // let opt = Opt::from_args();

    match args.nested {
        SubCommand::Take(cmd) => {
            if let Err(e) = run_take(cmd) {
                eprintln!("{}: {}", "error".red(), e)
            }
        }
        _ => {
            println!("{:#?}", args);
            ()
        }
    }
}
