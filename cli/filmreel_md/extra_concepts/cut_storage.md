## Full JSON object storage 

A *Cut Variable* can store any valid JSON object:

- This frame:
```json
{
  "protocol": "HTTP",
  "cut": {
    "to": {
      "COMPLEX_OBJECT": "'response'.'body'.'complex_object'"
    }
  },
  "request": {
    "uri": "GET /complex_object"
  },
  "response": {
    "body": {
      "complex_object": "${COMPLEX_OBJECT}"
    },
    "status": 200
  }
}
```

- With this response:
```json
{
  "complex_object": {
    "a_lot_of_stuff": {
      "things": [
        1,
        2,
        3,
        4
      ]
    }
  }
}
```

- Gives us this *Cut File*:
```json
{
  "COMPLEX_OBJECT": {
    "a_lot_of_stuff": {
      "things": [
        1,
        2,
        3,
        4
      ]
    }
  }
}
```
