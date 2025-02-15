## Retry Attempts:

* retry `attempts` are held in the `request` object: `{"request":{"attempts": {"times": 5, "ms": 500}}}`

This *Frame* will try up to 5 times to get a correct response match before
terminating with an error, waiting 500 milliseconds or half a second between
requests:

```json
{
  "protocol": "HTTP",
  "cut": {
    "to": {
      "OBJECT": "'response'.'body'.'object'"
    }
  },
  "request": {
    "uri": "GET /object",
    "attempts": {
      "times": 5,
      "ms": 500
    }
  },
  "response": {
    "body": {
      "complex_object": "${OBJECT}"
    },
    "status": 200
  }
}
```
