use crate::{BoxError, Record, Take};
use filmreel::frame::Request;

/// Parameters needed for a uri method to be sent.
#[derive(Debug, PartialEq)]
pub struct Params {
    pub tls: bool,
    pub header: Option<String>,
    pub address: String,
}

/// BaseParams contains parameter values provided by a Record or Take object
/// before the given values are checked for in the Frame
pub struct BaseParams<'a> {
    tls: bool,
    header: &'a Option<String>,
    address: &'a Option<String>,
}

impl<'a> From<&'a Record> for BaseParams<'a> {
    fn from(record: &'a Record) -> Self {
        Self {
            tls: record.tls,
            header: &record.header,
            address: &record.address,
        }
    }
}

impl<'a> From<&'a Take> for BaseParams<'a> {
    fn from(take: &'a Take) -> Self {
        Self {
            // TODO handle tls
            tls: take.tls,
            header: &take.header,
            address: &take.address,
        }
    }
}

impl<'a> BaseParams<'a> {
    /// init provides a frame's request properties to override or populated
    /// parameter fields desired by a specific Frame
    pub fn init(&self, request: Request) -> Result<Params, BoxError> {
        // let request = frame.get_request();

        let header: Option<String> = match request.get_header() {
            Some(i) => Some(i.to_string()),
            None => self.header.clone(),
        };

        let address = match request.get_entrypoint() {
            Some(i) => i,
            None => self.address.clone().ok_or("missing address")?,
        };

        Ok(Params {
            tls: self.tls,
            header,
            address,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use filmreel::frame::{Frame, Request};
    use std::path::PathBuf;

    #[test]
    fn test_init() {
        let take = &Take {
            tls: false,
            frame: PathBuf::new(),
            address: Some("www.initial_addr.com".to_string()),
            cut: PathBuf::new(),
            header: Some("initial_header".to_string()),
            output: None,
        };
        let request: Request = serde_json::from_str::<Frame>(
            r#"
{
  "protocol": "HTTP",
  "request": {
    "body": {},
    "header": "Authorization: Bearer BIG_BEAR",
    "entrypoint": "localhost:8000",
    "uri": "POST /it/notes"
  },
  "response": {
    "body": {},
    "status": 200
  }
}
    "#,
        )
        .unwrap()
        .get_request();
        let params: Params = BaseParams::from(take).init(request).unwrap();
        assert_eq!(
            Params {
                tls: false,
                header: Some("\"Authorization: Bearer BIG_BEAR\"".to_string()),
                address: "localhost:8000".to_string(),
            },
            params
        )
    }
}
