use anyhow::{anyhow, Context, Error};
use argh::FromArgs;
use mdcat::{push_tty, Environment, ResourceAccess, Settings, TerminalCapabilities, TerminalSize};
use pulldown_cmark::{Options, Parser};
use std::{io, str};
use syntect::parsing::SyntaxSet;
use url::Url;

const fn cut() -> &'static [u8] {
    include_bytes!("../filmreel_md/cut.md")
}
const fn reel() -> &'static [u8] {
    include_bytes!("../filmreel_md/reel.md")
}

/// Returns manual entries
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "man")]
pub struct Man {
    /// returns cargo package version, this is a temporary argh workaround
    #[argh(subcommand)]
    pub nested: SubCommand,
}

impl Man {
    pub fn output_entry(&self) -> Result<(), Error> {
        let md = match &self.nested {
            SubCommand::Cut(_) => cut(),
            SubCommand::Reel(_) => reel(),
        };

        let parser = Parser::new_ext(str::from_utf8(md)?, Options::empty());
        let env = &Environment {
            base_url: Url::parse("https://github.com/Bestowinc/filmReel")?,
            hostname: String::new(),
        };
        let settings = &Settings {
            resource_access:       ResourceAccess::LocalOnly,
            syntax_set:            SyntaxSet::default(),
            terminal_capabilities: TerminalCapabilities::none(),
            terminal_size:         TerminalSize::default(),
        };
        let mut stdout = io::stdout();

        push_tty(settings, env, &mut stdout, parser)?;

        Ok(())
    }
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum SubCommand {
    Cut(Cut),
    Reel(Reel),
}

#[derive(PartialEq, Debug, FromArgs)]
#[argh(subcommand, name = "cut")]
/// the cargo command
pub struct Cut {}

#[derive(PartialEq, Debug, FromArgs)]
#[argh(subcommand, name = "reel")]
/// the cargo command
pub struct Reel {}
