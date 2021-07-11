use anyhow::Error;
use darkroom::{
    record::{cmd_record, cmd_vrecord},
    take::cmd_take,
    *,
};
use std::io::{self, Write};

fn main() -> Result<(), Error> {
    let args: Command = argh::from_env();

    let opts: Opts = Opts::new(&args);
    let base_params = args.base_params();
    let nested_arg = args.get_nested();

    let log_level = if opts.verbose {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };

    log::set_boxed_logger(Box::new(Logger)).map(|()| log::set_max_level(log_level))?;

    let err_ts = |e: Error| -> Error {
        if base_params.timestamp {
            write!(io::stderr(), "[{}] ", chrono::Utc::now()).expect("write to stderr panic");
        }
        e
    };

    match nested_arg {
        SubCommand::Version(_) => {
            println!("{}", crate::version());
            Ok(())
        }
        #[cfg(feature = "man")]
        SubCommand::Man(cmd) => cmd.output_entry(),
        SubCommand::Take(cmd) => {
            cmd.validate()?;
            cmd_take(cmd, base_params)
        }
        SubCommand::Record(cmd) => {
            cmd.validate()?;
            cmd_record(cmd, base_params.clone()).map_err(err_ts)
        }
        SubCommand::VirtualRecord(cmd) => cmd_vrecord(cmd, base_params.clone()).map_err(err_ts),
    }
}
