use crate::take::run_take;
use darkroom::*;
use log;

fn main() -> Result<(), BoxError> {
    let args: Command = argh::from_env();
    let opts: Opts = Opts::new(&args);

    let log_level = match opts.verbose {
        true => log::LevelFilter::Info,
        false => log::LevelFilter::Warn,
    };
    log::set_boxed_logger(Box::new(Logger)).map(|()| log::set_max_level(log_level))?;

    match args.nested {
        SubCommand::Take(cmd) => {
            run_take(cmd)?;
            Ok(())
        }
        _ => {
            println!("{:#?}", args);
            Ok(())
        }
    }
}
