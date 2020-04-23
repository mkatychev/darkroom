use crate::{BoxError, Record, Take};
use filmreel::frame::Request;

/// Parameters needed for a uri method to be sent.
#[derive(Debug)]
pub struct Params {
    pub tls: bool,
    pub header: String,
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

        let header = match request.get_header() {
            Some(i) => i.to_string(),
            None => self.header.clone().ok_or("missing header")?,
        };

        let address = match request.get_endpoint() {
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
