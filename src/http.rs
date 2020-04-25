use crate::params::Params;
use crate::BoxError;
use filmreel::frame::{Request, Response};
use http::header::HeaderMap;
use lazy_static::lazy_static;
use reqwest::blocking::*;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::process::Command;
use url::Url;

/// Parses a Frame Request and a Params object to send a HTTP payload using reqwest
pub fn build_request(prm: Params, req: Request) -> Result<RequestBuilder, BoxError> {
    // let (_method: Method, _entrypoint: String, ..) =
    let method: Method;
    let endpoint: Url;

    match req.get_uri().split(" ").collect::<Vec<&str>>().as_slice() {
        [method_str, tail_str] => {
            method = Method::from_bytes(method_str.as_bytes())?;
            endpoint = Url::parse(prm.address.as_str())?.join(tail_str)?;
        }
        _ => return Err("unable to parse request uri field".into()),
    };

    Ok(Client::builder()
        .build()?
        .request(method, endpoint)
        .headers(build_header(&prm.header)?))
}

fn build_header(header: &str) -> Result<HeaderMap, BoxError> {
    let map: HashMap<String, String> = serde_json::from_str(header)?;
    return match HeaderMap::try_from(&map) {
        Ok(m) => Ok(m),
        Err(m) => Err(m.into()),
    };
}

// pub fn http_request(prm: Params, req: Request) -> Result<Response, BoxError> {
//     let tls = if prm.tls { "" } else { "-plaintext" };

//     let req_cmd = Command::new("grpcurl")
//         .arg("-H")
//         .arg(prm.header)
//         .arg(tls)
//         .arg("-d")
//         .arg(req.to_payload()?)
//         .arg(prm.address)
//         .arg(req.get_uri())
//         .output()?;

//     let response: Response = match req_cmd.status.code() {
//         Some(0) => Response {
//             body: serde_json::from_slice(&req_cmd.stdout)?,
//             status: 0,
//             etc: json!({}),
//         },
//         Some(_) => {
//             let err: ResponseError = ResponseError::try_from(&req_cmd.stderr)?;
//             // create frame response from deserialized grpcurl error
//             Response {
//                 body: serde_json::Value::String(err.message),
//                 status: err.code,
//                 etc: json!({}),
//             }
//         }
//         None => return Err("None Response code".into()),
//     };
//     Ok(response)
// }

// #[derive(Debug, Serialize, PartialEq)]
// struct ResponseError {
//     code: u32,
//     message: String,
// }

// impl TryFrom<&Vec<u8>> for ResponseError {
//     type Error = BoxError;

//     fn try_from(stderr: &Vec<u8>) -> Result<ResponseError, Self::Error> {
//         let stripped = cram_yaml(stderr);
//         match serde_yaml::from_slice::<ResponseError>(&stripped) {
//             Err(_) => Err(String::from_utf8(stderr.clone())?.into()),
//             Ok(err) => Ok(err),
//         }
//     }
// }
#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_http_request() {
    //     http_request().unwrap();
    // }
}
