# filmReel <img src="images/filmreel.svg" width="65"/>

A format for encoding API state flow expectations.

The *filmReel* specification aims to move the source of truth for how an API
should behave from the service itself and into a series of linear state
diagrams. This allows one to set loosely coupled and human readable
expectations for an API.

By having filmReel be the source of truth for how an API should behave one can:

* Identify regressions in service logic.
* Determine whether mocks or stubs that mimic a service have become outdated.
* Aggregate data to support stateful integration tests.

## filmReel concepts:

* [The Frame](frame.md) - the JSON file where input an output expectations are set
* [The Reel](reel.md)   - the file naming system tying Frames together
* [The Cut](cut.md)     - the data sharing system allowing one Frame to pass messages to the next Frame


#### Example: <sub>[annotated](frame.md#listing-1)</sub>

`usr.01s.createuser.fr.json` <sub>[naming a file](reel.md#reel-nomenclature)</sub>

```json
{
  "protocol": "gRPC",
  "cut": {
    "to": {
      "USER_ID": "'response'.'body'.'message'"
    }
  },
  "request": {
    "body": {
      "email": "new_user@humanmail.com"
    },
    "uri": "user_api.User/CreateUser"
  },
  "response": {
    "body": {
      "message": "created user: ${USER_ID}"
    },
    "status": 0
  }
}
```
