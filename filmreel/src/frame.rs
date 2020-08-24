use crate::{
    cut::Register,
    error::FrError,
    utils::{get_jql_value, ordered_set, ordered_str_map},
};
use serde::{Deserialize, Serialize};
use serde_json::{error::Error as SerdeError, json, to_value, Value};
use std::collections::{HashMap, HashSet};

/// Represents the entire deserialized frame file.
///
/// [Frame spec](https://github.com/Bestowinc/filmReel/blob/master/frame.md#frame)
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Frame<'a> {
    pub protocol: Protocol,
    // Both the reads and writes can be optional
    #[serde(default, borrow, skip_serializing_if = "InstructionSet::is_empty")]
    pub cut: InstructionSet<'a>,
    request: Request,
    pub response: Response,
}

impl<'a> Frame<'a> {
    /// Creates a new Frame object running post deserialization validations
    pub fn new(json_string: &str) -> Result<Frame, FrError> {
        let frame: Frame = serde_json::from_str(json_string)?;
        frame.cut.validate()?;
        Ok(frame)
    }

    /// Serializes the Frame struct to a serde_json::Value
    pub fn to_value(&self) -> Value {
        to_value(self).expect("serialization error")
    }

    /// Serialized payload
    pub fn get_request(&self) -> Request {
        self.request.clone()
    }

    /// Serialized payload
    pub fn get_request_uri(&self) -> Result<String, FrError> {
        let unst = serde_json::to_string(&self.request.uri)?;

        Ok(unst.replace("\"", ""))
    }

    /// Returns a Value object from the response body, used for response comparisons and writing to
    /// the cut register
    pub fn get_response_value(&self) -> Result<Value, SerdeError> {
        to_value(&self.response.body)
    }

    /// Traverses Frame properties where Read Operations are permitted and
    /// performs Register.read_operation on Strings with Cut Variables
    pub fn hydrate(&mut self, reg: &Register, hide: bool) -> Result<(), FrError> {
        let set = self.cut.clone();
        Self::hydrate_val(&set, &mut self.request.body, reg, hide)?;
        Self::hydrate_val(&set, &mut self.request.etc, reg, hide)?;
        if let Some(response_body) = &mut self.response.body {
            Self::hydrate_val(&set, response_body, reg, hide)?;
        }
        Self::hydrate_val(&set, &mut self.response.etc, reg, hide)?;
        if let Some(header) = &mut self.request.header {
            Self::hydrate_val(&set, header, reg, hide)?;
        }

        // URI and entrypoint is given an explicit read operation
        Self::hydrate_str(&set, &mut self.request.uri, reg, hide)?;
        if let Some(entrypoint) = &mut self.request.entrypoint {
            Self::hydrate_str(&set, entrypoint, reg, hide)?;
        }
        Ok(())
    }

    /// Traverses a given serde::Value enum attempting to modify found Strings
    /// for the moment this method also works as a Frame.init() check, emitting FrameParseErrors
    pub fn hydrate_val(
        set: &InstructionSet,
        val: &mut Value,
        reg: &Register,
        hide: bool,
    ) -> Result<(), FrError> {
        match val {
            Value::Object(map) => {
                for (_, val) in map.iter_mut() {
                    Self::hydrate_val(set, val, reg, hide)?;
                }
                Ok(())
            }
            Value::Array(vec) => {
                for val in vec.iter_mut() {
                    Self::hydrate_val(set, val, reg, hide)?;
                }
                Ok(())
            }
            Value::String(_) => {
                Self::hydrate_str(set, val, reg, hide)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Performs a Register.read_operation on the entire String
    fn hydrate_str(
        set: &InstructionSet,
        string: &mut Value,
        reg: &Register,
        hide: bool,
    ) -> Result<(), FrError> {
        {
            let matches = reg.read_match(&string.as_str().expect("hydrate_str None found"))?;
            // Check if the InstructionSet has the given variable
            for mat in matches.into_iter() {
                if let Some(n) = mat.name() {
                    if !set.contains(n) {
                        return Err(FrError::FrameParsef(
                            "Variable is not present in InstructionSet",
                            n.to_string(),
                        ));
                    }
                    // Now that the cut var is confirmed to exist in the entire instruction set
                    // perform read operation ony if cut var is present in read instructions
                    if set.reads.contains(n) {
                        reg.read_operation(mat, string, hide)?;
                        continue;
                    }
                    // if variable name is found in the "to" field of the InstructionSet
                    // AND `hydrate_writes` is true
                    if set.writes.contains_key(n) && set.hydrate_writes {
                        reg.read_operation(mat, string, hide)?;
                    }
                }
            }
            Ok(())
        }
    }
}

/// Represents the protocol used to send the frame payload.
///
/// [Protocol example](https://github.com/Bestowinc/filmReel/blob/master/frame.md#frame-nomenclature)
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Protocol {
    #[serde(rename(serialize = "gRPC", deserialize = "gRPC"))]
    GRPC,
    HTTP,
}

/// Contains read and write instructions for the [Cut Register](::Cut::Register),
/// `InstructionSet` should be immutable once initialized.
///
/// [Cut Instruction Set](https://github.com/Bestowinc/filmReel/blob/master/frame.md#cut-instruction-set)
#[derive(Serialize, Clone, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct InstructionSet<'a> {
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
        serialize_with = "ordered_str_map",
        borrow
    )]
    writes: HashMap<&'a str, &'a str>,
    #[serde(skip_serializing, default)]
    pub hydrate_writes: bool,
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

/// Encapsulates the request payload to be sent.
///
/// [Request Object](https://github.com/Bestowinc/filmReel/blob/master/frame.md#request)
#[derive(Serialize, Clone, Deserialize, Default, Debug, PartialEq)]
pub struct Request {
    body: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    header: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entrypoint: Option<Value>,
    #[serde(flatten)]
    etc: Value,
    uri: Value,
}

impl Request {
    pub fn to_payload(&self) -> Result<String, SerdeError> {
        serde_json::to_string_pretty(&self.body)
    }

