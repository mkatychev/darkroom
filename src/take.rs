use crate::{grpc::grpcurl, http::http_request, params::BaseParams, record::write_cut, Take};
use anyhow::{anyhow, Context, Error};
use colored::*;
use colored_diff::PrettyDifference;
use colored_json::{prelude::*, Colour, Styler};
use filmreel as fr;
use filmreel::{
    cut::Register,
    frame::{Frame, Protocol, Response},
    reel::MetaFrame,
    FrError, ToStringHidden, ToStringPretty,
};
use log::{debug, error, info, warn};
use prettytable::*;
use serde::Serialize;
use std::{
    convert::TryFrom,
    fs,
    io::{self, prelude::*},
    path::PathBuf,
    thread, time,
};

/// get_styler returns the custom syntax values for stdout json
fn get_styler() -> Styler {
    Styler {
        bool_value: Colour::Purple.normal(),
        float_value: Colour::RGB(255, 123, 0).normal(),
        integer_value: Colour::RGB(255, 123, 0).normal(),
        nil_value: Colour::Cyan.normal(),
        string_include_quotation: false,
        ..Default::default()
    }
}

trait ToTakeColouredJson {
    fn to_coloured_tk_json(&self) -> Result<String, FrError>;
}

impl<T> ToTakeColouredJson for T
where
    T: ToStringPretty,
{
    fn to_coloured_tk_json(&self) -> Result<String, FrError> {
        Ok(self
            .to_string_pretty()?
            .to_colored_json_with_styler(ColorMode::default().eval(), get_styler())?)
    }
}

trait ToTakeHiddenColouredJson: ToTakeColouredJson {
    // fn to_colored_json(&self) -> Result<String, FrError>;
    fn to_hidden_tk_json(&self) -> Result<String, FrError>;
}

impl<T> ToTakeHiddenColouredJson for T
where
    T: ToStringHidden + Serialize,
{
    fn to_hidden_tk_json(&self) -> Result<String, FrError> {
        Ok(self
            .to_string_hidden()?
            .to_colored_json_with_styler(ColorMode::default().eval(), get_styler())?)
    }
}

/// Performs a single frame hydration using a given json file and outputs a Take to either stdout
/// or a designated file
pub fn run_request<'a>(
    frame: &'a mut Frame,
    register: &'a Register,
    base_params: &BaseParams,
) -> Result<Response, Error> {
    let interactive = base_params.interactive;
    let verbose = base_params.verbose;

    let mut unhydrated_frame: Option<Frame> = None;
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

    let request_fn = match frame.protocol {
        Protocol::HTTP => http_request,
        Protocol::GRPC => grpcurl,
    };
    let params = base_params.init(frame.get_request())?;
    if let Some(attempts) = params.attempts.clone() {
        for n in 1..attempts.times {
            warn!(
                "attempt [{}/{}] | interval [{}{}]",
                n.to_string().yellow(),
                attempts.times,
                attempts.ms,
                "ms".yellow(),
            );
            let param_attempt = params.clone();
            if let Ok(r) = request_fn(param_attempt, frame.get_request()) {
                return Ok(r);
            }
            thread::sleep(time::Duration::from_millis(attempts.ms));
        }
        // for final retry attempt do not swallow error propagation
        warn!(
            "attempt [{}/{}]",
            attempts.times.to_string().red(),
            attempts.times
        );
        return request_fn(params, frame.get_request());
    }

    request_fn(params, frame.get_request())
}

pub fn process_response<'a>(
    frame: &'a mut Frame,
    cut_register: &'a mut Register,
    payload_response: Response,
    output: Option<PathBuf>,
) -> Result<&'a Register, Error> {
    let payload_matches = match frame
        .response
        .match_payload_response(&frame.cut, &payload_response)
    {
        Err(e) => {
            log_mismatch(
                frame.response.to_string_pretty()?,
                payload_response.to_string_pretty()?,
            )
            .context("fn log_mismatch failure")?;
            return Err(Error::from(e));
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
        frame.cut.hydrate_writes = true;
        Frame::hydrate_val(&frame.cut, &mut frame.response.body, &cut_register, false)?;
        Frame::hydrate_val(&frame.cut, &mut frame.response.etc, &cut_register, false)?;
    }

    if frame.response != payload_response {
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
            "Value Mismatch 🤷‍♀️ ".yellow(),
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

/// Run single take using the darkroom::Take struct
pub fn single_take(cmd: Take, base_params: BaseParams) -> Result<(), Error> {
    let frame_str = fr::file_to_string(&cmd.frame)?;
    let cut_str = fr::file_to_string(&cmd.cut)?;
    let get_metaframe = || MetaFrame::try_from(cmd.frame.clone());

    // Frame to be mutably borrowed
    let frame = Frame::new(&frame_str).context(
        get_metaframe()?
            .get_filename()
            .expect("MetaFrame.get_filename() panic"),
    )?;
    let mut payload_frame = frame.clone();
    let mut cut_register = Register::from(&cut_str)?;
    let payload_response = run_request(&mut payload_frame, &cut_register, &base_params)?;

    if let Err(e) = process_response(
        &mut payload_frame,
        &mut cut_register,
        payload_response,
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

fn log_mismatch(frame_str: String, response_str: String) -> Result<(), Error> {
    error!("{}\n", "Expected:".magenta());
    dbg!(&frame_str);
    error!(
        "{}\n",
        frame_str
            .to_colored_json_with_styler(ColorMode::default().eval(), get_styler())
            .context("log_mismatch \"Expected:\" serialization")?
    );
    error!("{}\n", "Actual:".magenta());
    error!(
        "{}\n",
        response_str
            .to_colored_json_with_styler(ColorMode::default().eval(), get_styler())
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
            body: json!("created user: BIG_BEN"),
            etc: json!({}),
            status: 200,
        };
        let mut register = Register::default();
        let processed_register =
            process_response(&mut frame, &mut register, payload_response, None).unwrap();
        assert_eq!(*processed_register, register!({"USER_ID"=>"BIG_BEN"}));
    }
}
