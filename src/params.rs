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

// /// Parameters initially pulled from
// impl<'a> From<&'a Frame> for Params<'a> {
//     fn from(record: &'a Record, frame: &'a Request) -> Self {
//     }

impl<'a> From<&'a Record> for BaseParams<'a> {
    fn from(record: &'a Record) -> Self {
        Self {
            // TODO handle tls
            tls: false,
            header: &record.header,
            address: &record.addr,
        }
    }
}

impl<'a> From<&'a Take> for BaseParams<'a> {
    fn from(take: &'a Take) -> Self {
        Self {
            // TODO handle tls
            tls: false,
            header: &take.header,
            address: &take.addr,
        }
    }
}

impl<'a> BaseParams<'a> {
    pub fn init(&self, request: Request) -> Result<Params, BoxError> {
        // let request = frame.get_request();

        let header = match request.get_header() {
            Some(i) => i.to_string(),
            None => self.header.clone().ok_or("missing header")?,
        };

        let address = match request.get_endpoint() {
            Some(i) => i.to_string(),
            None => self.address.clone().ok_or("missing address")?,
        };

        return Ok(Params {
            tls: self.tls,
            header,
            address,
        });
    }
}