    pub fn to_val_payload(&self) -> Result<Value, SerdeError> {
        serde_json::to_value(&self.body)
    }

    pub fn get_uri(&self) -> String {
        if let Value::String(string) = &self.uri {
            return string.to_string();
        }
        "".to_string()
    }

    pub fn get_etc(&self) -> Value {
        self.etc.clone()
    }

    pub fn get_header(&self) -> Option<Value> {
        self.header.clone()
    }

    pub fn get_entrypoint(&self) -> Option<String> {
        if let Some(entrypoint) = self.entrypoint.clone() {
            return Some(String::from(entrypoint.as_str()?));
        }
        None
    }
}

/// Encapsulates the expected response payload.
///
/// [Request Object](https://github.com/Bestowinc/filmReel/blob/master/frame.md#request)
#[derive(Serialize, Clone, Deserialize, Debug, Default, PartialEq)]
pub struct Response {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    #[serde(flatten)]
    pub etc: Value,
    pub status: u32,
}

impl Response {
    /// Cast to a serialized Frame as serde_json::Value object for consistency in jql object
    /// traversal: `"response"."body"` should always traverse a serialized Frame struct
    fn to_frame_value(&self) -> Result<Value, FrError> {
        let mut frame_value = json!({"response":{}});
        frame_value["response"] = to_value(self)?;
        Ok(frame_value)
    }

    /// Using the write instructions found in the frame InstructionSet, look for matches to be
    /// passed to write operations
    pub fn match_payload_response<'a>(
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
                    "frame write instruction did not correspond to a string object:",
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
/// [`"from"` key](https://github.com/Bestowinc/filmReel/blob/master/cut.md#from-to)
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
/// [`"to"` key](https://github.com/Bestowinc/filmReel/blob/master/cut.md#from-to)
// TODO check Cut Register during macro call
#[macro_export]
macro_rules! from {
    ($( $cut_var: expr ),*) => {{
        use ::std::collections::HashSet;

        #[allow(unused_mut)]
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
      "EMAIL",
      "FIRST",
      "HOST",
      "LAST",
      "METHOD",
      "OBJECT",
      "PORT",
      "USER_TOKEN"
    ]
  },
  "request": {
    "header": {
      "Authorization": "${USER_TOKEN}"
    },
    "entrypoint": "${HOST}:${PORT}",
    "body": {
      "name": "${FIRST} ${LAST}",
      "email": "${EMAIL}",
      "object": "${OBJECT}"
    },
    "uri": "user_api.User/${METHOD}"
  },
  "response": {
    "body": "${RESPONSE}",
    "status": 0
  }
}
    "#;

