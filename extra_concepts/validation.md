# Validation

## Partial Comparison:

Given a `validation` selector with a key of `{"partial": true}`, the
selected response object will non-recursively remove any elements in the actual
body that are not present in the Frame's response before attempting to compare
the actual and expected response.

<a name="listing-1"></a>

```jsonc
{
  "protocol": "HTTP",
  "request": {
    "entrypoint": "http://localhost:8080",
    "uri": "GET /object"
  },
  "response": {
    "validation": { 
      // partial validation will be applied to the elements of the response body
      "response.body": {
        "partial": true
      }
    },
    "body": {
      "desired_response": "Success!"
    },
    "status": 200
  }
}
```


**Listing 1:** The Frame file above will remove any keys that are not
`"desired_response"` from the response payload.

A partial comparison validator is meant to express: "I don't care about anything
extra except what I've specified in the Frame file".

Partial responses will only match for the exact value present found in the
selection with the exception of `{"objects": "and"}` `["arrays"]`:
* Objects will only be matched by the presence of keys and can will still fail
on a mismatch of values.
* Arrays will be searched for the _contiguous_ presence of the elements therein:
For a partial validation of array `[A,B,C]`, responses `[A,A,B,C]` and `[A,B,C]`
will result in a match but array `[A,A,B,B,C,C]` will not

## Unordered Comparison:

Given a `validation` selector with a key of `{"unordered": true}`, the
selected response object will attempt to order any elements in the actual
body as they are found in the Frame's response before attempting to compare
the actual and expected response.

<a name="listing-2"></a>

```jsonc
{
  "protocol": "HTTP",
  "request": {
    "entrypoint": "http://localhost:8080",
    "uri": "GET /object"
  },
  "response": {
    "validation": { 
      // unordered validation will be applied to the elements of the response body
      "response.body": {
        "unordered": true
      }
    },
    "body": {
      ["A", "B", "C"]
    },
    "status": 200
  }
}
```


**Listing 2:** The Frame file above will attempt, once per element given, to
place any string elements of values `"A"`, `"B"` or `"C"` to the front of the
actual response.

A partial comparison validator is meant to express: "I don't care about anything
extra except what I've specified in the Frame file".


## Unordered and Partial Comparison

Unordered and partial can also be applied at the same time to the same
selection.  This allows a validation that ignores the order and any additional
elements in the selection so long as the desired elements are present.

```jsonc
{
  "protocol": "HTTP",
  "request": {
    "entrypoint": "http://localhost:8080",
    "uri": "GET /object"
  },
  "response": {
    "validation": { 
      "response.body": {
        "partial": true,
        "unordered": true
      }
    },
    "body": {
      ["A", "B", "C"]
      // For the array above all
      // the elements below are valid matches:
      //
      // ["C", "B", "A", "A", "B", "C"]
      // ["C", "B", "A", false]
      // [["D"], "", "B", "C", "A"]
    },
    "status": 200
  }
}
```

**Listing 3:** The Frame file above will attempt, once per element given, to
place any string elements of values `"A"`, `"B"` or `"C"` to the front of the
actual response and remove additional elements from the match.
