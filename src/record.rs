use crate::{params::BaseParams, take::*, Record};
use anyhow::{anyhow, Context, Error};
use colored::*;
use filmreel as fr;
use fr::{cut::Register, frame::Frame, reel::*, ToStringHidden};
use log::{debug, error, warn};
use std::{
    fs,
    ops::Range,
    ops::RangeBounds,
    ops::Bound::*;
    path::{Path, PathBuf},
};

/// run_record runs through a Reel sequence using the darkroom::Record struct
pub fn run_record(cmd: Record, base_params: BaseParams) -> Result<(), Error> {
    let cut_str = fr::file_to_string(cmd.get_cut_file())?;
    let mut cut_register: Register = Register::from(&cut_str)?;
    let reel = Reel::new(&cmd.reel_path, &cmd.reel_name, &cmd.range)?;

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
    &cut_register.destructive_merge(merge_cuts?);

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
        &comp_reg.destructive_merge(vec![register]);
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

type InRange = Box<dyn FrameBounds>;
// parse_range parses the `"<start_num>:<end_num>"` provided to the `--range` cli argument
// returning a range object
fn parse_range<T>(str_range: T) -> Result<Option<InRange>, Error>
where
    T: AsRef<str>,
{
    match str_range
        .as_ref()
        .splitn(3, ':')
        .collect::<Vec<&str>>()
        .as_slice()
    {
        [start, end] => {
            let start_parse = || start.parse::<u32>().context("start range error");
            let end_parse = || end.parse::<u32>().context("end range error");
            if *start == "" {
                // make end string range inclusive
                let end_inclusive: u32 = end_parse()? + 1;
                Ok::<Option<InRange>, Error>(Some(Box::new(..end_inclusive)));
            }
            if *end == "" {
                Ok::<Option<InRange>, Error>(Some(Box::new(start_parse()?..)));
            }
            Ok(Some(Box::new(start_parse()?..end_parse()?)))
        }
        _ => Ok(None),
    }
}

trait FrameBounds {
    fn contains(&self, v: &u32) -> bool;
}

impl<T> FrameBounds for T
where
    T: RangeBounds<u32>,
{
    fn contains(&self, v: &u32) -> bool {
        RangeBounds::contains(self, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest(input, expected,
        case("04:08", Ok::<Option<InRange>, Error>(Some(Box::new(4..9)))),
        case(":10", Ok::<Option<InRange>, Error>(Some(Box::new(..11)))),
        case("3:", Ok::<Option<InRange>, Error>(Some(Box::new(3..)))),
        )]
    fn test_parse_range(input: &str, expected: Result<Option<InRange>, Error>) {
        match parse_range(input) {
            Ok(mat) => assert_eq!(expected.unwrap(), mat),
            Err(err) => assert_eq!("some_err", err.to_string()),
        }
    }

    #[test]
    fn test_metaframe_try_from() {
        let try_path = MetaFrame::try_from(PathBuf::from("./reel_name.01s.frame_name.fr.json"))
            .expect("test_metaframe_try_from failed try_from");
        assert_eq!(
            MetaFrame {
                frame_type: FrameType::Success,
                name: "frame_name".to_string(),
                path: PathBuf::from("./reel_name.01s.frame_name.fr.json"),
                reel_name: "reel_name".to_string(),
                step: 1.0,
            },
            try_path
        );
    }
}
