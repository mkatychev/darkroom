use anyhow::{anyhow, Error};
use argh::FromArgs;
use mdcat::{push_tty, Environment, ResourceAccess, Settings, TerminalCapabilities, TerminalSize};
use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag};
use std::{io, str};
use syntect::parsing::SyntaxSet;
use url::Url;

const fn readme() -> &'static [u8] {
    include_bytes!("../filmreel_md/README.md")
}

const fn frame() -> &'static [u8] {
    include_bytes!("../filmreel_md/frame.md")
}

const fn cut() -> &'static [u8] {
    include_bytes!("../filmreel_md/cut.md")
}

const fn reel() -> &'static [u8] {
    include_bytes!("../filmreel_md/reel.md")
}

/// Returns manual entries
/// valid arguments are:
///
/// readme,
/// frame,
/// cut,
/// reel
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "man")]
pub struct Man {
    /// the manual entry to specify
    #[argh(positional, default = "String::from(\"readme\")")]
    pub entry: String,
}

impl Man {
    // output_entry renders markdown for various filmreel and darkroom concepts
    pub fn output_entry(&self) -> Result<(), Error> {
        let md = match &self.entry as &str {
            "readme" => readme(),
            "cut" => cut(),
            "reel" => reel(),
            "frame" => frame(),
            _ => {
                return Err(anyhow!("invalid man argument: {}", &self.entry));
            }
        };

        let parser = Parser::new_ext(str::from_utf8(md)?, Options::empty()).filter(|event| {
            if let Event::Html(_) = event {
                return false;
            }
            true
        });

        // TODO replace relative paths with absolute URLs
        // let repo = Url::parse("https://github.com/Bestowinc/filmReel")?;
        // .map(|event| match event {
        //     Event::Start(Tag::Link(link @ LinkType::Inline, dest, title))
        //         if dest.ends_with(".md") =>
        //     {
        //         // dbg!(&s);
        //         // let new_str: String = repo.join(&dest).unwrap().clone().as_str().into();
        //         // dbg!(&new_str);

        //         Event::Start(Tag::Link(link, dest.replace("md", "nope").into(), title))
        //     }
        //     _ => event,
        // });
        // });

        // NOTE this does not do anything since markdown is pulled from constant functions
        let env = &Environment::for_local_directory(&"/")?;
        let settings = &Settings {
            resource_access:       ResourceAccess::LocalOnly,
            syntax_set:            SyntaxSet::default(),
            terminal_capabilities: TerminalCapabilities::detect(),
            terminal_size:         TerminalSize::from_terminal()
                .map_or_else(|| Err(anyhow!("termsize is None")), |v| Ok(v))?,
        };

        push_tty(settings, &env, &mut io::stdout(), parser)?;

        Ok(())
    }
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum SubCommand {
    Frame(Frame),
    Cut(Cut),
    Reel(Reel),
}

// TODO derive ManEntry
#[derive(PartialEq, Debug, FromArgs)]
#[argh(subcommand, name = "frame")]
/// frame documentation
pub struct Frame {}

#[derive(PartialEq, Debug, FromArgs)]
#[argh(subcommand, name = "cut")]
/// cut documentation
pub struct Cut {}

#[derive(PartialEq, Debug, FromArgs)]
#[argh(subcommand, name = "reel")]
/// reel documentation
pub struct Reel {}
