use filmreel::frame::{Frame, Response};
use std::error::Error;
use std::process::Command;

pub fn validate_grpcurl() -> Result<(), Box<dyn Error>> {
    if let Err(e) = Command::new("grpcurl").spawn() {
        if let ErrorKind::NotFound = e.kind() {
            return Err("`grpcurl` was not found! Check your PATH!".into());
        } else {
            return Err(e.into());
        }
    }
}

pub fn grpcurl() -> Result<Response, Box<dyn Error>> {
    let grpc_response = Command::new("grpcurl")
        .arg("-H")
        .arg(cmd.header)
        .arg("-plaintext")
        .arg("-d")
        .arg(frame.get_request())
        .arg(cmd.addr)
        .arg(grpc_uri)
        .output()
        .expect("No grpcurl");

    let response: Response = match grpc_response.status.code().unwrap() {
        0 => Response {
            body: serde_json::from_slice(&grpc_response.stdout).expect("invalid UTF-8"),
            status: 0,
            etc: json!({}),
        },
        _ => {
            let g_err: GrpcurlError = serde_yaml::from_slice(&grpc_response.stderr)?;
            // create frame response from deserialized grpcurl error
            Response {
                body: serde_json::Value::String(g_err.message),
                status: g_err.code,
                etc: json!({}),
            }
        }
    };
    Ok(Response)
}

#[derive(Debug, Serialize, PartialEq)]
struct GrpcurlError {
    code: u32,
    message: String,
}

impl<'de> Deserialize<'de> for GrpcurlError {
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
        Ok(GrpcurlError {
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
        let yaml_struct: GrpcurlError = serde_yaml::from_str(YAML_ERROR).unwrap();
        assert_eq!(
            GrpcurlError {
                code: 13,
                message: "input cannot be empty".to_owned()
            },
            yaml_struct
        );
    }
}
