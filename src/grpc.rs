use crate::params::{iter_path_args, Params};
use anyhow::{anyhow, Context, Error};
use filmreel::frame::{Request, Response};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
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

/// Parses a Frame Request and a Params object to send a gRPC payload using grpcurl
pub fn grpcurl(prm: Params, req: Request) -> Result<Response, Error> {
    validate_grpcurl().context("grpcurl request failure")?;

    let mut flags: Vec<&OsStr> = vec![OsStr::new("-format-error")];

    if !prm.tls {
        flags.push(OsStr::new("-plaintext"));
    }
    // prepend "-proto" to every protos PathBuf provided
    if let Some(protos) = prm.proto {
        flags.extend(iter_path_args(
            OsStr::new("-proto"),
            protos.iter().map(|x| x.as_ref()),
        ));
    }

    let headers = match prm.header {
        Some(h) => Some(h.replace("\"", "")),
        None => None,
    };

    if let Some(h) = headers.as_ref() {
        flags.push(OsStr::new("-H"));
        flags.push(h.as_ref());
    }

    let req_cmd = Command::new("grpcurl")
        .args(flags)
        .arg("-d")
        .arg(req.to_payload()?)
        .arg(prm.address)
        .arg(req.get_uri())
        .output()
        .context("grpcurl error")?;

    let response: Response = match req_cmd.status.code() {
        Some(0) => Response {
            body: serde_json::from_slice(&req_cmd.stdout)?,
            status: 0,
            etc: json!({}),
        },
        Some(_) => {
            let err: ResponseError = serde_json::from_slice(&req_cmd.stderr)?;
            // create frame response from deserialized grpcurl error
            Response {
                body: serde_json::Value::String(err.message),
                status: err.code,
                etc: json!({}),
            }
        }
        None => return Err(anyhow!("grpcurl Response code was <None>")),
    };
    Ok(response)
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct ResponseError {
    code: u32,
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
                code: 13,
                message: "input cannot be empty".to_owned()
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
                code: 16,
                message: "rpc error: code = Unauthenticated desc = Empty JWT token".to_owned()
            },
            json_struct
        );
    }
}
