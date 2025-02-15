# The Frame

## Encoding input/output expectations into a JSON file

A *Frame* provides the means for demonstrating how an API method should behave.
Once a Frame request object is turned into a payload and sent, the returning
payload is then compared against the Frame response object.

<a name="frame"></a>

`Frame` - A JSON file ending with the suffix `.fr.json`
* must contain the *protocol*, *request*, and *response* objects.
* can optionally hold a *Cut Instruction Set*.

A Frame must provide a string value for the `"protocol"` key indicating the
type of communication protocol used to ferry the payloads found in the payload
bodies.

## Frame nomenclature

<a name="listing-1"></a>

```jsonc
{
  "protocol": "gRPC",                       // communication protocol
  "cut": {                                  // Cut Instruction Set
    "to": {
      "USER_ID": "'response'.'body'.'response'"  // Write instruction
    }
  },
  "request": {                              // request object
    "body": {
      "email": "new_user@humanmail.com"
    },
    "uri": "user_api.User/CreateUser"       // request uri
  },
  "response": {                             // response object
    "body": {
      "message": "created user: ${USER_ID}"
    },
    "status": 0                             // response status code
  }
}

```

**Listing 1:** The example found in the README, now annotated

<a name="request"></a>

`Request` - A [JSON object](https://en.wikipedia.org/wiki/JSON#Data_types_and_syntax) that fully
defines how the *Frame* payload is built and sent.
* A request's `"uri"` key must map to a string value specifying a fully formed
[request URI](https://www.w3.org/Protocols/rfc2616/rfc2616-sec5.html#sec5.1.2)
for the protocol defined.
* Must contain a *body* object even if no body data is to be sent.
If no [cut `${VARIABLES}`](cut.md#cut-variable) are specified in the body
then the payload will be sent as-is.

<a name="listing-2"></a>

```json
{
  "protocol": "HTTP",
  "request": {
    "body": {
      "full_name": "Primus Ultimus"
    },
    "uri": "GET /email/by_name"
  },
  "response": {
    "body": {
      "email": "p.ultimus@humanmail.com"
    },
    "status": 200
  }
}
```

**Listing 2:** An example *Frame* that maps the request URI to an HTTP method and endpoint.


<a name="response"></a>

`Response` - A JSON object that defines the expectations for the contents of a
response message.
* The response key must map to a body and status field.
* The response `"status"` key is intended to map to a protocol's status code.

<a name="listing-3"></a>

```json
{
  "protocol": "gRPC",
  "request": {
    "body": {
      "full_name": "Primus Secundus"
    },
    "uri": "user_api.User/GetEmail"
  },
  "response": {
    "body": "email not found for the name provided",
    "status": 5
  }
}
```

**Listing 3:** A *Frame* file that expects an error message in the response
body and a ["Not Found"](https://github.com/grpc/grpc/blob/master/doc/statuscodes.md)
status code in the response status.

<a name="cut-instruction-set"></a>

`Cut Instruction Set` - A JSON object holding Read and Write instructions that
push and pull variables `"from"` and `"to"` the *Cut Register* through
[*Cut operations*](cut.md#cut-operation).

* Must contain one or both of these key names: `"from"` and `"to"`.
* The `"from"` key holds an array of `Read instructions`.
* The `"to"` key holds an associative array of `Write instructions`.

<a name="listing-4"></a>

```json
{
  "protocol": "HTTP",
  "cut": {
    "from": ["FIRST_NAME"]
  },
  "request": {
    "body": {
      "first_name": "${FIRST_NAME}"
    },
  "uri": "GET /last_name/by_first_name"
  },
  "response": {
    "body": {
      "last_name": "Ultimus"
    },
    "status": 200
  }
}
```

**Listing 4:** A *Frame* file containing one *Read instruction* in the *Cut
Instruction Set*. <sub>[associated Cut Register](cut.md#listing-1)</sub>

## filmReel concepts:

* [The Frame](frame.md)
* [The Reel](reel.md)
* [The Cut](cut.md)
