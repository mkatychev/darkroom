use anyhow::{anyhow, Error};
use argh::FromArgs;
use mdcat::{push_tty, Environment, ResourceAccess, Settings, TerminalCapabilities, TerminalSize};
use minus::{page_all, Pager};
use pulldown_cmark::{Event, Options, Parser};
use std::str;
use syntect::parsing::SyntaxSet;

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

const fn hidden_variables() -> &'static [u8] {
    include_bytes!("../filmreel_md/extra_concepts/hidden_variables.md")
}

const fn ignored_variables() -> &'static [u8] {
    include_bytes!("../filmreel_md/extra_concepts/ignored_variables.md")
}

const fn merge_cuts() -> &'static [u8] {
    include_bytes!("../filmreel_md/extra_concepts/merge_cuts.md")
}

const fn retry_attempts() -> &'static [u8] {
    include_bytes!("../filmreel_md/extra_concepts/retry_attempts.md")
}

/// <entry>:
/// readme,
/// frame,
/// cut,
/// reel,
/// hidden-variables,
/// ignored-variables,
/// merge-cuts,
/// retry-attempts
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
            "retry-attempts" => retry_attempts(),
            "merge-cuts" => merge_cuts(),
            "ignored-variables" => ignored_variables(),
            "hidden-variables" => hidden_variables(),
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
                .map_or_else(|| Err(anyhow!("termsize is None")), Ok)?,
        };

        let mut pager = Pager::new();
        let mut buf = Vec::new();
        push_tty(settings, &env, &mut buf, parser)?;
        pager.lines = String::from_utf8(buf)?;
        pager.prompt = "darkroom".to_string();

        page_all(pager)?;

        Ok(())
    }
}
