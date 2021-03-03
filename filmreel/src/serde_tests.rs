use crate::{frame::*, from, test_ser_de, to};
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
