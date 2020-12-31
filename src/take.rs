use crate::{
    grpc, http,
    params::{BaseParams, Params},
    record::write_cut,
    Take, ToStringPretty, ToTakeColouredJson, ToTakeHiddenColouredJson,
};
use anyhow::{anyhow, Context, Error};
use colored::*;
use colored_diff::PrettyDifference;
use filmreel as fr;
use filmreel::{
    cut::Register,
    frame::{Frame, Protocol, Response},
    reel::MetaFrame,
};
use log::{debug, error, info, warn};
use prettytable::*;
use std::{
    convert::TryFrom,
    fs,
    io::{self, prelude::*},
    path::PathBuf,
    thread, time,
};

// run_request decides which protocol to use for sending a hydrated Frame Request
pub fn run_request<'a>(params: &Params, frame: &'a mut Frame) -> Result<Response, Error> {
    let request_fn = match frame.protocol {
        Protocol::HTTP => http::request,
        Protocol::GRPC => grpc::request,
    };
    request_fn(params.clone(), frame.get_request())
}

// process_response grabs the expected Response from the given Frame and attempts to match the values
// present in the payload Response printing a "Value Mismatch" diff to stdout and returning an
// error if there is not a complete match
pub fn process_response<'a>(
    params: Params,
    frame: &'a mut Frame,
    cut_register: &'a mut Register,
    payload_response: Response,
    output: Option<PathBuf>,
) -> Result<&'a Register, Error> {
    let payload_matches = frame
        .response
        .match_payload_response(&frame.cut, &payload_response)
        .map_err(Error::from)
        .or_else(|e| {
            log_mismatch(&params, &frame.response, &payload_response)
                .context("fn log_mismatch failure")?;
            Err(e)
        })?;

    // If there are valid matches for write operations
    if let Some(matches) = payload_matches {
        debug!("writing to cut register...");
        for (k, v) in matches {
            cut_register.write_operation(k, v)?;
        }

        // For now simply run hydrate again to hydrate the newly written cut variables into the
        // Response
        frame.cut.hydrate_writes = true;

        if let Some(response_body) = &mut frame.response.body {
            Frame::hydrate_val(&frame.cut, response_body, &cut_register, false)?;
        }
        Frame::hydrate_val(&frame.cut, &mut frame.response.etc, &cut_register, false)?;
    }

    if frame.response != payload_response {
        params.error_timestamp();
        error!(
            "{}",
            PrettyDifference {
                expected: &frame.response.to_string_pretty()?,
                actual: &payload_response.to_string_pretty()?,
            }
        );
        error!(
            "{}{}{}",
            "= ".red(),
            "Value Mismatch 🤷".yellow(),
            "===".red()
        );
        return Err(anyhow!("request/response mismatch"));
    }

    // remove lowercase values
    cut_register.flush_ignored();

    info!(
        "{}{}{}",
        "= ".green(),
        "Match 👍 ".yellow(),
        "============\n".green()
    );

    // If an output was specified create a take file
    if let Some(frame_out) = output {
        debug!("creating take receipt...");
        fs::write(frame_out, frame.to_string_pretty()?)?;
    }

    Ok(cut_register)
}

