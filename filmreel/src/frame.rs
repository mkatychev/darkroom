use crate::cut::Register;
use crate::error::FrError;
use crate::utils::{get_jql_string, ordered_map, ordered_set};
use serde::{Deserialize, Serialize};
use serde_json::error::Error as SerdeError;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::error::Error;

/// Represents the entire deserialized frame file.
///
/// [Frame spec](https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#frame)
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Frame<'a> {
    protocol: Protocol,
    // Both the reads and writes can be optional
    #[serde(default, borrow, skip_serializing_if = "InstructionSet::is_empty")]
    cut: InstructionSet<'a>,
    request: Request,
    response: Response,
}

#[allow(dead_code)] // FIXME
impl<'a> Frame<'a> {
    /// Creates a new Frame object running post deserialization validations
    pub fn new(json_string: &str) -> Result<Frame, FrError> {
        let frame: Frame = serde_json::from_str(json_string)?;
        frame.cut.validate()?;
        Ok(frame)
    }

    /// Pretty json formatting for Frame serialization
    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialization error")
    }

    /// Serializes the Frame struct to a serde_json::Value
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("serialization error")
    }

    /// Serialized payload
    pub fn get_request(&self) -> String {
        serde_json::to_string_pretty(&self.request.body).expect("serialization error")
    }

    /// Serialized payload
    pub fn get_request_uri(&self) -> Result<String, FrError> {
        let unst = serde_json::to_string(&self.request.uri)?;

        Ok(unst.replace("\"", ""))
    }

    /// Returns a Value object from the response body, used for response comparisons and writing to
    /// the cut register
    pub fn get_response_value(&self) -> Result<Value, SerdeError> {
        serde_json::to_value(&self.response.body)
    }

    /// Traverses Frame properties where Read Operations are permitted and
    /// performs Register.read_operation on Strings with Cut Variables
    pub fn hydrate(&mut self, reg: &Register) -> Result<(), FrError> {
        let set = self.cut.clone();
        Self::hydrate_val(&set, &mut self.request.body, reg)?;
        Self::hydrate_val(&set, &mut self.request.etc, reg)?;
        Self::hydrate_val(&set, &mut self.response.body, reg)?;
        Self::hydrate_val(&set, &mut self.response.etc, reg)?;

        // URI is given an explicit read operation
        Self::hydrate_str(&set, &mut self.request.uri, reg)?;
        Ok(())
    }

    /// Traverses a given serde::Value enum attempting to modify found Strings
    /// for the moment this method also works as a Frame.init() check, emitting FrameParseErrors
    fn hydrate_val(set: &InstructionSet, val: &mut Value, reg: &Register) -> Result<(), FrError> {
        match val {
            Value::Object(map) => {
                for (_, val) in map.iter_mut() {
                    Self::hydrate_val(set, val, reg)?;
                }
                Ok(())
            }
            Value::Array(vec) => {
                for val in vec.iter_mut() {
                    Self::hydrate_val(set, val, reg)?;
                }
                Ok(())
            }
            Value::String(string) => {
                Self::hydrate_str(set, string, reg)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Performs a Register.read_operation on the entire String
    fn hydrate_str(
        set: &InstructionSet,
        string: &mut String,
        reg: &Register,
    ) -> Result<(), FrError> {
        {
            let matches = reg.read_match(string)?;
            // Check if the InstructionSet has the given variable
            for mat in matches.into_iter() {
                if let Some(n) = mat.name() {
                    if !set.contains(n) {
                        return Err(FrError::FrameParsef(
                            "Variable is not present in Frame InstructionSet",
                            n.to_string(),
                        ));
                    }
                }
                reg.read_operation(mat, string)?;
            }
            Ok(())
        }
    }

    /// Using the write instructions found in the frame InstructionSet, look for matches to be
    /// passed to write operations
    pub fn match_payload_response(
        &self,
        payload_response: &'a Response,
    ) -> Result<HashMap<&str, String>, Box<dyn Error>> {
        let frame_response: Value = self.response.to_frame_value()?;
        let payload_response: Value = payload_response.to_frame_value()?;

        let mut write_matches: HashMap<&str, String> = HashMap::new();
        for (k, query) in self.cut.writes.iter() {
            // Temporary holdover until write operations are implemented for request
            let frame_str = get_jql_string(&frame_response, query)?;
            let payload_str = get_jql_string(&payload_response, query)?;

            // println!("{}{}", frame_jq, payload_jq);
            let write_match = Register::write_match(k, &frame_str, &payload_str)?;
            if let Some(mat) = write_match {
                write_matches.insert(k, mat);
            }
            // TODO reg.write_operation(k, to_val.to_string())?;
        }

        Ok(write_matches)
    }
}

/// Represents the protocol used to send the frame payload.
///
/// [Protocol example](https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#frame-nomenclature)
#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Protocol {
    #[serde(rename(serialize = "gRPC", deserialize = "gRPC"))]
    GRPC,
    HTTP,
}

/// Encapsulates the request payload to be sent.
///
/// [Request Object](https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#request)
#[derive(Serialize, Clone, Deserialize, Default, Debug, PartialEq)]
struct Request {
    body: Value,
    #[serde(flatten)]
    etc: Value,
    uri: String,
}

/// Contains read and write instructions for the [Cut
/// Register](::Cut::Register), `InstructionSet` should be immutable once
/// initialized.
///
/// [Cut Instruction Set](https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#cut-instruction-set)
#[derive(Serialize, Clone, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
struct InstructionSet<'a> {
    #[serde(rename(serialize = "from", deserialize = "from"))]
    #[serde(
        skip_serializing_if = "HashSet::is_empty",
        serialize_with = "ordered_set",
        borrow
    )]
    reads: HashSet<&'a str>,
    #[serde(rename(serialize = "to", deserialize = "to"))]
    #[serde(
        skip_serializing_if = "HashMap::is_empty",
        serialize_with = "ordered_map",
        borrow
    )]
    writes: HashMap<&'a str, &'a str>,
}

