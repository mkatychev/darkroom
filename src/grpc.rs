use crate::{BoxError, Take};
use filmreel::frame::{Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_yaml;
use serde_yaml::Error;
use std::convert::TryFrom;
use std::io::ErrorKind;
use std::process::Command;

/// Checks to see if grpcurl is in the system path
pub fn validate_grpcurl() -> Result<(), BoxError> {
    if let Err(e) = Command::new("grpcurl").spawn() {
        if let ErrorKind::NotFound = e.kind() {
            return Err("`grpcurl` was not found! Check your PATH!".into());
        } else {
            return Err(e.into());
        }
    }
    Ok(())
}

/// Parameters needed for a uri method to be sent.
pub struct Params<'a> {
    tls: bool,
    header: &'a String,
    address: &'a String,
}

impl<'a> From<&'a Take> for Params<'a> {
    fn from(take: &'a Take) -> Self {
        Self {
            // TODO handle tls
            tls: false,
            header: &take.header,
            address: &take.addr,
        }
    }
}

/// Parses a Frame Request and a Params object to send a gRPC payload using grpcurl
pub fn grpcurl(prm: &Params, req: &Request) -> Result<Response, BoxError> {
    let tls = match prm.tls {
        true => "",
        false => "-plaintext",
    };

    let req_cmd = Command::new("grpcurl")
        .arg("-H")
        .arg(prm.header)
        .arg(tls)
        .arg("-plaintext")
        .arg("-d")
        .arg(req.to_payload()?)
        .arg(prm.address)
        .arg(req.uri())
        .output()?;

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
        match serde_yaml::from_slice::<ResponseError>(stderr) {
            Err(_) => return Err(String::from_utf8(stderr.clone())?.into()),
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

        // deserialize a nested yaml object by casing it to an inner struct first
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
            _ => return Err(D::Error::custom("unexpected gRPC error code"))?,
        };
        Ok(ResponseError {
            code,
            message: outer.ERROR.Message,
        })
    }
}

#[cfg(test)]
mod serde_tests {
    use super::*;
    use serde_yaml;

    const YAML_ERROR: &str = r#"
        ERROR:
            Code: Internal
            Message: input cannot be empty"#;

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
}
