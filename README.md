# Darkroom

<img src="darkroomlogo_mini.svg?sanitize=true" width="150"/>

A contract testing tool built in Rust using the [filmReel format](https://github.com/Bestowinc/filmReel).


```
Usage: dark [-v] <command> [<args>]

Top-level command.

Options:
  -v, --verbose     enable verbose output
  --help            display usage information

Commands:
  take              Takes a single frame, sends the request and compares the
                    returned response
  record            Attemps to play through an entire Reel sequence

```


`dark take`:

```
Usage: dark take <frame> -H <header> [--proto <proto>] -a <addr> -c <cut> [-o <output>]

Takes a single frame, sends the request and compares the returned response

Options:
  -H, --header      args passed to grpcurl
  --proto           pass proto files used for payload forming
  -a, --addr        address passed to grpcurl
  -c, --cut         filepath of cut file
  -o, --output      output of take file
  --help            display usage information


```

`dark record`:

```
Usage: dark record <path> <name> -H <header> [--proto <proto>] -a <addr> [-c <cut>] [-o <output>] [-i]

Attemps to play through an entire Reel sequence

Options:
  -H, --header      header string passed to grpcurl
  --proto           pass proto files used for payload forming
  -a, --addr        address passed to grpcurl
  -c, --cut         filepath of output cut file
  -o, --output      output directory for successful takes
  -i, --interactive interactive frame sequence transitions
  --help            display usage information

```

### New in `0.2`:

* HTTP support
* Full json object storage and retrieval, the cut register is no longer a flat associative array, strings are still used to map to JSON objects for templating
* Variable discarding: `${lowercase}` variables will only be kept around for the duration of the frame
* Headers and entrypoints can be stored and read on a per JSON frame basis
* SOPS/json secrets support

