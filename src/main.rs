use darkroom::record::run_record;
use darkroom::take::single_take;
use darkroom::*;
use log;

fn main() -> Result<(), BoxError> {
    let args: Command = argh::from_env();
    let opts: Opts = Opts::new(&args);

    let log_level = if opts.verbose {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };

    log::set_boxed_logger(Box::new(Logger)).map(|()| log::set_max_level(log_level))?;

    match args.nested {
        SubCommand::Take(cmd) => {
            cmd.validate()?;
            single_take(cmd)?;
            Ok(())
        }
        SubCommand::Record(cmd) => {
            cmd.validate()?;
            run_record(cmd)?;
            Ok(())
        }
    }
}
