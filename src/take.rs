use crate::grpc::{grpcurl, Params};
use crate::{BoxError, Take};
use colored::*;
use filmreel as fr;
use filmreel::cut::Register;
use filmreel::frame::Frame;
use log::info;
use std::fs;

/// Performs a single frame hydration using a given json file and outputs a Take to either stdout
/// or a designated file
pub fn run_take(cmd: Take) -> Result<(), BoxError> {
    let frame_str = fr::file_to_string(&cmd.frame)?;
    let cut_str = fr::file_to_string(&cmd.cut)?;

    let mut frame = Frame::new(&frame_str)?;

    let cut_register = Register::new(&cut_str)?;

    info!("this is info!");
    info!("{} frame JSON:", "Unhydrated".red());
    info!("{}", frame.to_string_pretty());
    info!("{}", "=======================".magenta());
    info!("HYDRATING...");
    info!("{}", "=======================".magenta());

    frame.hydrate(&cut_register)?;

    info!("{} frame JSON:", "Hydrated".green());
    info!("{}", frame.to_string_pretty());
    info!("{} {}", "Request URI:".yellow(), frame.get_request_uri()?);

    // Send out the payload here
    let payload_response = grpcurl(&Params::from(&cmd), frame.get_request())?;

    // If there are valid matches for write operations
    if let Some(matches) = frame.match_payload_response(&payload_response)? {
        let mut out_register = cut_register.clone();
        for (k, v) in matches {
            out_register.write_operation(k, v)?;
        }
        fs::write(&cmd.cut.clone(), &out_register.to_string_pretty())?;
    }

    // If an output was specified create a take file
    if let Some(frame_out) = cmd.output {
        fs::write(frame_out, frame.to_string_pretty())?;
    }

    Ok(())
}
