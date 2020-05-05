use crate::params::iter_path_args;
use crate::params::Params;
use crate::BoxError;
use filmreel::frame::{Request, Response};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

/// Checks to see if grpcurl is in the system path
pub fn validate_grpcurl() -> Result<(), &'static str> {
    lazy_static! {
        static ref GRPCURL: which::Result<PathBuf> = which::which("grpcurl");
    }
    if !GRPCURL.is_ok() {
        return Err("`grpcurl` was not found! Check your PATH!");
    }
    Ok(())
}

/// Parses a Frame Request and a Params object to send a gRPC payload using grpcurl
pub fn grpcurl(prm: Params, req: Request) -> Result<Response, BoxError> {
    validate_grpcurl()?;

    let mut flags: Vec<&OsStr> = vec![];

    if !prm.tls {
        flags.push(OsStr::new("-plaintext"));
    }
    // prepend "-proto" to every protos PathBuf provided
    let mut proto_args: Vec<&OsStr> = match prm.proto {
        Some(protos) => {
            iter_path_args(OsStr::new("-proto"), protos.iter().map(|x| x.as_ref())).collect()
        }
        None => vec![],
    };
    flags.append(&mut proto_args);

    let headers = match prm.header {
        Some(h) => Some(h.replace("\"", "")),
        None => None,
    };

    if let Some(h) = headers.as_ref() {
        flags.push(OsStr::new("-H"));
        flags.push(h.as_ref());
    }

    let req_cmd = match Command::new("grpcurl")
        .args(flags)
        .arg("-d")
        .arg(req.to_payload()?)
        .arg(prm.address)
        .arg(req.get_uri())
        .output()
    {
        Ok(v) => v,
        Err(e) => return Err(format!("grpcurl error: {}", e).into()),
    };

    let response: Response = match req_cmd.status.code() {
        Some(0) => Response {
            body: serde_json::from_slice(&req_cmd.stdout)?,
            status: 0,
            etc: json!({}),
        },
        Some(_) => {
            let err: ResponseError = ResponseError::try_from(&req_cmd.stderr)?;
            // create frame response from deserialized grpcurl error
            Response {
                body: serde_json::Value::String(err.message),
                status: err.code,
                etc: json!({}),
            }
        }
        None => return Err("None Response code".into()),
    };
    Ok(response)
}

#[derive(Debug, Serialize, PartialEq)]
struct ResponseError {
    code: u32,
    message: String,
}

impl TryFrom<&Vec<u8>> for ResponseError {
    type Error = BoxError;

    fn try_from(stderr: &Vec<u8>) -> Result<ResponseError, Self::Error> {
        let stripped = cram_yaml(stderr);
        match serde_yaml::from_slice::<ResponseError>(&stripped) {
            Err(_) => Err(String::from_utf8(stderr.clone())?.into()),
            Ok(err) => Ok(err),
        }
    }
}

impl<'de> Deserialize<'de> for ResponseError {
    /// Handles string version error codes returned by grpcurl
    /// [gRPC Status codes](https://github.com/grpc/grpc/blob/master/doc/statuscodes.md)
    #[allow(non_snake_case)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::Error;

        // deserialize a nested yaml object by casting it to an inner struct first
        #[derive(Deserialize)]
        struct Outer {
            ERROR: Inner,
        }

        #[derive(Deserialize)]
        struct Inner {
            Code: String,
            Message: String,
        }
        let outer = Outer::deserialize(deserializer)?;
        let code = match outer.ERROR.Code.as_str() {
            "Canceled" => 1,
            "Unknown" => 2,
            "InvalidArgument" => 3,
            "DeadlineExceeded" => 4,
            "NotFound" => 5,
            "AlreadyExists" => 6,
            "PermissionDenied" => 7,
            "ResourceExhausted" => 8,
            "FailedPrecondition" => 9,
            "Aborted" => 10,
            "OutOfRange" => 11,
            "Unimplemented" => 12,
            "Internal" => 13,
            "Unavailable" => 14,
            "DataLoss" => 15,
            "Unauthenticated" => 16,
            _ => return Err(D::Error::custom("unexpected gRPC error code")),
        };
        Ok(ResponseError {
            code,
            message: outer.ERROR.Message,
        })
    }
}

/// Horrible hack to make grpcurl output look like yaml
fn cram_yaml(stderr: &[u8]) -> Vec<u8> {
    let mut clean_vec: Vec<String> = Vec::new();
    for line in std::str::from_utf8(stderr)
        .expect("failed string cast")
        .lines()
    {
        if let Some(col_index) = line.find(':') {
            let (key, val) = line.split_at(col_index);
            let mut clean_val = val.to_string();
            clean_val.retain(|c| c != ':');
            clean_vec.push(format!("{}:{}", key, clean_val))
        } else {
            clean_vec.push(line.to_string());
        }
    }
    clean_vec.join("\n").as_bytes().to_vec()
}

#[cfg(test)]
mod serde_tests {
    use super::*;
    use serde_yaml;

    const YAML_ERROR: &str = r#"
        ERROR:
            Code: Internal
            Message: input cannot be empty"#;
    const AUTH_ERROR: &str = r#"
        ERROR:
            Code: Unauthenticated
            Message: rpc error: code = Unauthenticated desc = Empty JWT token"#;

    #[test]
    fn test_yaml() {
        let yaml_struct: ResponseError = serde_yaml::from_str(YAML_ERROR).unwrap();
        assert_eq!(
            ResponseError {
                code: 13,
                message: "input cannot be empty".to_owned()
            },
            yaml_struct
        );
    }

    #[test]
    fn test_auth() {
        let yaml_struct: ResponseError =
            ResponseError::try_from(&AUTH_ERROR.as_bytes().to_vec()).unwrap();
        assert_eq!(
            ResponseError {
                code: 16,
                message: "rpc error code = Unauthenticated desc = Empty JWT token".to_owned()
            },
            yaml_struct
        );
    }
}
