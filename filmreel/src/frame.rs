use crate::cut::Register;
use crate::utils::{ordered_map, ordered_set};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Represents the entire deserialized frame file.
///
/// [Frame spec](https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#frame)
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Frame<'a> {
    protocol: Protocol,
    #[serde(borrow)]
    cut: InstructionSet<'a>,
    request: Request,
    response: Response,
}

// impl<'a> Frame<'a> {
//     fn hydrate(mut self, register: &Register) -> Self {
//         let cut_vars: Vec<&str> = register.map(|k, _| k).collect();
//         for (k, _) in register {}
//     }
// }

/// Represents the protocol used to send the frame payload.
///
/// [Protocol example](https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#frame-nomenclature)
#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Protocol {
    GRPC,
    HTTP,
}

/// Encapsulates the request payload to be sent.
///
/// [Request Object](https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#request)
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
/// [Cut Instruction Set](https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#cut-instruction-set)
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
struct InstructionSet<'a> {
    #[serde(rename(serialize = "from", deserialize = "from"))]
    #[serde(serialize_with = "ordered_set", borrow)]
    reads: HashSet<&'a str>,
    #[serde(rename(serialize = "to", deserialize = "to"))]
    #[serde(serialize_with = "ordered_map", borrow)]
    writes: HashMap<&'a str, &'a str>,
}

/// Encapsulates the expected response payload.
///
/// [Request Object](https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#request)
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
/// let read_instructions = from!["USER_ID", "USER_TOKEN"];
/// ```
///
/// [`"to"` key](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#from-to)
/// TODO check Cut Register during macro call
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
mod serde_tests {
    use super::*;
    use crate::test_ser_de;
    use serde_json::json;

    const PROTOCOL_GRPC_JSON: &str = r#""GRPC""#;
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
}