impl<'a> InstructionSet<'a> {
    fn is_empty(&self) -> bool {
        self.reads.is_empty() && self.writes.is_empty()
    }

    fn contains(&self, var: &str) -> bool {
        self.reads.contains(var) || self.writes.contains_key(var)
    }

    /// Ensures no Cut Variables are present in both read and write instructions
    fn validate(&self) -> Result<(), FrError> {
        let writes_set: HashSet<&str> = self.writes.keys().cloned().collect();
        let intersection = self.reads.intersection(&writes_set).next();

        if intersection.is_some() {
            return Err(FrError::FrameParsef(
                "Cut Variables cannot be referenced by both read and write instructions",
                format!("{:?}", intersection),
            ));
        }
        Ok(())
    }
}

/// Encapsulates the expected response payload.
///
/// [Request Object](https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#request)
#[derive(Serialize, Clone, Deserialize, Debug, Default, PartialEq)]
pub struct Response {
    pub body: Value,
    #[serde(flatten)]
    pub etc: Value,
    pub status: u32,
}

impl Response {
    /// Cast to a serialized Frame as serde_json::Value object for consistency in jql object
    /// traversal: `"response"."body"` should always traverse a serialized Frame struct
    fn to_frame_value(&self) -> Result<Value, FrError> {
        let mut frame_value = json!({"response":{}});
        frame_value["response"] = serde_json::to_value(self)?;
        Ok(frame_value)
    }
}

/// Constructs a set of read instructions from strings meant associated with
/// variables present in the `Cut Register`
///
/// ```edition2018
/// use filmreel::to;
///
/// let write_instructions = to!({
///     "SESSION_ID" => ".response.body.session_id",
///     "DATETIME" => ".response.body.timestamp"});
/// ```
///
/// [`"from"` key](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#from-to)
#[macro_export]
macro_rules! to {
    ({$( $key: expr => $val: expr ),*}) => {{
        use ::std::collections::HashMap;

        let mut map: HashMap<&str, &str> = HashMap::new();
        $(map.insert($key, $val);)*
        map
    }}
}

