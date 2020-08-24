use crate::{params::BaseParams, take::*, Record};
use anyhow::{anyhow, Context, Error};
use colored::*;
use filmreel as fr;
use fr::{cut::Register, frame::Frame, reel::*, ToStringHidden};
use log::{debug, error, warn};
use std::{
    fs,
    ops::Range,
    path::{Path, PathBuf},
};

/// run_record runs through a Reel sequence using the darkroom::Record struct
pub fn run_record(cmd: Record, base_params: BaseParams) -> Result<(), Error> {
    let cut_str = fr::file_to_string(cmd.get_cut_file())?;
    let mut cut_register: Register = Register::from(&cut_str)?;
    let frame_range = match cmd.range {
        Some(r) => parse_range(r)?,
        None => None,
    };
    let reel = Reel::new(&cmd.reel_path, &cmd.reel_name, frame_range)?;

    // #### Component init
    let (mut comp_reels, mut comp_reg) = init_components(cmd.component)?;
    comp_reg.destructive_merge(vec![cut_register]);
    comp_reels.push(reel);
    cut_register = comp_reg;

    // #### Merge init
    // Merge any found PathBufs into the cut register destructively
    let merge_cuts: Result<Vec<Register>, _> = cmd
        .merge_cuts
        .iter()
        .flat_map(fr::file_to_string)
        .map(Register::from)
        .collect();
    cut_register.destructive_merge(merge_cuts?);

    for meta_frame in comp_reels.into_iter().flatten() {
        // if cmd.output is Some, provide a take PathBuf
        let output = cmd
            .take_out
            .as_ref()
            .map(|dir| take_output(&dir, &&meta_frame.path));
        warn!(
            "{} {:?}",
            "File:".yellow(),
            meta_frame
                .get_filename()
                .context("unable to unwrap MetaFrame.path")?
        );
        warn!("{}", "=======================".green());

        let frame_str = fr::file_to_string(&meta_frame.path)?;
        let frame = Frame::new(&frame_str)?;
        // Frame to be mutably borrowed
        let mut payload_frame = frame.clone();

        if let Err(e) = run_take(&mut payload_frame, &mut cut_register, &base_params, output) {
            write_cut(&base_params.cut_out, &cut_register, &cmd.reel_name, true)?;
            return Err(e);
        }
    }

    write_cut(&base_params.cut_out, &cut_register, &cmd.reel_name, false)?;

    Ok(())
}

/// write_cut dumps the in memory Cut Regiser to the PathBuf provided.
pub fn write_cut<T>(
    cut_out: &Option<PathBuf>,
    cut_register: &Register,
    reel_name: T,
    failed_response: bool,
) -> Result<(), Error>
where
    T: AsRef<str> + std::fmt::Display,
{
    if let Some(path) = cut_out {
        // announce that write_cut is dumping a failed record register
        if failed_response {
            error!("{}", "take aborted! writing to --cut-out provided...".red());
        }
        // write with a hidden cut if directory w,as provided
        if path.is_dir() {
            let dir_cut = &path.join(format!(".{}.cut.json", reel_name));
            fs::write(dir_cut, &cut_register.to_string_hidden()?)
                .context("unable to write to --cut_out directory")?;
        } else {
            debug!("writing cut output to PathBuf...");
            fs::write(path, &cut_register.to_string_hidden()?)
                .context("unable to write to cmd.get_cut_copy()")?;
        }
    }
    Ok(())
}

/// take_output grabs a Record command's output directory and joins it with a MetaFrame's file stem
pub fn take_output<P: AsRef<Path>>(dir: &P, file: &P) -> PathBuf {
    let frame_stem: &str = file
        .as_ref()
        .file_stem()
        .and_then(|f| f.to_str())
        .map(|f| f.trim_end_matches(".fr"))
        .expect("take_output: failed filepath trimming");

    dir.as_ref().join(format!("{}.tk.json", frame_stem))
}

/// create component output
pub fn init_components(components: Vec<String>) -> Result<(Vec<Reel>, Register), Error> {
    let mut comp_reg = Register::new();
    let mut reels = vec![];
    for comp in components {
        let (reel, register) = parse_component(comp)?;
        // TODO implement single merge
        comp_reg.destructive_merge(vec![register]);
        reels.push(reel);
    }

    Ok((reels, comp_reg))
}

// parse_component parses the `"<dir>&<reel_name>"` provided to the `--component` cli argument
// validating the ampersand separated directory and reel name are valid
fn parse_component(component: String) -> Result<(Reel, Register), Error> {
    let reel_path: PathBuf;
    let reel_name: &str;
    match component.splitn(2, '&').collect::<Vec<&str>>().as_slice() {
        [path_str, name_str] => {
            reel_path = PathBuf::from(path_str);
            reel_name = name_str;
        }
        _ => {
            return Err(anyhow!("unable to parse component string => {}", component));
        }
    }
    let reel = Reel::new(reel_path, reel_name, None)
        .context(format!("component Reel::new failure => {}", reel_name))?;
    let cut_path = reel.get_default_cut_path();
    if !cut_path.is_file() {
        return Err(anyhow!(
            "component cut must be a valid file => {:?}",
            cut_path
        ));
    }
    Ok((
        reel,
        Register::from(fr::file_to_string(cut_path.to_str().unwrap())?).context(format!(
            "component Register::from failure => {:?}",
            cut_path
        ))?,
    ))
}

type ParsedRange = Option<Range<u32>>;
// parse_range parses the `"<start_u32>:<end_u32>"` provided to the `--range` cli argument
// returning a range object
fn parse_range<T>(str_range: T) -> Result<ParsedRange, Error>
where
    T: AsRef<str>,
{
    match str_range
        .as_ref()
        .splitn(2, ':')
        .collect::<Vec<&str>>()
        .as_slice()
    {
        [start, end] => {
            let start_parse = || start.parse::<u32>().context("start range parse error");
            let end_parse = || end.parse::<u32>().context("end range parse error");
            if *start == "" {
                // make end string range inclusive
                return Ok(Some(0..end_parse()? + 1));
            }
            if *end == "" {
                return Ok(Some(start_parse()?..u32::MAX));
            }
            Ok(Some(start_parse()?..end_parse()? + 1))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest(input, expected,
        case("04:08", Ok::<ParsedRange, Error>(Some(4..9))),
        case(":10", Ok::<ParsedRange, Error>(Some(0..11))),
        case("3:", Ok::<ParsedRange, Error>(Some(3..u32::MAX))),
        case("number:", Err(anyhow!("start range parse error"))),
        case(":number", Err(anyhow!("end range parse error"))),
        case("number:number", Err(anyhow!("start range parse error"))),
        )]
    fn test_parse_range(input: &str, expected: Result<ParsedRange, Error>) {
        match parse_range(input) {
            Ok(mat) => assert_eq!(expected.unwrap(), mat),
            Err(err) => assert_eq!(expected.unwrap_err().to_string(), err.to_string()),
        }
    }
}
