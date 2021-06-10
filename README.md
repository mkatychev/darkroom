# Darkroom  <img src="https://raw.githubusercontent.com/mkatychev/darkroom/master/darkroomlogo_mini.svg" width="149" align="right"/>

[![Crates.io](https://img.shields.io/crates/v/darkroom.svg)](https://crates.io/crates/darkroom) [![Docs.rs](https://docs.rs/darkroom/badge.svg)](https://docs.rs/darkroom/)



A contract testing tool built in Rust using the [filmReel format](https://github.com/mkatychev/filmReel).

---

## [Sample request](https://github.com/mkatychev/filmreel/blob/master/frame.md#listing-1):


[`usr.cut.json`](https://github.com/Bestowinc/filmReel/blob/master/cut.md#cut-register):
```jsonc
// Cut: the data sharing system allowing one Frame to pass messages to the next Frame
{"HTTP_ENDPOINT": "/create_user"}
```

[`usr.01s.createuser.fr.json`](https://github.com/Bestowinc/filmReel/blob/master/frame.md#frame-nomenclature):

```jsonc
// Frame: the JSON file where input an output expectations are set
{                                          
  // protocol: the declared communication protocol
  "protocol": "HTTP",
  // cut: declare what variables should be pulled "from" and pushed "to" `usr.cut.json`
  "cut": {                                 
    // pull the HTTP_ENDPOINT "from" `usr.cut.json`
    "from": ["HTTP_ENDPOINT"],
    // push the USER_ID found in .response.body.msg "to" `usr.cut.json`
    "to": {
      "USER_ID": "'response'.'body'.'msg'" 
    }
  },
  "request": {                             // Request object
    "body": {                              // Request body
      "email": "new_user@humanmail.com"
    },
    "uri": "POST ${HTTP_ENDPOINT}"         // Request uri: HTTP_ENDPOINT will be replaced by "/create_user"
  },
  "response": {                            // Response object
    "body": {                              // Response body
      "msg": "created user: ${USER_ID}"    // USER_ID will be stored if there is a match for the surrounding values
    },
    "status": 200                          // expected response status code
  }
}
```

## Installation

* Simple: `cargo install --git https://github.com/mkatychev/darkroom`
* Clone with submodules: `git clone --recurse-submodules -j8 https://github.com/mkatychev/darkroom`

&nbsp;


For gRPC requests: Darkroom `0.3` or greater requires [grpcurl v1.6.0 or greater](https://github.com/fullstorydev/grpcurl/#installation) for making gRPC requests.


## Usage:


`dark`:

<!-- dark start -->
```
Usage: dark [<address>] [-v] [-H <header>] [--cut-out <file>] [-i] [--tls] [--proto-dir <dir...>] [-p <file...>] <command> [<args>]

Darkroom: A contract testing tool built in Rust using the filmReel format.

Options:
  -v, --verbose     enable verbose output
  -H, --header      fallback header passed to the specified protocol
  --cut-out         output of final cut file
  -i, --interactive interactive frame sequence transitions
  --tls             enable TLS (automatically inferred for HTTP/S)
  --proto-dir       the path to a directory from which proto sources can be
                    imported, for use with --proto flags.
  -p, --proto       pass proto files used for payload forming
  --help            display usage information

Commands:
  version           Returns CARGO_PKG_VERSION
  take              Takes a single frame, emitting the request then validating
                    the returned response
  record            Attempts to play through an entire Reel sequence running a
                    take for every frame in the sequence
  man               return a given manual entry
  vrecord           Attempts to play through an entire VirtualReel sequence
                    running a take for every frame in the sequence

Examples:
  Step through the httpbin test in [-i]nteractive mode:
      $ dark -i record ./test_data post
  Echo the origin `${IP}` that gets written to the cut register from the httpbin.org POST request:
      $ dark --cut-out >(jq .IP) take ./test_data/post.01s.body.fr.json
  Run the post reel in a v-reel setup:
      $ dark vrecord ./test_data/post.vr.json

Notes:
  Use `dark man` for details on filmReel, the JSON format.

```
<!-- dark stop -->

## Examples:

```sh
# step through the httpbin test in [-i]nteractive mode
dark -i record ./test_data post
# to fail at the third httpbin frame, set a timeout of two seconds
dark --interactive record ./test_data post --timeout 2
# multiple merge cuts can be used, with values being overridden left to right (right will have newer values)
dark --interactive record ./test_data post --cut ./test_data/post.cut.json '{"NEW":"value"}' '{"NEWER": "value", "NEW":"overridden"}'
# echo the origin "${IP}" that gets written to the cut register from the httpbin.org POST response
dark --cut-out >(jq .IP) take ./test_data/post.01s.body.fr.json --cut ./test_data/post.cut.json
# create a stripe token using the public Stripe API key
dark --verbose --cut-out >(jq) record ./test_data stripe_token
# create a stripe subscription preceding it with the stripe_token flow
dark --cut-out >(jq) record ./test_data stripe_subscription --component './test_data&stripe_token'
```

## CHANGELOG

Please see the [CHANGELOG](CHANGELOG.md) for a release history.

<!--
VERSION="0.6.0"
DR_DIR=$PWD
GRPCURL_DIR=${GRPCURL_DIR:-../grpcurl}
cargo build --release && \
tar czf darkroom-"$VERSION"-x86_64-apple-darwin.tar.gz -C target/release dark && \
docker run --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release && \
tar czf darkroom-"$VERSION"-x86_64-unknown-linux-musl.tar.gz -C ./target/x86_64-unknown-linux-musl/release dark
(cd $GRPCURL_DIR; env CGO_ENABLED=0 GOOS=darwin GOARCH=amd64 go build -a -o $DR_DIR/target/release/grpcurl ./cmd/grpcurl) && \
tar czf darkroom-"$VERSION"-grpcurl-x86_64-apple-darwin.tar.gz -C target/release dark grpcurl && \
(cd $GRPCURL_DIR; env CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -a -o $DR_DIR/target/x86_64-unknown-linux-musl/release/grpcurl ./cmd/grpcurl) && \
tar czf darkroom-"$VERSION"-grpcurl-x86_64-unknown-linux-musl.tar.gz -C ./target/x86_64-unknown-linux-musl/release dark grpcurl
-->