/// run_take
/// 1. initializes cli settings for the take using base_params
/// 2. performs a single frame hydration using a given json file
/// 3. initializes frame specific settings for the take using base_params.init(frame.get_request())
/// 4. runs a request and processes the response, multiple times if attempts are present in the Params object
/// 5. Outputs a diff to stdout and returns an error if there is a mismatch:
///    - Form Mismatch: output during run_request when the returned JSON does not match the
///     expected structure
///    - Value Mismatch: output during process_response when the returned JSON values do not
///    match
pub fn run_take(
    frame: &mut Frame,
    register: &mut Register,
    base_params: &BaseParams,
    output: Option<PathBuf>,
) -> Result<(), Error> {
    let interactive = base_params.interactive;
    let verbose = base_params.verbose;
    let mut unhydrated_frame: Option<Frame> = None;
    // hidden_frame is meant to sanitize ${_HIDDEN} variables
    let hidden_frame: Option<Frame> = if interactive || verbose {
        unhydrated_frame = Some(frame.clone());
        let mut hydrated = frame.clone();
        hydrated.hydrate(&register, true)?;
        Some(hydrated)
    } else {
        None
    };

    info!("[{}] frame:", "Unhydrated".red());
    info!("{}", frame.to_coloured_tk_json()?);
    info!("{}", "=======================".magenta());
    info!("HYDRATING...");
    info!("{}", "=======================".magenta());
    frame.hydrate(&register, false)?;
    // init params after hydration so that  cut register params can be pulled otherwise this can
    // happen: Params { address: "${ADDRESS}", }
    let params = base_params.init(frame.get_request())?;

    if interactive {
        let mut stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut table = Table::new();
        table.add_row(row![
            format!("[{}] frame", "Unhydrated".red()),
            format!("[{}]", "Cut Register".yellow()),
            format!("[{}] frame", "Hydrated".green()),
        ]);

        let hidden = match hidden_frame {
            Some(f) => f,
            None => return Err(anyhow!("None for interactive hidden_frame")),
        };
        table.add_row(row![
            unhydrated_frame
                .expect("None for unhydrated_frame")
                .to_coloured_tk_json()?,
            register.to_hidden_tk_json()?,
            hidden.to_coloured_tk_json()?,
        ]);
        table.printstd();
        write!(
            stdout,
            "{}",
            format!("Press {} to continue...", "ENTER".yellow())
        )
        .expect("write to stdout panic");
        stdout.flush().expect("stdout flush panic");

        // Read a single byte and discard
        let _ = stdin.read(&mut [0u8]).expect("read stdin panic");
    } else if verbose {
        let hidden = match hidden_frame {
            Some(f) => f,
            None => return Err(anyhow!("None for interactive hidden_frame")),
        };
        info!("{} {}", "Request URI:".yellow(), frame.get_request_uri()?);
        info!("[{}] frame:", "Hydrated".green());
        info!("{}", hidden.to_coloured_tk_json()?);
    }

    if let Some(attempts) = params.attempts {
        for n in 1..attempts.times {
            warn!(
                "attempt [{}/{}] | interval [{}{}]",
                n.to_string().yellow(),
                attempts.times,
                attempts.ms.to_string().yellow(),
                "ms",
            );
            if let Ok(response) = run_request(&params, frame) {
                if process_response(params.clone(), frame, register, response, output.clone())
                    .is_ok()
                {
                    return Ok(());
                }
            }
            thread::sleep(time::Duration::from_millis(attempts.ms));
        }
        // for final retry attempt do not swallow error propagation
        warn!(
            "attempt [{}/{}]",
            attempts.times.to_string().red(),
            attempts.times
        );
    }

    let response = run_request(&params, frame)?;
    match process_response(params, frame, register, response, output) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

/// single_take runs a single take using the darkroom::Take struct
pub fn single_take(cmd: Take, base_params: BaseParams) -> Result<(), Error> {
    let frame_str = fr::file_to_string(&cmd.frame)?;
    let cut_str = fr::file_to_string(&cmd.cut)?;
    let get_metaframe = || MetaFrame::try_from(cmd.frame.clone());

    // Frame to be mutably borrowed
    let frame = Frame::new(&frame_str).context(get_metaframe()?.get_filename())?;
    let mut payload_frame = frame.clone();
    let mut cut_register = Register::from(&cut_str)?;
    if let Err(e) = run_take(
        &mut payload_frame,
        &mut cut_register,
        &base_params,
        cmd.take_out.clone(),
    ) {
        write_cut(
            &base_params.cut_out,
            &cut_register,
            get_metaframe()?.reel_name,
            true,
        )?;
        return Err(e);
    }

    write_cut(
        &base_params.cut_out,
        &cut_register,
        get_metaframe()?.reel_name,
        false,
    )?;

    Ok(())
}

// log_mismatch provides the "Form Mismatch" diff when the returned payload Response does not match
// the expected object structure of the Frame Response
fn log_mismatch(
    params: &Params,
    frame_response: &Response,
    payload_response: &Response,
) -> Result<(), Error> {
    params.error_timestamp();
    error!("{}\n", "Expected:".magenta());
    error!(
        "{}\n",
        frame_response
            .to_coloured_tk_json()
            .context("log_mismatch \"Expected:\" serialization")?
    );
    error!("{}\n", "Actual:".magenta());
    error!(
        "{}\n",
        payload_response
            .to_coloured_tk_json()
            .context("log_mismatch \"Actual:\"  serialization")?
    );
    error!(
        "{}{}{}",
        "= ".red(),
        "Form Mismatch 🌋 ".yellow(),
        "====".red()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use filmreel::{cut::Register, frame::Response, register};
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
            body: Some(json!("created user: BIG_BEN")),
            etc: json!({}),
            status: 200,
        };
        let mut register = Register::default();
        let params = Params::default();
        let processed_register =
            process_response(params, &mut frame, &mut register, payload_response, None).unwrap();
        assert_eq!(*processed_register, register!({"USER_ID"=>"BIG_BEN"}));
    }
}
