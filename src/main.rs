use crate::take::run_take;
use darkroom::*;

fn main() -> Result<(), BoxError> {
    let args: Command = argh::from_env();
    let opts: Opts = Opts::new(&args);

    match args.nested {
        SubCommand::Take(cmd) => {
            run_take(cmd, opts)?;
            Ok(())
        }
        _ => {
            println!("{:#?}", args);
            Ok(())
        }
    }
}
