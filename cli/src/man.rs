use anyhow::{anyhow, Error};
use argh::FromArgs;
use minus::{page_all, Pager};
use pulldown_cmark::{Event, Options, Parser, Tag};
use pulldown_cmark_mdcat::{
    push_tty, resources::NoopResourceHandler, Environment, Settings, TerminalProgram, TerminalSize,
    Theme,
};
use std::str;
use syntect::parsing::SyntaxSet;
use url::Url;

const README: &str = include_str!("../../filmreel_md/README.md");
const FRAME: &str = include_str!("../../filmreel_md/frame.md");
const FRAME_QUICK: &str = include_str!("../../filmreel_md/quickref/frame.md");
const CUT: &str = include_str!("../../filmreel_md/cut.md");
const CUT_QUICK: &str = include_str!("../../filmreel_md/quickref/cut.md");
const REEL: &str = include_str!("../../filmreel_md/reel.md");
const REEL_QUICK: &str = include_str!("../../filmreel_md/quickref/reel.md");
const HIDDEN_VARIABLES: &str = include_str!("../../filmreel_md/extra_concepts/hidden_variables.md");
const IGNORED_VARIABLES: &str =
    include_str!("../../filmreel_md/extra_concepts/ignored_variables.md");
const MERGE_CUTS: &str = include_str!("../../filmreel_md/extra_concepts/merge_cuts.md");
const RETRY_ATTEMPTS: &str = include_str!("../../filmreel_md/extra_concepts/retry_attempts.md");
const MISMATCH: &str = include_str!("../../filmreel_md/extra_concepts/mismatch.md");
const COMPONENT: &str = include_str!("../../filmreel_md/extra_concepts/component.md");
const FILENAME: &str = include_str!("../../filmreel_md/quickref/frame_type.md");
const STORAGE: &str = include_str!("../../filmreel_md/extra_concepts/cut_storage.md");
const VALIDATION: &str = include_str!("../../filmreel_md/extra_concepts/validation.md");

const ENTRY_DOCSTRING: &str = r#"<entry>:
    readme
    frame
    cut
    reel
    component
    filename
    hidden-variables
    ignored-variables
    merge-cuts
    mismatch
    retry-attempts
    storage
    validation
    "#;

const FILMREEL_REPO: &str = "https://github.com/mkatychev/filmReel/blob/master/";

#[derive(FromArgs, PartialEq, Eq, Debug)]
#[argh(subcommand, name = "man")]
#[argh(note = r#"<entry>:
readme
frame
cut
reel
component
filename
hidden-variables
ignored-variables
merge-cuts
mismatch
retry-attempts
storage
validation"#)]
/// return a given manual entry
pub struct Man {
    /// the manual entry to specify
    #[argh(positional, default = "String::from(\"readme\")")]
    pub entry: String,
    /// return the TLDR variant of: reel, frame, and cut
    #[argh(switch, short = 'q')]
    pub quick: bool,
}

impl Man {
    // output_entry renders markdown for various filmreel and darkroom concepts
    pub fn output_entry(&self) -> Result<(), Error> {
        let md = match (&self.entry[..3], self.quick) {
            ("rea", _) => README,
            ("cut", true) => CUT_QUICK,           // "cut"
            ("cut", false) => CUT,                // "cut"
            ("ree", true) => REEL_QUICK,          // "reel"
            ("ree", false) => REEL,               // "reel"
            ("fra", true) => FRAME_QUICK,         // "frame"
            ("fra", false) => FRAME,              // "frame"
            ("com", _) => COMPONENT,              // "component"
            ("fil", _) => FILENAME,               // "filename"
            ("hid", _) => HIDDEN_VARIABLES,       // "hidden-variables" | "hidden"
            ("ign", _) => IGNORED_VARIABLES,      // "ignored-variables" | "ignore" | "ignored"
            ("mer", _) => MERGE_CUTS,             // "merge-cuts"
            ("mis", _) => MISMATCH,               // "mismatch"
            ("ret" | "att", _) => RETRY_ATTEMPTS, // "retry-attempts" | "attempts"
            ("sto", _) => STORAGE,                // "storage"
            ("par" | "uno" | "val", _) => VALIDATION,
            _ => {
                return Err(anyhow!("invalid entry argument\n{}", ENTRY_DOCSTRING));
            }
        };

        let repo = Url::parse(FILMREEL_REPO)?;
        let parser = Parser::new_ext(md, Options::empty())
            .filter(|event| {
                if let Event::Html(_) = event {
                    return false;
                }
                true
            })
            .map(|event| match event {
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) if !dest_url.starts_with("http") && dest_url.contains(".md") => {
                    let dest_url = repo.join(&dest_url).unwrap().to_string().into();

                    Event::Start(Tag::Link {
                        link_type,
                        dest_url,
                        title,
                        id,
                    })
                }
                _ => event,
            });

        // NOTE this does not do anything since markdown is pulled from constant functions
        let env = &Environment::for_local_directory(&"/")?;
        let settings = &Settings {
            theme: Theme::default(),
            syntax_set: &SyntaxSet::default(),
            terminal_capabilities: TerminalProgram::detect().capabilities(),
            terminal_size: TerminalSize::from_terminal()
                .map_or_else(|| Err(anyhow!("termsize is None")), Ok)?,
        };

        let mut pager = Pager::new();
        let mut buf = Vec::new();
        push_tty(settings, env, &NoopResourceHandler, &mut buf, parser)?;
        pager.lines = String::from_utf8(buf)?;
        pager.prompt = "darkroom".to_string();

        page_all(pager)?;

        Ok(())
    }
}
