use crate::{
    cut::Register,
    error::FrError,
    response::Response,
    utils::{ordered_set, ordered_str_map, MatchQuery},
};
use serde::{Deserialize, Serialize};
use serde_json::{error::Error as SerdeError, json, to_value, Value};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    convert::TryFrom,
    path::PathBuf,
};

/// Represents the entire deserialized frame file.
///
/// [Frame spec](https://github.com/Bestowinc/filmReel/blob/master/frame.md#frame)
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Frame<'a> {
    pub protocol:       Protocol,
    #[serde(default, skip_serializing_if = "InstructionSet::is_empty")]
    pub cut:            InstructionSet<'a>, // Both the reads and writes can be optional
    pub(crate) request: Request,
    pub response:       Response<'a>,
}

const MISSING_VAR_ERR: &str = "Variable is not present in InstructionSet";
const DUPE_VAR_REFERENCE_ERR: &str =
    "Cut Variables cannot be referenced by both read and write instructions";

impl<'a> Frame<'a> {
    /// Creates a new Frame object running post deserialization validations
    pub fn new(json_string: &str) -> Result<Frame, FrError> {
        let frame: Frame = serde_json::from_str(json_string)?;
        frame.cut.validate()?;
        frame.response.validate()?;
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
    pub fn get_request_uri(&self) -> String {
        self.request.get_uri()
    }

    /// Returns a Value object from the response body, used for response comparisons and writing to
    /// the cut register
    pub fn get_response_value(&self) -> Result<Value, SerdeError> {
        to_value(&self.response.body)
    }

    /// Traverses Frame properties where Read Operations are permitted and
    /// performs Register.read_operation on Strings with Cut Variables
    pub fn hydrate(&mut self, reg: &Register, hide: bool) -> Result<Vec<MatchQuery>, FrError> {
        let set = self.cut.clone();

        // call Frame::hydrate_val on all properties that are Option::Some
        let match_selectors: Vec<MatchQuery> = vec![
            &mut self.request.body,
            &mut self.request.header,
            &mut self.request.etc,
            &mut self.request.entrypoint,
            &mut self.response.body,
            &mut self.response.etc,
        ]
        .into_iter()
        .filter_map(|val| val.as_mut())
        .map(|val| Self::hydrate_val(&set, val, reg, hide, MatchQuery::new()))
        .collect::<Result<Vec<_>, FrError>>()?;

        // cast uri to serde_json::Value so that we can use it in hydrate_str
        let mut value_uri = Value::String(self.request.uri.to_owned());
        Self::hydrate_str(&set, &mut value_uri, reg, hide, MatchQuery::new())?;
        if let Value::String(s) = value_uri {
            self.request.uri = s;
        }

        Ok(match_selectors)
    }

    /// Traverses a given serde::Value enum attempting to modify found Strings
    /// for the moment this method also works as a Frame.init() check, emitting FrameParseErrors
    pub fn hydrate_val(
        set: &InstructionSet,
        val: &mut Value,
        reg: &Register,
        hide: bool,
        match_selector: MatchQuery,
    ) -> Result<Vec<MatchQuery>, FrError> {
        let selectors = match val {
            Value::Object(map) => map
                .iter_mut()
                .map(|(k, val)| {
                    let k_query = match_selector.clone();
                    k_query.append(k);
                    Self::hydrate_val(set, val, reg, hide, k_query)
                })
                .collect(),
            Value::Array(vec) => vec.iter_mut().enumerate().map(|(i, val)| {
                let i_query = match_selector.clone();
                i_query.append(i);
                Self::hydrate_val(set, val, reg, hide, match_selector)
            }),
            Value::String(_) => Self::hydrate_str(set, val, reg, hide, match_selector)?,
            _ => None,
        }
        .filter(Option::is_some)
        .collect();

        Ok(selectors)
    }

    /// Performs a Register.read_operation on the entire String
    fn hydrate_str(
        set: &InstructionSet,
        string: &mut Value,
        reg: &Register,
        hide: bool,
        match_selector: MatchQuery,
    ) -> Result<Option<MatchQuery>, FrError> {
        {
            let matches = reg.read_match(
                &string
                    .as_str()
                    .ok_or_else(|| FrError::ReadInstruction("hydrate_str None found"))?,
            )?;
            // Check if the InstructionSet has the given variable
            for mat in matches.into_iter() {
                if let Some(n) = mat.name() {
                    if !set.contains(n) {
                        return Err(FrError::FrameParsef(MISSING_VAR_ERR, n.to_string()));
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
            Ok(None)
        }
    }
}

impl<'a> TryFrom<PathBuf> for Frame<'a> {
    type Error = FrError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let buf = crate::file_to_reader(&path)?;

        let frame: Frame = serde_json::from_reader(buf)?;
        Ok(frame)
    }
}

/// Represents the protocol used to send the frame payload.
///
/// [Protocol example](https://github.com/Bestowinc/filmReel/blob/master/frame.md#frame-nomenclature)
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Protocol {
    #[serde(rename(serialize = "gRPC", deserialize = "gRPC"))]
    #[allow(clippy::upper_case_acronyms)]
    GRPC,
    #[allow(clippy::upper_case_acronyms)]
    HTTP,
}

/// Contains read and write instructions for the [`crate::Register`],
/// [`InstructionSet`] should be immutable once initialized.
///
/// [Cut Instruction Set](https://github.com/Bestowinc/filmReel/blob/master/frame.md#cut-instruction-set)
#[derive(Serialize, Clone, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct InstructionSet<'a> {
    #[serde(
        rename(serialize = "from", deserialize = "from"),
        skip_serializing_if = "HashSet::is_empty",
        serialize_with = "ordered_set"
    )]
    pub(crate) reads:   HashSet<Cow<'a, str>>,
    #[serde(
        rename(serialize = "to", deserialize = "to"),
        skip_serializing_if = "HashMap::is_empty",
        serialize_with = "ordered_str_map"
    )]
    pub(crate) writes:  HashMap<Cow<'a, str>, Cow<'a, str>>,
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
        let writes_set: HashSet<Cow<str>> = self.writes.keys().cloned().collect();
        let intersection = self.reads.intersection(&writes_set).next();

        if intersection.is_some() {
            return Err(FrError::FrameParsef(
                DUPE_VAR_REFERENCE_ERR,
                format!("{:?}", intersection),
            ));
        }
        Ok(())
    }
}

