# Frame
* A JSON file ending with the suffix `.fr.json`
* ##### **Cut Instruction Set** - A JSON object holding Read and Write instructions that push and pull variables `"from"` and `"to"` the **Cut Register** through **Cut operations**
* ##### **Request** - A [JSON object](https://en.wikipedia.org/wiki/JSON#Data_types_and_syntax) that fully defines how the *Frame* payload is built and sent.
* ##### **Response** - A JSON object that defines the expectations for the contents of a response message.

```jsonc
{
  "protocol": "HTTP",
  "cut": {
    "from": [                        // pull from the register
      "FIRST_NAME"
    ],
    "to": ".response.body.last_name" // send to the register
  },
  "request": {
    "body": {
      "first_name": "${FIRST_NAME}"
    },
    "entrypoint": "website.com",
    // HTTP GET from "website.com/last_name/by_fname/Tom"
    "uri": "GET /last_name/by_fname/${FIRST_NAME}"
  },
  "response": {
    "body": {
      "last_name": "${LAST_NAME}"
    },
    "status": 200
  }
}

```
