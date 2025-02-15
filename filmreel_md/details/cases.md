## Implementation Cases TODO:

* Detecting API documentation examples that no longer reflect the true behaviour of a service.
* http://acid3.acidtests.org/ (similar to this, provide edge cases for implementation unit tests)

## Cut
* Upon the start of a reel flow, the cut file is copied to a hidden file with a `.` prepended to the name:  `.some_reel_name.cut.json`
* One must manually add values written to the *carbon copy* to prevent committing sensitive information.  (should this be further elaborated?)

<a name="listing-3"></a>

```json
{
  "USER_ID": "Hannibal_Barca",
  "USER_TOKEN": "@LpIne"
}
```

```json
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
```
