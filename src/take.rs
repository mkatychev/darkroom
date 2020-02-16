use crate::grpc::{grpcurl, Params};
use crate::{BoxError, Take};
use colored::*;
use filmreel as fr;
use filmreel::cut::Register;
use filmreel::frame::{Frame, Response};
use log::{debug, error, info};
use std::fs;
use std::path::PathBuf;

/// Performs a single frame hydration using a given json file and outputs a Take to either stdout
/// or a designated file
pub fn run_take<'a>(
    frame: &'a mut Frame,
    cut_register: &'a mut Register<'a>,
    params: &Params,
    cut_out: Option<&PathBuf>,
    output: Option<PathBuf>,
) -> Result<(), BoxError> {
    info!("{} frame JSON:", "Unhydrated".red());
    info!("{}", frame.to_string_pretty());
    info!("{}", "=======================".magenta());
    info!("HYDRATING...");
    info!("{}", "=======================".magenta());

    frame.hydrate(&cut_register)?;

    info!("{} frame JSON:", "Hydrated".green());
    info!("{}", frame.to_string_pretty());
    info!("{} {}", "Request URI:".yellow(), frame.get_request_uri()?);

    frame.hydrate(&cut_register)?;
    // Send out the payload here
    let payload_response: Response = grpcurl(params, frame.get_request())?;

    let payload_matches = match frame
        .response
        .match_payload_response(&frame.cut, &payload_response)
    {
        Err(e) => {
            unexpected_response(
                frame.response.to_string_pretty(),
                payload_response.to_string_pretty(),
            );
            return Err(e);
        }
        Ok(r) => r,
    };

    // If there are valid matches for write operations
    if let Some(matches) = payload_matches {
        debug!("writing to cut register...");
        for (k, v) in matches {
            cut_register.write_operation(k, v)?;
        }

        // For now simply run hydrate again to hydrate the newly written cut variables into the
        // Response
        Frame::hydrate_val(&frame.cut, &mut frame.response.body, &cut_register)?;
        Frame::hydrate_val(&frame.cut, &mut frame.response.etc, &cut_register)?;

        // Error expected actual
        if let Some(cut_out) = cut_out {
            debug!("writing to cut file...");
            fs::write(cut_out, &cut_register.to_string_pretty())?;
        }
    }

    // If an output was specified create a take file
    if let Some(frame_out) = output {
        debug!("creating take recepit...");
        fs::write(frame_out, frame.to_string_pretty())?;
    }
    if payload_response != frame.response {
        error!("OK");
    }

    Ok(())
}

/// Run single take using the darkroom::Take struct
pub fn single_take(cmd: Take) -> Result<(), BoxError> {
    let frame_str = fr::file_to_string(&cmd.frame)?;
    let cut_str = fr::file_to_string(&cmd.cut)?;

    let mut frame = Frame::new(&frame_str)?;
    let mut cut_register = Register::new(&cut_str)?;
    run_take(
        &mut frame,
        &mut cut_register,
        &Params::from(&cmd),
        Some(&cmd.cut),
        cmd.output.clone(),
    )
}

fn unexpected_response(frame_str: String, response_str: String) {
    error!("{}\n", "Expected:".magenta());
    error!("{}\n", frame_str);
    error!("{}\n", "Actual:".magenta());
    error!("{}\n", response_str);
}
