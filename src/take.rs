use crate::Command as Cmd;
use filmreel as fr;
use filmreel::cut::Register;
use filmreel::frame::Frame;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;

/// Performs a single frame hydration using a given json file and outputs a Take to either stdout
/// or a designated file
pub fn run_take(cmd: Cmd) {
    let frame_str: String;
    let cut_str: String;
    let cut_path: PathBuf;
    let header_str: String;
    let dest_str: String;
    let frame_out: Option<PathBuf>;

    match cmd {
        Cmd::Take {
            frame,
            cut,
            header,
            dest,
            output,
        } => {
            frame_str = fr::file_to_string(frame).expect("frame error");
            cut_path = cut.clone();
            cut_str = fr::file_to_string(cut).expect("cut error");
            header_str = header;
            dest_str = dest;
            frame_out = output;
        }
        _ => {
            panic!("WRONG enum");
        }
    }
    let mut frame = Frame::new(&frame_str).unwrap_or_else(|err| {
        // eprintln!("Frame {}", &frame.to_str().unwrap());
        eprintln!("{}", err);
        exit(1);
    });

    let cut_register = Register::new(&cut_str).unwrap_or_else(|err| {
        eprintln!("Cut Register {}", err);
        exit(1);
    });

    println!("{}", frame.to_string_pretty());

    frame.hydrate(&cut_register).unwrap_or_else(|err| {
        eprintln!("Frame {}", err);
        exit(1);
    });

    dbg!(&header_str);
    dbg!(frame.get_request_uri());
    let response_payload = Command::new("/Users/mkatychev/.go/bin/grpcurl")
        .arg("-H")
        .arg(header_str)
        .arg("-plaintext")
        .arg("-d")
        .arg(frame.get_request())
        .arg(dest_str)
        .arg(frame.get_request_uri())
        .output()
        .expect("No grpcurl");

    dbg!(&response_payload);
    let mut out_register = cut_register.clone();

    let mut response_str = String::new();
    std::io::stdin().read_line(&mut response_str);
    frame.to_cut_register(&mut out_register, &response_str);
    std::fs::write(cut_path, &out_register.to_string_pretty()).expect("Unable to write file");
    if let Some(frame_out) = frame_out {
        std::fs::write(frame_out, frame.to_string_pretty()).expect("Unable to write file");
    }

    println!("{}", frame.to_string_pretty());
}
