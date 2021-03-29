use crate::{cut::Register, error::FrError, frame::*, utils::get_jql_value};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_value, Value};
use std::collections::{BTreeMap, HashMap};

const INVALID_INSTRUCTION_TYPE_ERR: &str =
    "Frame write instruction did not correspond to a string object";

///
/// Encapsulates the expected response payload.
///
/// [Request Object](https://github.com/Bestowinc/filmReel/blob/master/frame.md#request)
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Response<'a> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body:       Option<Value>,
    //
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub etc:        Option<Value>, // https://github.com/serde-rs/serde/issues/1626
    #[serde(borrow, skip_serializing_if = "Option::is_none")]
    pub validation: Option<Validation<'a>>,
    pub status:     u32,
}

impl<'a> Response<'a> {
    /// Cast to a serialized Frame as serde_json::Value object for consistency in jql object
    /// traversal: `"response"."body"` should always traverse a serialized Frame struct
    fn to_frame_value(&self) -> Result<Value, FrError> {
        Ok(json!({"response":to_value(self)?}))
    }

    /// Using the write instructions found in the frame InstructionSet, look for matches to be
    /// passed to write operations
    pub fn match_payload_response(
        &self,
        set: &'a InstructionSet,
        payload_response: &Response,
    ) -> Result<Option<HashMap<&'a str, Value>>, FrError> {
        let frame_response: Value = self.to_frame_value()?;
        let payload_response: Value = payload_response.to_frame_value()?;

        let mut write_matches: HashMap<&str, Value> = HashMap::new();
        for (k, query) in set.writes.iter() {
            // ensure frame jql query returns a string object
            let frame_str = match get_jql_value(&frame_response, query) {
                Ok(Value::String(v)) => Ok(v),
                Ok(_) => Err(FrError::FrameParsef(
                    INVALID_INSTRUCTION_TYPE_ERR,
                    query.to_string(),
                )),
                Err(e) => Err(e),
            }?;
            let payload_val = get_jql_value(&payload_response, query)?;

            if let Value::String(payload_str) = &payload_val {
                let write_match = Register::write_match(k, &frame_str, payload_str)?;
                if let Some(mat) = write_match {
                    write_matches.insert(k, to_value(mat)?);
                }
                continue;
            }
            // handle non string payload values returned by the jql query
            Register::expect_standalone_var(k, &frame_str)?;
            write_matches.insert(k, payload_val);
        }

        if write_matches.iter().next().is_some() {
            return Ok(Some(write_matches));
        }

        Ok(None)
    }

    // /// Applies the validations usking the BTree key as the Value selector
    // pub fn apply_validation(&mut self, other: &mut Self) -> Result<(), FrError> {
    //     for (query, validation) in self.validator.iter() {
    //         query.get_jql_value
    //     }
    //     Ok(())
    // }
}

impl Default for Response<'_> {
    fn default() -> Self {
        Self {
            body:       None,
            etc:        Some(json!({})),
            validation: None,
            status:     0,
        }
    }
}

/// PartialEq needs to exlcude self.validation to ensure that [Response::aply_validation] can
/// diffentiatiate between the parent `Response` (the one pulled directle from the filmReel file)
/// and the child `Response` (one deserialized from returned data) since the client validations
/// should always be `None`
impl<'a> PartialEq for Response<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.body.eq(&other.body) && self.etc.eq(&other.etc) && self.status.eq(&other.status)
    }
}

impl<'a> Eq for Response<'a> {}

type Validation<'a> = BTreeMap<&'a str, Validator>;

/// Validator represents one validation ruleset applied to a single JSON selection
#[derive(Serialize, Clone, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct Validator {
    strict: bool,
    sorted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from, to};
    use serde_json::json;

    #[test]
    fn test_match_payload_response() {
        let frame = Frame {
            protocol: Protocol::GRPC,
            cut:      InstructionSet {
                reads:          from![],
                writes:         to! ({
                    "USER_ID"=> "'response'.'body'.'id'",
                    "CREATED"=> "'response'.'body'.'created'",
                    "ignore"=> "'response'.'body'.'array'.[0].'ignore'"
                }),
                hydrate_writes: true,
            },
            request:  Request {
                ..Default::default()
            },
            response: Response {
                body: Some(json!({
                    "id": "${USER_ID}",
                    "created": "${CREATED}",
                    "array": [{"ignore":"${ignore}"}]
                })),
                status: 0,
                ..Default::default()
            },
        };

        let payload_response = Response {
            body: Some(json!({
                "id": "ID_010101",
                "created": 101010,
                "array": [{"ignore": "value"}]
            })),
            status: 0,
            ..Default::default()
        };
        let mat = frame
            .response
            .match_payload_response(&frame.cut, &payload_response)
            .unwrap();
        let mut expected_match = HashMap::new();
        expected_match.insert("USER_ID", to_value("ID_010101").unwrap());
        expected_match.insert("CREATED", to_value(101010).unwrap());
        expected_match.insert("ignore", to_value("value").unwrap());
        assert_eq!(expected_match, mat.unwrap());
    }
}
