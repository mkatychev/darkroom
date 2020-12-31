use crate::params::Params;
use anyhow::{anyhow, Context, Error};
use filmreel::frame::{Request, Response};
use http::header::HeaderMap;
use log::warn;
use reqwest::{blocking::*, Method};
use serde_json::{json, Value};
use std::{collections::HashMap, convert::TryFrom, time::Duration};
use url::Url;

/// build_request parses a Frame Request and a Params object to send a HTTP payload using reqwest
pub fn build_request(prm: Params, req: Request) -> Result<RequestBuilder, Error> {
    let method: Method;
    let endpoint: Url;

    let timeout = match prm.timeout {
        0 => None,
        _ => Some(Duration::from_secs(prm.timeout)),
    };

    match &req
        .get_uri()
        .splitn(2, ' ')
        .collect::<Vec<&str>>()
        .as_slice()
    {
        [method_str, tail_str] => {
            method = Method::from_bytes(method_str.as_bytes())?;
            let entrypoint = prm.address;
            endpoint = Url::parse(&entrypoint)
                .context(format!("base url: {}", entrypoint))?
                .join(tail_str)
                .context(format!(
                    "base url: {}, This is the case if the scheme and ':' delimiter are not followed by a '/',
such as 'data:' mailto: URLs, and localhost without a leading http:// or https://",
                    entrypoint
                ))?;
        }
        _ => {
            return Err(anyhow!("unable to parse request uri field"));
        }
    };

    let mut builder = Client::builder()
        .timeout(timeout)
        .build()?
        .request(method, endpoint);
    match req.to_val_payload() {
        Ok(b) => {
            // TODO handle empty body better
            if b != json!({}) {
                builder = builder.body(b.to_string());
            }
        }
        Err(e) => return Err(Error::from(e)),
    }

    match req.get_etc().get("form") {
        Some(Value::Object(f)) => builder = builder.form(&f),
        Some(Value::Null) | None => {}
        _ => return Err(anyhow!("request[\"form\"] must be a key value map")),
    }

    match req.get_etc().get("query") {
        Some(Value::Object(f)) => builder = builder.query(&f),
        Some(Value::Null) | None => {}
        _ => return Err(anyhow!("request[\"query\"] must be a key value map")),
    }

    if let Some(h) = prm.header {
        builder = builder.headers(build_header(&h)?);
    }
    Ok(builder)
}

/// build_header constructs a header map from the header arg passed in from a ::Take or ::Record struct
fn build_header(header: &str) -> Result<HeaderMap, Error> {
    let map: HashMap<String, String> = serde_json::from_str(header)?;
    match HeaderMap::try_from(&map) {
        Ok(m) => Ok(m),
        Err(m) => Err(Error::from(m)),
    }
}

// request is used by run_request to send an http request and deserialize the returned data
// into a Response struct
pub fn request(prm: Params, req: Request) -> Result<Response, Error> {
    let response = build_request(prm, req)?.send()?;
    let status = response.status().as_u16() as u32;
    // reqwest.Response is a private Option<Value> field so we rely on
    // the Response.content_length() method to get the exact body byte size
    let response_body: Option<Value> = match response.content_length() {
        Some(0) => None,
        None => {
            warn!("unable to determine Response body content length");
            None
        }
        Some(_) => response
            .json()
            .context("http::request response.json() decode failure")?,
    };

    Ok(Response {
        // TODO add response headers
        body: response_body,
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