/// Encapsulates the request payload to be sent.
///
/// [Request Object](https://github.com/Bestowinc/filmReel/blob/master/frame.md#request)
#[derive(Serialize, Clone, Deserialize, Debug, PartialEq)]
pub struct Request {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) body: Option<Value>,
    pub(crate) uri:  String,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub(crate) etc:  Option<Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) header:     Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) entrypoint: Option<Value>,
}

impl Request {
    pub fn to_payload(&self) -> Result<String, SerdeError> {
        serde_json::to_string_pretty(&self.body)
    }

    pub fn to_val_payload(&self) -> Result<Option<Value>, SerdeError> {
        self.body.as_ref().map(serde_json::to_value).transpose()
    }

    pub fn get_uri(&self) -> String {
        self.uri.clone()
    }

    pub fn get_etc(&self) -> Option<Value> {
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

impl Default for Request {
    fn default() -> Self {
        Self {
            body:       None,
            uri:        String::new(),
            etc:        Some(json!({})),
            header:     None,
            entrypoint: None,
        }
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
        use ::std::borrow::Cow;

        let mut map: HashMap<Cow<str>, Cow<str>> = HashMap::new();
        $(map.insert($key.into(), $val.into());)*
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
        use ::std::borrow::Cow;

        #[allow(unused_mut)]
        let mut set:HashSet<Cow<str>> = HashSet::new();
        $( set.insert($cut_var.into()); )*
        set
    }}
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::register;
    use pretty_assertions::assert_eq;
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
                cut:      InstructionSet {
                    reads:          from![
                        "EMAIL",
                        "FIRST",
                        "HOST",
                        "LAST",
                        "METHOD",
                        "OBJECT",
                        "PORT",
                        "USER_TOKEN"
                    ],
                    writes:         HashMap::new(),
                    hydrate_writes: false,
                },
                request:  Request {
                    body:       Some(json!({
                        "name": "Mario Rossi",
                        "email": "new_user@humanmail.com",
                        "object": json!({ "key": "value"})
                    })),
                    header:     Some(json!({"Authorization": "Bearer jWt"})),
                    entrypoint: Some(json!("localhost:8080")),
                    uri:        "user_api.User/CreateUser".to_string(),
                    etc:        Some(json!({})),
                },

                response: Response {
                    body: Some(json!("${RESPONSE}")),
                    status: 0,
                    ..Default::default()
                },
            },
            frame
        );
    }

    #[test]
    fn test_instruction_set_validate() {
        let set = InstructionSet {
            reads:          from!["USER_ID"],
            writes:         to! ({"USER_ID"=> "'response'.'body'.'id'"}),
            hydrate_writes: false,
        };
        assert!(set.validate().is_err());
    }
}
