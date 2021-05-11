# Darkroom  <img src="https://raw.githubusercontent.com/Bestowinc/darkroom/master/darkroomlogo_mini.svg" width="149" align="right"/>

[![Crates.io](https://img.shields.io/crates/v/darkroom.svg)](https://crates.io/crates/darkroom) [![Docs.rs](https://docs.rs/darkroom/badge.svg)](https://docs.rs/darkroom/)



A contract testing tool built in Rust using the [filmReel format](https://github.com/Bestowinc/filmReel).

---

* Darkroom `0.3` or greater requires [grpcurl v1.6.0 or greater](https://github.com/fullstorydev/grpcurl/#installation) for making gRPC requests.


## Usage:


`dark`:

<!-- dark start -->
```
Usage: dark [<address>] [-v] [-H <header>] [--cut-out <file>] [-i] [--tls] [--proto-dir <dir...>] [-p <proto...>] <command> [<args>]

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

Examples:
  Step through the httpbin test in [-i]nteractive mode:
  $ dark -i record ./test_data post
  
  Echo the origin `${IP}` that gets written to the cut register from the httpbin.org POST request:
  $ dark --cut-out >(jq .IP) take ./test_data/post.01s.body.fr.json

Notes:
  Use `dark man` for details on filmReel, the JSON format.

```
<!-- dark stop -->

## Examples:

```sh
# step through the httpbin test in [-i]nteractive mode
dark -i record ./test_data post
# to fail at the third httpbin frame, set a timeout of two seconds
dark -i record ./test_data post --timeout 2
# multiple merge cuts can be used, with values being overridden left to right (right will have newer values)
dark --interactive record ./test_data post --cut ./test_data/post.cut.json '{"NEW":"value"}' '{"NEWER": "value", "NEW":"overridden"}'
# echo the origin "${IP}" that gets written to the cut register from the httpbin.org POST request
dark --cut-out >(jq .IP) take ./test_data/post.01s.body.fr.json --cut ./test_data/post.cut.json
# create a stripe token using public API key
dark --cut-out >(jq) record ./test_data stripe_token
# create a stripe subscription using the stripoe_token component
dark --cut-out >(jq) record ./test_data stripe_subscription --component './test_data&stripe_token'
```

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
