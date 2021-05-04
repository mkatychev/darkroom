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
  $ dark --cut-out >(jq .IP) take ./test_data/post.01s.body.fr.json --cut ./test_data/post.cut.json

Notes:
  Use `dark man` for details on filmReel, the JSON format.

```
<!-- dark stop -->


`dark take`:

<!-- dark take start -->
```
Usage: dark take <frame> -c <cut> [-o <file>]

Takes a single frame, emitting the request then validating the returned response

Options:
  -c, --cut         filepath of input cut file
  -o, --take-out    output of take file
  --help            display usage information

```
<!-- dark take stop -->

`dark record`:

<!-- dark record start -->
```
Usage: dark record <reel_path> <reel_name> [<merge_cuts...>] [-c <cut>] [-b <component...>] [-o <take-out>] [-r <range>] [-t <timeout>] [-s] [-d]

Attempts to play through an entire Reel sequence running a take for every frame in the sequence

Options:
  -c, --cut         filepath of input cut file
  -b, --component   repeatable component reel pattern using an ampersand
                    separator: --component "<dir>&<reel_name>"
  -o, --take-out    output directory for successful takes
  -r, --range       the range (inclusive) of frames that a record session will
                    use, colon separated: --range <start>:<end> --range <start>:
  -t, --timeout     client request timeout in seconds, --timeout 0 disables
                    request timeout [default: 30]
  -s, --timestamp   print timestamp at take start, error return, and reel
                    completion
  -d, --duration    print total time elapsed from record start to completion
  --help            display usage information

```
<!-- dark record stop -->

## Examples:

```sh
# step through the httpbin test in [-i]nteractive mode
dark -i record ./test_data post
# to fail at the third httpbin frame, set a timeout of two seconds
dark -i record ./test_data post --timeout 2
# multiple merge cuts can be used, with values being overridden left to right (right will have newer values)
dark -v --interactive record ./test_data post --cut ./test_data/post.cut.json \
    <(echo '{"new":"value"}') <(echo '{"newer": "value", "new":"overridden"}')
# echo the origin "${IP}" that gets written to the cut register from the httpbin.org POST request
dark --cut-out >(jq .IP) take ./test_data/post.01s.body.fr.json --cut ./test_data/post.cut.json
# create a stripe token using public API key
dark --cut-out >(jq) record ./test_data stripe_token
# create a stripe subscription using the stripoe_token component
dark --cut-out >(jq) record ./test_data stripe_subscription --component './test_data&stripe_token'
```


## Changes:

#### `0.2`:

* HTTP support
* added `form` key to HTTP frame requests: `{"request":{"uri":"POST post","form":{"key":"val","array[0]":"val0"}}}`
* full JSON object storage and retrieval, the cut register is no longer a flat associative array, strings are still used to map to JSON objects for templating
* variable discarding: `${lowercase}` variables will only be kept around for the duration of the frame
* headers and entrypoints can be stored and read on a per JSON frame basis
* SOPS/JSON secrets support

#### `0.2.1`:

* added hidden variable support, hidden variables are defined with a leading underscore: `${_HIDDEN}`
* added `dark version` command
* moved common parameters into the main `dark` command to be shared across subcommands

#### `0.2.3`:

* added component reel support, component reels are generated as a prelude to the provided reel   `dark record --component "<dir>&<reel_name>" ./dir/ my_reel_name`
* added anyhow error handling
* `--cut-out` can now be returned on a failed `record` or `take`

#### `0.3.0`:

* removed YAML deserialization now that `grpcurl` properly emits JSON errors
* added retry `attempts` to frame requests: `{"request":{"attempts": {"times": 5, "ms": 500}}}`

#### `0.3.1`:

* frame response body is now optional

#### `0.3.2`:

* request retry attempts now include a `process_response` comparison
* `ToTakeHiddenColouredJson` is now a generic trait
* `ToStringHidden` is now a generc trait
* moved styler out of take.rs and into lib.rs

#### `0.3.3`:

* range is added to recordings: `dark record --range "<start_u32>:<end_u32>" ./dir/ my_reel_name`
* `grpcurl` errors propagate to stdout properly
* `"request"["form"]` request building URL functionality moved to `"request"["query"]`
* `"request"["form"]` now properly bulids the form data of the HTTP request

#### `0.4.0`:

* range is added to recordings: `dark record --range "<start_u32>:<end_u32>" ./dir/ my_reel_name`

#### `0.5.0`:

* timestamps are added to recordings: `dark record record ./test_data post --timestamp`
* 30 sec default timeout can now be overridden: `dark record record ./test_data post --timeout 2`
* reel sequence numbers are now checked for duplicates

#### `0.6.0`:
* added `--import-path` for `dark --proto` arguments specifying paths for proto definition lookup
* added `dark record --duration` to display total time elapsed in a record session

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
