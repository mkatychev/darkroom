use crate::grpc::*;
use crate::take::*;
use crate::{BoxError, Record};
use colored::*;
use filmreel as fr;
use filmreel::cut::Register;
use filmreel::frame::Frame;
use filmreel::reel::*;
use log::{debug, warn};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run_record(cmd: Record) -> Result<(), BoxError> {
    dbg!(cmd.get_cut_file());
    let cut_str = fr::file_to_string(cmd.get_cut_file())?;
    let mut cut_register = Register::new(&cut_str)?;
    let reel = Reel::new(&cmd.path, &cmd.name)?;
    for meta_frame in reel {
        let output = cmd
            .output
            .as_ref()
            .map(|dir| take_output(&dir, &&meta_frame.path));
        warn!(
            "{} {:?}",
            "File:".yellow(),
            meta_frame
                .path
                .file_stem()
                .expect("unable to unwrap MetaFrame.path")
        );
        warn!("{}", "=======================".green());

        let frame_str = fr::file_to_string(&meta_frame.path)?;
        let mut frame = Frame::new(&frame_str)?;
        let payload_response = run_request(
            &mut frame,
            &cut_register,
            &Params::from(&cmd),
            cmd.interactive,
        )?;
        process_response(
            &mut frame,
            &mut cut_register,
            payload_response,
            None,
            output,
        )?;
    }

    debug!("writing to cut file...");
    fs::write(cmd.get_cut_copy(), &cut_register.to_string_pretty())?;

    Ok(())
}

/// Grabs a Record command's output directory and joins it with a MetaFrame's file stem
pub fn take_output<P: AsRef<Path>>(dir: &P, file: &P) -> PathBuf {
    let frame_stem: &str = file
        .as_ref()
        .file_stem()
        .and_then(|f| f.to_str())
        .map(|f| f.trim_end_matches(".fr"))
        .expect("take_output: failed filepath trimming");

    dir.as_ref().join(format!("{}.tk.json", frame_stem))
}