/// Constructs a set of read instructions from strings meant associated with
/// variables present in the `Cut Register`
///
/// ```edition2018
/// use filmreel::from;
///
/// let read_instructions = from!["USER_ID", "USER_TOKEN"];
/// ```
///
/// [`"to"` key](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#from-to)
// TODO check Cut Register during macro call
#[macro_export]
macro_rules! from {
    ($( $cut_var: expr ),*) => {{
        use ::std::collections::HashSet;

        let mut set:HashSet<&str> = HashSet::new();
        $( set.insert($cut_var); )*
        set
    }}
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::register;
    // use rstest::*;
    use serde_json::json;

    const FRAME_JSON: &str = r#"
    {
      "protocol": "gRPC",
      "cut": {
        "from": [
          "FIRST",
          "LAST",
          "EMAIL",
          "METHOD"
        ]
      },
      "request": {
        "body": {
          "name": "${FIRST} ${LAST}",
          "email": "${EMAIL}"
        },
        "uri":"user_api.User/${METHOD}"
      },
      "response": {
        "body": "YES!",
        "status": 0
      }
    }
    "#;

    #[test]
    fn test_hydrate() {
        let reg = register!({
            "EMAIL"=> "new_user@humanmail.com",
            "FIRST"=> "Mario",
            "LAST"=> "Rossi",
            "METHOD"=> "CreateUser"
        });
        let mut frame: Frame = serde_json::from_str(FRAME_JSON).unwrap();
        frame.hydrate(&reg).unwrap();
        assert_eq!(
            Frame {
                protocol: Protocol::GRPC,
                cut: InstructionSet {
                    reads: from!["METHOD", "FIRST", "LAST", "EMAIL"],
                    writes: HashMap::new(),
                },
                request: Request {
                    body: json!({
                        "name": "Mario Rossi",
                        "email": "new_user@humanmail.com"
                    }),
                    etc: json!({}),
                    uri: String::from("user_api.User/CreateUser"),
                },

                response: Response {
                    body: json!("YES!"),
                    etc: json!({}),
                    status: 0,
                },
            },
            frame
        );
    }
    const WRITE_FRAME_JSON: &str = r#"
{
  "protocol": "gRPC",
  "cut": {
    "from": [
      "FIRST",
      "LAST",
      "EMAIL",
      "METHOD"
    ],
    "to": {
      "USER_ID": "'response'.'body'.'id'"
    }
  },
  "request": {
    "body": {
      "name": "${FIRST} ${LAST}",
      "email": "${EMAIL}"
    },
    "uri": "user_api.User/${METHOD}"
  },
  "response": {
    "body": {
      "id": "${USER_ID}"
    },
    "status": 0
  }
}
    "#;

    #[test]
    fn test_match_payload_response() {
        let reg = register!({
            "EMAIL"=> "new_user@humanmail.com",
            "FIRST"=> "Mario",
            "LAST"=> "Rossi",
            "METHOD"=> "CreateUser"
        });
        let frame = Frame {
            protocol: Protocol::GRPC,
            cut: InstructionSet {
                reads: from![],
                writes: to! ({"USER_ID"=> "'response'.'body'.'id'"}),
            },
            request: Request {
                ..Default::default()
            },
            response: Response {
                body: json!({"id": "${USER_ID}"}),
                etc: json!({}),
                status: 0,
            },
        };

        let payload_response = Response {
            body: json!({ "id": "ID_010101" }),
            etc: json!({}),
            status: 0,
        };
        let mat = frame.match_payload_response(&payload_response).unwrap();
        let mut expected_match = HashMap::new();
        expected_match.insert("USER_ID", "ID_010101".to_string());
        assert_eq!(expected_match, mat);
    }

    #[test]
    fn test_instruction_set_validate() {
        let set = InstructionSet {
            reads: from!["USER_ID"],
            writes: to! ({"USER_ID"=> "'response'.'body'.'id'"}),
        };
        assert!(set.validate().is_err());
    }
}
#[cfg(test)]
mod serde_tests {
    use super::*;
    use crate::test_ser_de;
    use serde_json::json;

    const PROTOCOL_GRPC_JSON: &str = r#""gRPC""#;
    test_ser_de!(
        protocol_grpc_ser,
        protocol_grpc_de,
        Protocol::GRPC,
        PROTOCOL_GRPC_JSON
    );

    const PROTOCOL_HTTP_JSON: &str = r#""HTTP""#;
    test_ser_de!(
        protocol_http_ser,
        protocol_http_de,
        Protocol::HTTP,
        PROTOCOL_HTTP_JSON
    );

    const REQUEST_JSON: &str = r#"
    {
      "body": {
        "email": "new_user@humanmail.com"
      },
      "uri": "user_api.User/CreateUser"
    }
    "#;
    test_ser_de!(
        request_ser,
        request_de,
        Request {
            body: json!({"email": "new_user@humanmail.com"}),
            etc: json!({}),
            uri: String::from("user_api.User/CreateUser"),
        },
        REQUEST_JSON
    );

