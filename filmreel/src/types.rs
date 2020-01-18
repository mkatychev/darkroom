use jql;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Represents the protocol used to send the frame payload.
///
/// https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#frame-nomenclature
#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Protocol {
    GRPC,
    HTTP,
}

/// Represents the entire deserialized frame file.
///
/// https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#frame
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Frame<'a> {
    protocol: Protocol,
    #[serde(borrow)]
    cut: InstructionSet<'a>,
    request: Request,
    response: Response,
}

/// Encapsulates the request payload to be sent.
///
/// https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#request
#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
/// https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#cut-instruction-set
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
struct InstructionSet<'a> {
    #[serde(rename(serialize = "from", deserialize = "from"))]
    #[serde(serialize_with = "ordered_set", borrow)]
    reads: HashSet<&'a str>,
    #[serde(rename(serialize = "to", deserialize = "to"))]
    #[serde(serialize_with = "ordered_map", borrow)]
    writes: HashMap<&'a str, &'a str>,
}

fn ordered_map<S>(value: &HashMap<&str, &str>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

fn ordered_set<S>(value: &HashSet<&str>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeSet<_> = value.iter().collect();
    ordered.serialize(serializer)
}

/// Encapsulates the expected response payload.
///
/// https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#request
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Response {
    body: Value,
    #[serde(flatten)]
    etc: Value,
    status: u32,
}

/// Constructs a set of read instructions from strings meant associated with
/// variables present in the `Cut Register`
///
/// ```edition2018
/// let write_instructions = to![
///     "SESSION_ID" => ".response.body.session_id",
///     "DATETIME" => ".response.body.timestamp"];
/// ```
///
/// https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#from-to
macro_rules! to {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map: HashMap<&str, &str> = std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

/// Constructs a set of read instructions from strings meant associated with
/// variables present in the `Cut Register`
///
/// ```edition2018
/// let read_instructions = from!["USER_ID", "USER_TOKEN"];
/// ```
///
/// https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#from-to
/// TODO check Cut Register during macro call
macro_rules! from {
    ($( $cut_var: expr ),*) => {{
         let mut set:HashSet<&str> = std::collections::HashSet::new();
         $( set.insert($cut_var); )*
         set
    }}
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, json, to_string};

    /// test_ser_de tests the serialization and deserialization of frame structs
    ///
    /// ```edition2018
    ///  test_ser_de!(
    ///      protocol_grpc_ser,  // serialization test name
    ///      protocol_grpc_de,   // deserialization test name
    ///      Protocol,           // struct type
    ///      Protocol::GRPC,     // struct
    ///      PROTOCOL_GRPC_JSON  // json format
    /// );
    /// ```
    ///
    macro_rules! test_ser_de {
        ($ser:ident, $de:ident, $type:ty, $struct:expr, $str_json:expr) => {
            #[test]
            fn $ser() {
                let str_val: Value = from_str($str_json).unwrap();
                let actual = serde_json::value::to_value(&$struct).unwrap();
                assert_eq!(str_val, actual);
            }
            #[test]
            fn $de() {
                let actual: $type = from_str($str_json).unwrap();
                assert_eq!(&$struct, &actual);
            }
        };
    }

    const PROTOCOL_GRPC_JSON: &str = r#""GRPC""#;
    test_ser_de!(
        protocol_grpc_ser,
        protocol_grpc_de,
        Protocol,
        Protocol::GRPC,
        PROTOCOL_GRPC_JSON
    );

    const PROTOCOL_HTTP_JSON: &str = r#""HTTP""#;
    test_ser_de!(
        protocol_http_ser,
        protocol_http_de,
        Protocol,
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
        Request,
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
        Request,
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
        Response,
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
        Response,
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
        InstructionSet,
        InstructionSet {
            reads: from!["USER_ID", "USER_TOKEN"],
            writes: to![
                "SESSION_ID" => ".response.body.session_id",
                "DATETIME" => ".response.body.timestamp"],
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
        Frame,
        Frame {
            protocol: Protocol::HTTP,
            cut: InstructionSet {
                reads: from!["USER_ID", "USER_TOKEN"],
                writes: to!["SESSION_ID" => ".response.body.session_id",
    "DATETIME" => ".response.body.timestamp"],
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
}
