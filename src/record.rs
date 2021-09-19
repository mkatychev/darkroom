use crate::{guess_json_obj, params::BaseParams, take::*, Record, VirtualRecord};
use anyhow::{anyhow, Context, Error};
use colored::*;
use filmreel as fr;
use fr::{cut::Register, frame::Frame, reel::*, ToStringHidden};
use log::{debug, error, warn};
use std::{
    convert::TryFrom,
    fs,
    ops::Range,
    path::{Path, PathBuf},
    time::Instant,
};

pub struct RecordRunner {
    duration:   bool,
    reel_name:  String,
    take_out:   Option<PathBuf>,
    register:   Register,
    pub frames: Vec<MetaFrame>,
}

pub fn cmd_record(cmd: Record, mut base_params: BaseParams) -> Result<(), Error> {
    base_params.timeout = cmd.timeout;
    base_params.timestamp = cmd.timestamp;

    let mut cut_register = Register::try_from(cmd.get_cut_file())?;
    let frame_range = match cmd.range {
        Some(r) => parse_range(r)?,
        None => None,
    };
    let reel = Reel::new(&cmd.reel_path, &cmd.reel_name, frame_range)?;

    // #### Component init
    let (mut comp_reels, mut comp_reg) = init_components(cmd.component)?;
    comp_reg.single_merge(cut_register);
    comp_reels.push(reel);
    cut_register = comp_reg;

    // add merge_cuts destructively
    read_into(&mut cut_register, cmd.merge_cuts)?;

    run_record(
        RecordRunner {
            duration:  cmd.duration,
            reel_name: cmd.reel_name,
            take_out:  cmd.take_out,
            register:  cut_register,
            frames:    comp_reels.into_iter().flatten().collect(),
        },
        base_params,
    )
}

pub fn cmd_vrecord(cmd: VirtualRecord, mut base_params: BaseParams) -> Result<(), Error> {
    use fr::vreel::*;

    base_params.timeout = cmd.timeout;
    base_params.timestamp = cmd.timestamp;

    let vreel = cmd.init()?;
    let register = match vreel.cut {
        VirtualCut::Register(r) => r,
        VirtualCut::MergeCuts(cuts) if cuts.is_empty() => Register::new(),
        VirtualCut::MergeCuts(cuts) => Register::try_from(cuts)?,
        VirtualCut::Cut(cut) => Register::try_from(cut)?,
    };

    let frames = match vreel.frames {
        VirtualFrames::List(list) => list
            .iter()
            .map(MetaFrame::try_from)
            .collect::<Result<Vec<MetaFrame>, _>>()?,
        VirtualFrames::RenamedList(map) => map
            .iter()
            .map(|(k, v)| -> Result<MetaFrame, Error> {
                let mut frame = MetaFrame::try_from(v)?;
                frame.alt_name = Some(k.to_string());
                Ok(frame)
            })
            .collect::<Result<Vec<MetaFrame>, _>>()?,
    };

    run_record(
        RecordRunner {
            duration: false,
            reel_name: vreel.name.into(),
            take_out: cmd.take_out,
            register,
            frames,
        },
        base_params,
    )
}

/// runs through a [fr::Reel] sequence using the [crate::Record] or [crate::VirtualRecord] structs
pub fn run_record(mut runner: RecordRunner, base_params: BaseParams) -> Result<(), Error> {
    let start = Instant::now();
    let duration = runner.duration;
    let get_duration = || {
        if duration {
            warn!(
                "[Total record duration: {:.3}sec]",
                start.elapsed().as_secs_f32(),
            );
        }
    };

    for meta_frame in runner.frames.into_iter() {
        // if cmd.output is Some, provide a take PathBuf
        let output = runner
            .take_out
            .as_ref()
            .map(|dir| take_output(&dir, &&meta_frame.path));

        let mut info_str = format!("{} {:?}", "File:".yellow(), meta_frame.get_filename());
        if let Some(alt_name) = meta_frame.alt_name {
            info_str = format!("{:45} | {} {}", info_str, "Name:".yellow(), alt_name);
        }
        warn!("{}{}", base_params.fmt_timestamp(), info_str,);
        warn!("{}", "=======================".green());

        let frame = Frame::try_from(meta_frame.path)?;
        // Frame to be mutably borrowed
        let mut payload_frame = frame.clone();

        if let Err(e) = run_take(
            &mut payload_frame,
            &mut runner.register,
            &base_params,
            output,
        ) {
            get_duration();
            write_cut(
                &base_params.cut_out,
                &runner.register,
                &runner.reel_name,
                true,
            )?;
            return Err(e);
        }
    }
    warn!(
        "{}{}{}{}",
        base_params.fmt_timestamp(),
        "= ".green(),
        "Success 🎉 ".yellow(),
        "==========\n".green()
    );
    get_duration();

    write_cut(
        &base_params.cut_out,
        &runner.register,
        &runner.reel_name,
        false,
    )?;

    Ok(())
}

// merge any found [PathBuf]s into the cut register destructively
pub fn read_into(base_register: &mut Register, merge_cuts: Vec<String>) -> Result<(), Error> {
    let mut err = Ok(());
    // Merge any found PathBufs into the cut register destructively
    let merge_registers: Vec<Register> = merge_cuts
        .into_iter()
        .map(|c| {
            // if we're passing a json string such as '{"key": "value"}'
            if guess_json_obj(&c) {
                return Ok(c);
            }
            fr::file_to_string(&c).map_err(|e| anyhow!("{} - {}", c, e))
        })
        .scan(&mut err, filmreel::until_err)
        .map(Register::from)
        .collect::<Result<Vec<Register>, _>>()?;
    // TODO tidy up scan calling only on file_to_string errors
    err?;

    base_register.destructive_merge(merge_registers);

    Ok(())
}

/// write_cut dumps the in memory [Regiser] to the [PathBuf] provided.
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
        comp_reg.single_merge(register);
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
        Register::try_from(cut_path.clone()).context(format!(
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
            if start.is_empty() {
                // make end string range inclusive
                return Ok(Some(0..end_parse()? + 1));
            }
            if end.is_empty() {
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
