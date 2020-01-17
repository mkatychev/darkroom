use jql;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#frame-nomenclature
#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Protocol {
    GRPC,
    HTTP,
}

// /// https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#frame
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Frame<'a> {
    protocol: Protocol,
    #[serde(borrow)]
    cut:      InstructionSet<'a>,
    request:  Request,
    response: Response,
}

// /// https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#request
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Request {
    body: Value,
    #[serde(flatten)]
    etc:  Value,
    uri:  String,
}

/// https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#cut-instruction-set
// This contains read and write instructions for the cut register, struct should be immutable after
// creation
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
struct InstructionSet<'a> {
    #[serde(alias = "from", borrow)]
    reads:  HashSet<&'a str>,
    #[serde(alias = "to", borrow)]
    writes: HashMap<&'a str, &'a str>,
}

/// https://github.com/Bestowinc/filmReel/blob/supra_dump/frame.md#request
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Response {
    body:   Value,
    #[serde(flatten)]
    etc:    Value,
    status: u32,
}

// to macro creates a write instuction HashMap
macro_rules! to {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map: HashMap<&str, &str> = std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         map
    }}
}

// from macro creates a read instuction HashSet
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

    macro_rules! test_deserialize {
        ($name:ident, $expected:expr, $str_json:expr) => {
            #[test]
            fn $name() {
                let actual = from_str($str_json).unwrap();
                assert_eq!($expected, actual);
            }
        };
    }

    // TODO
    // macro_rules! test_serialize {
    //     ($name:ident, $struct:expr, $str_json:expr) => {
    //         #[test]
    //         fn $name() {
    //             let formatted_str_json =
    // to_string(from_str($str_json).unwrap()).unwrap();
    // let serialized_struct = to_string($struct).unwrap();
    // assert_eq!(formatted_str_json, serialized_struct);         }
    //     };
    // }

    const PROTOCOL_GRPC_JSON: &str = r#""GRPC""#;
    const PROTOCOL_HTTP_JSON: &str = r#""HTTP""#;
    test_deserialize!(protocol_grpc_de, Protocol::GRPC, PROTOCOL_GRPC_JSON);
    test_deserialize!(protocol_http_de, Protocol::HTTP, PROTOCOL_HTTP_JSON);

    const REQUEST_JSON: &str = r#"
{
  "body": {
    "email": "new_user@humanmail.com"
  },
  "uri": "user_api.User/CreateUser"
}
"#;
    test_deserialize!(
        request_de,
        Request {
            body: json!({"email": "new_user@humanmail.com"}),
            etc:  json!({}),
            uri:  String::from("user_api.User/CreateUser"),
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

    test_deserialize!(
        request_etc_de,
        Request {
            body: json!({}),
            etc:  json!({"header": { "Authorization": "${USER_TOKEN}" }, "id": "007"}),
            uri:  String::from("POST /logout/${USER_ID}"),
        },
        REQUEST_ETC_JSON
    );

    const RESPONSE_JSON: &str = r#"
{
  "body": "created user: ${USER_ID}",
  "status": 0
}
"#;
    test_deserialize!(
        response_de,
        Response {
            body:   json!("created user: ${USER_ID}"),
            etc:    json!({}),
            status: 0,
        },
        RESPONSE_JSON
    );

    const RESPONSE_ETC: &str = r#"
{
  "body": "created user: ${USER_ID}",
  "user_level": "admin",
  "status": 0
}
"#;
    test_deserialize!(
        response_etc_de,
        Response {
            body:   json!("created user: ${USER_ID}"),
            etc:    json!({"user_level": "admin"}),
            status: 0,
        },
        RESPONSE_ETC
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
    test_deserialize!(
        instruction_set_de,
        InstructionSet {
            reads:  from!["USER_ID", "USER_TOKEN"],
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
    test_deserialize!(
        frame_de,
        Frame {
            protocol: Protocol::HTTP,
            cut:      InstructionSet {
                reads:  from!["USER_ID", "USER_TOKEN"],
                writes: to!["SESSION_ID" => ".response.body.session_id", "DATETIME" => ".response.body.timestamp"],
            },
            request:  Request {
                body: json!({}),
                etc:  json!({"header": { "Authorization": "${USER_TOKEN}"}}),
                uri:  String::from("POST /logout/${USER_ID}"),
            },

            response: Response {
                body:   json!({
                  "message": "User ${USER_ID} logged out",
                  "session_id": "${SESSION_ID}",
                  "timestamp": "${DATETIME}"
                }),
                etc:    json!({}),
                status: 200,
            },
        },
        FRAME_JSON
    );
}
