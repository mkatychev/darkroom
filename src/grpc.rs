use crate::params::{iter_path_args, Params};
use anyhow::{anyhow, Context, Error};
use filmreel::{frame::Request, response::Response};
use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::json;
use std::{ffi::OsStr, path::PathBuf, process::Command};

/// Checks to see if grpcurl is in the system path
pub fn validate_grpcurl() -> Result<(), Error> {
    lazy_static! {
        static ref GRPCURL: which::Result<PathBuf> = which::which("grpcurl");
    }
    if !GRPCURL.is_ok() {
        return Err(anyhow!("`grpcurl` was not found! Check your PATH!"));
    }
    Ok(())
}

/// request parses a Frame Request and a Params object to send a gRPC payload using `grpcurl`
/// the command line tool
pub fn request<'a>(prm: &'a Params, req: Request) -> Result<Response<'a>, Error> {
    validate_grpcurl().context("grpcurl request failure")?;

    let mut flags: Vec<&OsStr> = vec![OsStr::new("-format-error")];

    if !prm.tls {
        flags.push(OsStr::new("-plaintext"));
    }

    // prepend "-import-path" to every protos PathBuf provided
    if let Some(proto_path) = prm.proto_path {
        flags.extend(iter_path_args(
            OsStr::new("-import-path"),
            proto_path.iter().map(OsStr::new),
        ));
    }

    // prepend "-proto" to every protos PathBuf provided
    if let Some(protos) = prm.proto {
        flags.extend(iter_path_args(
            OsStr::new("-proto"),
            protos.iter().map(OsStr::new),
        ));
    }

    let headers = match &prm.header {
        Some(h) => Some(h.replace("\"", "")),
        None => None,
    };

    if let Some(h) = &headers {
        flags.push(OsStr::new("-H"));
        flags.push(OsStr::new(h));
    }

    let req_cmd = Command::new("grpcurl")
        .args(flags)
        .arg("-connect-timeout")
        .arg(format!("{:.1}", prm.timeout as f32))
        .arg("-d")
        .arg(req.to_payload()?)
        .arg(&prm.address)
        .arg(req.get_uri())
        .output()
        .context("failed to execute grpcurl process")?;

    let response = match req_cmd.status.code() {
        Some(0) => Response {
            body:       serde_json::from_slice(&req_cmd.stdout)?,
            status:     0,
            etc:        Some(json!({})),
            validation: None,
        },
        Some(_) => {
            let err: ResponseError = serde_json::from_slice(&req_cmd.stderr).map_err(|_| {
                // if we fail to map to a serde struct, stringingfy stderr bytes and cast to anyhow error
                String::from_utf8(req_cmd.stderr)
                    .map_err(Error::from)
                    .map(|v| anyhow!(v))
                    .context("grpcurl error")
                    .unwrap_or_else(|e| e)
            })?;
            // create frame response from deserialized grpcurl error
            Response {
                body:       Some(serde_json::Value::String(err.message)),
                status:     err.code,
                etc:        Some(json!({})),
                validation: None,
            }
        }
        None => return Err(anyhow!("grpcurl response code was <None>")),
    };
    Ok(response)
}

#[derive(Debug, Deserialize, PartialEq)]
struct ResponseError {
    code:    u32,
    message: String,
}

#[cfg(test)]
mod serde_tests {
    use super::*;
    use serde_json;

    const INTERNAL_ERROR: &str = r#"{
  "code": 13,
  "message": "input cannot be empty"
}"#;
    const AUTH_ERROR: &str = r#"{
  "code": 16,
  "message": "rpc error: code = Unauthenticated desc = Empty JWT token"
}"#;

    #[test]
    fn test_internal() {
        let json_struct: ResponseError = serde_json::from_str(INTERNAL_ERROR).unwrap();
        assert_eq!(
            ResponseError {
                code:    13,
                message: "input cannot be empty".to_owned(),
            },
            json_struct
        );
    }

    #[test]
    fn test_auth() {
        let json_struct: ResponseError =
            serde_json::from_slice(&AUTH_ERROR.as_bytes().to_vec()).unwrap();
        assert_eq!(
            ResponseError {
                code:    16,
                message: "rpc error: code = Unauthenticated desc = Empty JWT token".to_owned(),
            },
            json_struct
        );
    }
}