    const REQUEST_ETC_JSON: &str = r#"
    {
      "header": {
        "Authorization": "${USER_TOKEN}"
      },
      "id" : "007",
      "body": {},
      "uri": "POST /logout/${USER_ID}"
    }
    "#;

    test_ser_de!(
        request_etc_ser,
        request_etc_de,
        Request {
            body: json!({}),
            etc: json!({"header": { "Authorization": "${USER_TOKEN}" }, "id": "007"}),
            uri: String::from("POST /logout/${USER_ID}"),
        },
        REQUEST_ETC_JSON
    );

    const RESPONSE_JSON: &str = r#"
    {
      "body": "created user: ${USER_ID}",
      "status": 0
    }
    "#;
    test_ser_de!(
        response_ser,
        response_de,
        Response {
            body: json!("created user: ${USER_ID}"),
            etc: json!({}),
            status: 0,
        },
        RESPONSE_JSON
    );

    const RESPONSE_ETC_JSON: &str = r#"
    {
      "body": "created user: ${USER_ID}",
      "user_level": "admin",
      "status": 0
    }
    "#;
    test_ser_de!(
        response_etc_ser,
        response_etc_de,
        Response {
            body: json!("created user: ${USER_ID}"),
            etc: json!({"user_level": "admin"}),
            status: 0,
        },
        RESPONSE_ETC_JSON
    );

    const INSTRUCTION_SET_JSON: &str = r#"
    {
      "from": [
        "USER_ID",
        "USER_TOKEN"
      ],
      "to": {
        "SESSION_ID": ".response.body.session_id",
        "DATETIME": ".response.body.timestamp"
      }
    }
    "#;
    test_ser_de!(
        instruction_set_ser,
        instruction_set_de,
        InstructionSet {
            reads: from!["USER_ID", "USER_TOKEN"],
            writes: to!({
                "SESSION_ID" => ".response.body.session_id",
                "DATETIME" => ".response.body.timestamp"
            }),
        },
        INSTRUCTION_SET_JSON
    );

    const FRAME_JSON: &str = r#"
    {
      "protocol": "HTTP",
      "cut": {
        "from": [
          "USER_ID",
          "USER_TOKEN"
        ],
        "to": {
          "SESSION_ID": ".response.body.session_id",
          "DATETIME": ".response.body.timestamp"
        }
      },
      "request": {
        "header": {
          "Authorization": "${USER_TOKEN}"
        },
        "body": {},
        "uri": "POST /logout/${USER_ID}"
      },
      "response": {
        "body": {
          "message": "User ${USER_ID} logged out",
          "session_id": "${SESSION_ID}",
          "timestamp": "${DATETIME}"
        },
        "status": 200
      }
    }
    "#;
    test_ser_de!(
        frame_ser,
        frame_de,
        Frame {
            protocol: Protocol::HTTP,
            cut: InstructionSet {
                reads: from!["USER_ID", "USER_TOKEN"],
                writes: to!({"SESSION_ID" => ".response.body.session_id",
                    "DATETIME" => ".response.body.timestamp"}),
            },
            request: Request {
                body: json!({}),
                etc: json!({"header": { "Authorization": "${USER_TOKEN}"}}),
                uri: String::from("POST /logout/${USER_ID}"),
            },

            response: Response {
                body: json!({
                  "message": "User ${USER_ID} logged out",
                  "session_id": "${SESSION_ID}",
                  "timestamp": "${DATETIME}"
                }),
                etc: json!({}),
                status: 200,
            },
        },
        FRAME_JSON
    );
    const SIMPLE_FRAME_JSON: &str = r#"
    {
      "protocol": "HTTP",
      "request": {
        "body": {},
        "uri": "POST /logout/${USER_ID}"
      },
      "response": {
        "body": {},
        "status": 200
      }
    }
    "#;
    test_ser_de!(
        simple_frame_ser,
        simple_frame_de,
        Frame {
            protocol: Protocol::HTTP,
            cut: InstructionSet::default(),
            request: Request {
                body: json!({}),
                etc: json!({}),
                uri: String::from("POST /logout/${USER_ID}"),
            },

            response: Response {
                body: json!({}),
                etc: json!({}),
                status: 200,
            },
        },
        SIMPLE_FRAME_JSON
    );
}
