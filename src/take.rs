use crate::grpc::{grpcurl, Params};
use crate::{BoxError, Take};
use colored::*;
use colored_diff::PrettyDifference;
use colored_json::prelude::*;
use filmreel as fr;
use filmreel::cut::Register;
use filmreel::frame::{Frame, Response};
use log::{debug, error, info};
use prettytable::*;
use std::fs;
use std::io::{self, prelude::*};
use std::path::PathBuf;

/// Performs a single frame hydration using a given json file and outputs a Take to either stdout
/// or a designated file
pub fn run_request<'a>(
    frame: &'a mut Frame,
    register: &'a Register,
    params: &Params,
    interactive: bool,
) -> Result<Response, BoxError> {
    let unhydrated_frame: Option<String> = if interactive {
        Some(frame.to_string_pretty())
    } else {
        info!("[{}] frame:", "Unhydrated".red());
        info!("{}", frame.to_string_pretty().to_colored_json_auto()?);
        info!("{}", "=======================".magenta());
        info!("HYDRATING...");
        info!("{}", "=======================".magenta());
        None
    };

    frame.hydrate(&register)?;

    if interactive {
        let mut stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut table = Table::new();
        table.add_row(row![
            format!("[{}] frame", "Unhydrated".red()),
            "Cut Register",
            format!("[{}] frame", "Hydrated".green()),
        ]);
        table.add_row(row![
            unhydrated_frame.unwrap().to_colored_json_auto()?,
            register.to_string_pretty().to_colored_json_auto()?,
            frame.to_string_pretty().to_colored_json_auto()?,
        ]);
        table.printstd();
        write!(
            stdout,
            "{}",
            format!("Press {} to continue...", "ENTER".yellow())
        )
        .expect("write to stdout panic");
        stdout.flush().expect("stoud flush panic");

        // Read a single byte and discard
        let _ = stdin.read(&mut [0u8]).expect("read stdin panic");
    } else {
        info!("[{}] frame:", "Hydrated".green());
        info!("{}", frame.to_string_pretty().to_colored_json_auto()?);
        info!("\n");
        info!("{} {}", "Request URI:".yellow(), frame.get_request_uri()?);
        info!("{}", "=======================".magenta());
        info!("\n");
    }

    // Send out the payload here
    grpcurl(params, frame.get_request())
}

pub fn process_response<'a>(
    frame: &'a mut Frame,
    cut_register: &'a mut Register,
    payload_response: Response,
    cut_out: Option<&PathBuf>,
    output: Option<PathBuf>,
) -> Result<&'a Register, BoxError> {
    let payload_matches = match frame
        .response
        .match_payload_response(&frame.cut, &payload_response)
    {
        Err(e) => {
            log_mismatch(
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
    }

    if frame.response != payload_response {
        error!(
            "{}",
            PrettyDifference {
                expected: &frame.response.to_string_pretty(),
                actual: &payload_response.to_string_pretty(),
            }
        );
        return Err("request/response mismatch".into());
    }

    // Error expected actual
    if let Some(cut_out) = cut_out {
        debug!("writing to cut file...");
        fs::write(cut_out, &cut_register.to_string_pretty())?;
    }

    // If an output was specified create a take file
    if let Some(frame_out) = output {
        debug!("creating take recepit...");
        fs::write(frame_out, frame.to_string_pretty())?;
    }
    if payload_response != frame.response {
        error!("OK");
    }

    Ok(cut_register)
}

/// Run single take using the darkroom::Take struct
pub fn single_take(cmd: Take) -> Result<(), BoxError> {
    let frame_str = fr::file_to_string(&cmd.frame)?;
    let cut_str = fr::file_to_string(&cmd.cut)?;

    let mut frame = Frame::new(&frame_str)?;
    let mut cut_register = Register::new(&cut_str)?;
    let response = run_request(&mut frame, &cut_register, &Params::from(&cmd), false)?;

    process_response(
        &mut frame,
        &mut cut_register,
        response,
        Some(&cmd.cut),
        cmd.output.clone(),
    )?;
    Ok(())
}

fn log_mismatch(frame_str: String, response_str: String) {
    error!("{}\n", "Expected:".magenta());
    error!(
        "{}\n",
        frame_str
            .to_colored_json_auto()
            .expect("log_mismatch expected panic")
    );
    error!("{}\n", "Actual:".magenta());
    error!(
        "{}\n",
        response_str
            .to_colored_json_auto()
            .expect("log_mismatch actual panic")
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use filmreel::cut::Register;
    use filmreel::frame::Response;
    use filmreel::register;
    use serde_json::{self, json};

    #[test]
    fn test_process_response() {
        let mut frame: Frame = serde_json::from_str(
            r#"
{
  "protocol": "HTTP",
  "cut": {
    "to": {
      "USER_ID": "'response'.'body'"
    }
  },
  "request": {
    "body": {},
    "uri": ""
  },
  "response": {
    "body": "created user: ${USER_ID}",
    "status": 200
  }
}
    "#,
        )
        .unwrap();
        let payload_response = Response {
            body: json!("created user: BIG_BEN"),
            etc: json!({}),
            status: 200,
        };
        let mut register = Register::default();
        let processed_register =
            process_response(&mut frame, &mut register, payload_response, None, None).unwrap();
        assert_eq!(*processed_register, register!({"USER_ID"=>"BIG_BEN"}));
    }
}
