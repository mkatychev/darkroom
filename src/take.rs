use crate::vprintln;
use crate::{grpc::*, BoxError, Opts, Take};
use colored::*;
use filmreel as fr;
use filmreel::cut::Register;
use filmreel::frame::Frame;
use std::fs;

/// Performs a single frame hydration using a given json file and outputs a Take to either stdout
/// or a designated file
pub fn run_take(cmd: Take, opts: Opts) -> Result<(), BoxError> {
    let v = opts.verbose;
    let frame_str = fr::file_to_string(&cmd.frame)?;
    let cut_str = fr::file_to_string(&cmd.cut)?;

    let mut frame = Frame::new(&frame_str)?;

    let cut_register = Register::new(&cut_str)?;

    vprintln!(v, "{}", "Unhydrated frame JSON:".red());
    vprintln!(v, "{}", frame.to_string_pretty());
    vprintln!(v, "{}", "=======================".magenta());
    vprintln!(v, "HYDRATING...");
    vprintln!(v, "{}", "=======================".magenta());

    frame.hydrate(&cut_register)?;

    vprintln!(v, "{}", "Hydrated frame JSON:".green());
    vprintln!(v, "{}", frame.to_string_pretty());
    vprintln!(
        v,
        "{} {}",
        "Request URI:".yellow(),
        frame.get_request_uri()?
    );
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
