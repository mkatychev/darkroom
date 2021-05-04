use anyhow::{anyhow, Error};
use argh::FromArgs;
use mdcat::{push_tty, Environment, ResourceAccess, Settings, TerminalCapabilities, TerminalSize};
use minus::{page_all, Pager};
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::str;
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
    include_bytes!("../filmreel_md/Reel.md")
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

const ENTRY_DOCSTRING: &str = r#"<entry>:
    readme
    frame
    cut
    reel
    hidden-variables
    ignored-variables
    merge-cuts
    retry-attempts"#;

const FILMREEL_REPO: &str = "https://github.com/Bestowinc/filmReel/blob/master/";

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "man")]
#[argh(note = r#"<entry>:
readme
frame
cut
reel
hidden-variables
ignored-variables
merge-cuts
retry-attempts"#)]
/// return a given manual entry
pub struct Man {
    /// the manual entry to specify
    #[argh(positional, default = "String::from(\"readme\")")]
    pub entry: String,
}

impl Man {
    // output_entry renders markdown for various filmreel and darkroom concepts
    pub fn output_entry(&self) -> Result<(), Error> {
        let md = match &self.entry[..3] as &str {
            "rea" => readme(),                 // "readme"
            "cut" => cut(),                    // "cut"
            "ree" => reel(),                   // "reel"
            "fra" => frame(),                  // "frame"
            "ret" | "att" => retry_attempts(), // "retry-attempts" | "attempts"
            "mer" => merge_cuts(),             // "merge-cuts"
            "ign" => ignored_variables(),      // "ignored-variables" | "ignore" | "ignored"
            "hid" => hidden_variables(),       // "hidden-variables" | "hidden"
            _ => {
                return Err(anyhow!("invalid entry argument\n{}", ENTRY_DOCSTRING));
            }
        };

        let repo = Url::parse(FILMREEL_REPO)?;
        let parser = Parser::new_ext(str::from_utf8(md)?, Options::empty())
            .filter(|event| {
                if let Event::Html(_) = event {
                    return false;
                }
                true
            })
            .map(|event| match event {
                Event::End(Tag::Link(link, dest, title))
                    if !dest.starts_with("http") && dest.contains(".md") =>
                {
                    let new_str = repo.join(&dest).unwrap().to_string();

                    Event::End(Tag::Link(link, new_str.into(), title))
                }
                _ => event,
            });

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
