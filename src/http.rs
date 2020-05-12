use crate::params::Params;
use anyhow::{anyhow, Context, Error};
use filmreel::frame::{Request, Response};
use http::header::HeaderMap;
use reqwest::blocking::*;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::TryFrom;
use url::Url;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
struct Form {
    #[serde(flatten)]
    form: BTreeMap<String, Value>,
}

/// Parses a Frame Request and a Params object to send a HTTP payload using reqwest
pub fn build_request(prm: Params, req: Request) -> Result<RequestBuilder, Error> {
    // let (_method: Method, _entrypoint: String, ..) =
    let method: Method;
    let endpoint: Url;

    match &req
        .get_uri()
        .splitn(2, " ")
        .collect::<Vec<&str>>()
        .as_slice()
    {
        [method_str, tail_str] => {
            method = Method::from_bytes(method_str.as_bytes())?;
            endpoint = Url::parse(prm.address.as_str())?.join(tail_str)?;
        }
        _ => {
            return Err(anyhow!("unable to parse request uri field"));
        }
    };

    let mut builder = Client::builder().build()?.request(method, endpoint);
    match req.to_val_payload() {
        Ok(b) => {
            // TODO handle empty body better
            if b != json!({}) {
                builder = builder.body(b.to_string());
            }
        }
        Err(e) => return Err(Error::from(e)),
    }

    let form = serde_json::from_value(req.get_etc()["form"].clone())?;
    match form {
        Value::Object(_) => builder = builder.query(&form),
        Value::Null => {}
        _ => return Err(anyhow!("request[\"form\"] must be a key value map")),
    }

    if let Some(h) = prm.header {
        builder = builder.headers(build_header(&h)?);
    }
    Ok(builder)
}

/// Builds a header map from the header arg passed in from a ::Take or ::Record struct
fn build_header(header: &str) -> Result<HeaderMap, Error> {
    let map: HashMap<String, String> = serde_json::from_str(header)?;
    return match HeaderMap::try_from(&map) {
        Ok(m) => Ok(m),
        Err(m) => Err(Error::from(m)),
    };
}

pub fn http_request(prm: Params, req: Request) -> Result<Response, Error> {
    let response = build_request(prm, req)?.send()?;
    let status = response.status().as_u16() as u32;

    Ok(Response {
        body: response
            .json()
            .context("reqwest::Response.json() decode failure")?,
        // TODO add response headers
        etc: json!({}),
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::header;
    use rstest::*;

    fn case_build_header(case: u32) -> HeaderMap {
        let mut header = HeaderMap::new();
        match case {
            1 => {
                header.insert(header::AUTHORIZATION, "Bearer jWt".parse().unwrap());
            }
            2 => {
                header.insert(header::CONNECTION, "keep-alive".parse().unwrap());
                header.insert(header::AUTHORIZATION, "Bearer jWt".parse().unwrap());
            }
            _ => return header,
        };
        header
    }

    #[rstest(
        string_header,
        expected,
        case(r#"{"Authorization": "Bearer jWt"}"#, case_build_header(1)),
        case(
            r#"{"Connection": "keep-alive", "Authorization": "Bearer jWt"}"#,
            case_build_header(2)
        )
    )]
    fn test_build_header(string_header: &str, expected: HeaderMap) {
        assert_eq!(expected, build_header(string_header).unwrap());
    }
}
