use filmreel as fr;
use filmreel::cut::Register;
use filmreel::frame::Frame;
use std::path::PathBuf;
use std::process::exit;

/// Performs a single frame hydration using a given json file and outputs a Take to either stdout
/// or a designated file
pub fn run_take(frame: PathBuf, cut: PathBuf, frame_out: Option<PathBuf>) {
    let frame_str = fr::file_to_string(&frame).expect("frame error");

    let mut frame = Frame::new(&frame_str).unwrap_or_else(|err| {
        eprintln!("Frame {}", &frame.to_str().unwrap());
        eprintln!("{}", err);
        exit(1);
    });

    let cut_str = fr::file_to_string(cut).expect("cut error");

    let cut_register = Register::new(&cut_str).unwrap_or_else(|err| {
        eprintln!("Cut Register {}", err);
        exit(1);
    });

    println!("{}", frame.to_string_pretty());

    frame.hydrate(&cut_register).unwrap_or_else(|err| {
        eprintln!("Frame {}", err);
        exit(1);
    });

    if let Some(frame_out) = frame_out {
        std::fs::write(frame_out, frame.to_string_pretty()).expect("Unable to write file");
    }

    println!("{}", frame.to_string_pretty());
}