    #[test]
    fn test_hydrate() {
        let reg = register!({
            "EMAIL"=> "new_user@humanmail.com",
            "FIRST"=> "Mario",
            "HOST"=> "localhost",
            "LAST"=> "Rossi",
            "METHOD"=> "CreateUser",
            "OBJECT"=>json!({ "key": "value"}),
            "PORT"=> "8080",
            "USER_TOKEN"=> "Bearer jWt"
        });
        let mut frame: Frame = Frame::new(FRAME_JSON).unwrap();
        // TODO add hidden test
        frame.hydrate(&reg, false).unwrap();
        assert_eq!(
            Frame {
                protocol: Protocol::GRPC,
                cut: InstructionSet {
                    reads: from![
                        "EMAIL",
                        "FIRST",
                        "HOST",
                        "LAST",
                        "METHOD",
                        "OBJECT",
                        "PORT",
                        "USER_TOKEN"
                    ],
                    writes: HashMap::new(),
                    hydrate_writes: false,
                },
                request: Request {
                    body: json!({
                        "name": "Mario Rossi",
                        "email": "new_user@humanmail.com",
                        "object": json!({ "key": "value"})
                    }),
                    header: Some(json!({"Authorization": "Bearer jWt"})),
                    entrypoint: Some(json!("localhost:8080")),
                    etc: json!({}),
                    uri: json!("user_api.User/CreateUser"),
                },

                response: Response {
                    body: Some(json!("${RESPONSE}")),
                    etc: json!({}),
                    status: 0,
                },
            },
            frame
        );
    }

    #[test]
    fn test_match_payload_response() {
        let frame = Frame {
            protocol: Protocol::GRPC,
            cut: InstructionSet {
                reads: from![],
                writes: to! ({
                    "USER_ID"=> "'response'.'body'.'id'",
                    "CREATED"=> "'response'.'body'.'created'"
                }),
                hydrate_writes: false,
            },
            request: Request {
                ..Default::default()
            },
            response: Response {
                body: Some(json!({
                    "id": "${USER_ID}",
                    "created": "${CREATED}"
                })),
                etc: json!({}),
                status: 0,
            },
        };

        let payload_response = Response {
            body: Some(json!({
                "id": "ID_010101",
                "created": 101010
            })),
            etc: json!({}),
            status: 0,
        };
        let mat = frame
            .response
            .match_payload_response(&frame.cut, &payload_response)
            .unwrap();
        let mut expected_match = HashMap::new();
        expected_match.insert("USER_ID", to_value("ID_010101").unwrap());
        expected_match.insert("CREATED", to_value(101010).unwrap());
        assert_eq!(expected_match, mat.unwrap());
    }

    #[test]
    fn test_instruction_set_validate() {
        let set = InstructionSet {
            reads: from!["USER_ID"],
            writes: to! ({"USER_ID"=> "'response'.'body'.'id'"}),
            hydrate_writes: false,
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
            header: None,
            entrypoint: None,
            etc: json!({}),
            uri: json!("user_api.User/CreateUser"),
        },
        REQUEST_JSON
    );

    const REQUEST_ETC_JSON: &str = r#"
{
  "header": {
    "Authorization": "${USER_TOKEN}"
  },
  "id": "007",
  "body": {},
  "uri": "POST /logout/${USER_ID}"
}
    "#;

    test_ser_de!(
        request_etc_ser,
        request_etc_de,
        Request {
            body: json!({}),
            header: Some(json!({"Authorization": "${USER_TOKEN}"})),
            entrypoint: None,
            etc: json!({"id": "007"}),
            uri: json!("POST /logout/${USER_ID}"),
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
            body: Some(json!("created user: ${USER_ID}")),
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
            body: Some(json!("created user: ${USER_ID}")),
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
            hydrate_writes: false,
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
                writes: to!({
                    "SESSION_ID" => ".response.body.session_id",
                    "DATETIME" => ".response.body.timestamp"
                }),
                hydrate_writes: false,
            },
            request: Request {
                body: json!({}),
                header: Some(json!({ "Authorization": "${USER_TOKEN}" })),
                uri: json!("POST /logout/${USER_ID}"),
                etc: json!({}),
                ..Default::default()
            },

            response: Response {
                body: Some(json!({
                  "message": "User ${USER_ID} logged out",
                  "session_id": "${SESSION_ID}",
                  "timestamp": "${DATETIME}"
                })),
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
                uri: json!("POST /logout/${USER_ID}"),
                ..Default::default()
            },

            response: Response {
                body: None,
                etc: json!({}),
                status: 200,
            },
        },
        SIMPLE_FRAME_JSON
    );
}
