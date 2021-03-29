use crate::{frame::*, from, response::*, to};
use serde_json::json;

/// test_ser_de tests the serialization and deserialization of frame structs
///
/// ```edition2018
///  test_ser_de!(
///      protocol_grpc,      // test name
///      Protocol::GRPC,     // struct/enum to test
///      PROTOCOL_GRPC_JSON  // json string to test
/// );
/// ```
#[macro_export]
macro_rules! test_ser_de {
    ($name:ident, $struct:expr, $json_str:expr) => {
        paste::paste! {
            #[test]
            fn [<$name _ser>]() {
                let val_from_str: serde_json::Value = serde_json::from_str($json_str).unwrap();
                let val_from_struct = serde_json::value::to_value(&$struct).unwrap();
                pretty_assertions::assert_eq!(val_from_str, val_from_struct);
            }
            #[test]
            fn [<$name _de>]() {
                crate::serde_tests::test_deserialize($struct, $json_str)
            }
        }
    };
}

pub(crate) fn test_deserialize<'a, T>(de_json: T, json: &'a str)
where
    T: serde::Deserialize<'a> + PartialEq + std::fmt::Debug,
{
    let actual = serde_json::from_str(json).unwrap();
    pretty_assertions::assert_eq!(de_json, actual);
}

const PROTOCOL_GRPC_JSON: &str = r#""gRPC""#;
test_ser_de!(protocol_grpc, Protocol::GRPC, PROTOCOL_GRPC_JSON);

const PROTOCOL_HTTP_JSON: &str = r#""HTTP""#;
test_ser_de!(protocol_http, Protocol::HTTP, PROTOCOL_HTTP_JSON);

const REQUEST_JSON: &str = r#"
{
  "body": {
    "email": "new_user@humanmail.com"
  },
  "uri": "user_api.User/CreateUser"
}
    "#;
test_ser_de!(
    request,
    Request {
        body: Some(json!({"email": "new_user@humanmail.com"})),
        uri: json!("user_api.User/CreateUser"),
        ..Default::default()
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
    request_etc,
    Request {
        body:       Some(json!({})),
        header:     Some(json!({"Authorization": "${USER_TOKEN}"})),
        entrypoint: None,
        etc:        Some(json!({"id": "007"})),
        uri:        json!("POST /logout/${USER_ID}"),
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
    response,
    Response {
        body: Some(json!("created user: ${USER_ID}")),
        status: 0,
        ..Default::default()
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
    response_etc,
    Response {
        body: Some(json!("created user: ${USER_ID}")),
        etc: Some(json!({"user_level": "admin"})),
        status: 0,
        ..Default::default()
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
    instruction_set,
    InstructionSet {
        reads:          from!["USER_ID", "USER_TOKEN"],
        writes:         to!({
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
    frame,
    Frame {
        protocol: Protocol::HTTP,
        cut:      InstructionSet {
            reads:          from!["USER_ID", "USER_TOKEN"],
            writes:         to!({
                "SESSION_ID" => ".response.body.session_id",
                "DATETIME" => ".response.body.timestamp"
            }),
            hydrate_writes: false,
        },
        request:  Request {
            body: Some(json!({})),
            header: Some(json!({ "Authorization": "${USER_TOKEN}" })),
            uri: json!("POST /logout/${USER_ID}"),
            ..Default::default()
        },

        response: Response {
            body: Some(json!({
              "message": "User ${USER_ID} logged out",
              "session_id": "${SESSION_ID}",
              "timestamp": "${DATETIME}"
            })),
            status: 200,
            ..Default::default()
        },
    },
    FRAME_JSON
);
const SIMPLE_FRAME_JSON: &str = r#"
{
  "protocol": "HTTP",
  "request": {
    "uri": "POST /logout/${USER_ID}"
  },
  "response": {
    "status": 200
  }
}
    "#;
test_ser_de!(
    simple_frame,
    Frame {
        protocol: Protocol::HTTP,
        cut:      InstructionSet::default(),
        request:  Request {
            uri: json!("POST /logout/${USER_ID}"),
            ..Default::default()
        },

        response: Response {
            status: 200,
            ..Default::default()
        },
    },
    SIMPLE_FRAME_JSON
);
