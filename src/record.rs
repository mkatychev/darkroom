use crate::{BoxError, Record, Take};
use filmreel as fr;
use filmreel::cut::Register;
use filmreel::frame::{Frame, Response};
use filmreel::reel::*;

pub fn run_record(cmd: Record) -> Result<(), BoxError> {
    dbg!(cmd.get_cut_file());
    dbg!(cmd.get_cut_copy());
    let cut_str = fr::file_to_string(cmd.get_cut_file())?;
    let mut cut_register = Register::new(&cut_str)?;
    let reel = Reel::new(cmd.path, &cmd.name)?;
    for meta_frame in reel {
        let frame_str = fr::file_to_string(&meta_frame.path)?;
        let mut frame = Frame::new(&frame_str)?;
    }
    Ok(())
}
