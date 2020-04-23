# Darkroom

<img src="darkroomlogo_mini.svg?sanitize=true" width="150"/>

A contract testing tool built in Rust using the [filmReel format](https://github.com/Bestowinc/filmReel).


```
Usage: target/debug/dark [-v] <command> [<args>]

Top-level command.

Options:
  -v, --verbose     enable verbose output
  --help            display usage information

Commands:
  take              Takes a single frame, sends the request and compares the
                    returned response
  record            Attempts to play through an entire Reel sequence

```


`dark take`:

```
Usage: target/debug/dark take <frame> [<addr>] -c <cut> [-H <header>] [-o <output>]

Takes a single frame, sends the request and compares the returned response

Options:
  -c, --cut         filepath of cut file
  -H, --header      args passed to grpcurl
  -o, --output      output of take file
  --help            display usage information

```

`dark record`:

```
Usage: target/debug/dark record <path> <name> [-H <header>] [-a <addr>] [-c <cut>] [-o <output>] [-i]

Attempts to play through an entire Reel sequence

Options:
  -H, --header      header string passed to grpcurl
  -a, --addr        address passed to grpcurl
  -c, --cut         filepath of output cut file
  -o, --output      output directory for successful takes
  -i, --interactive interactive frame sequence transitions
  --help            display usage information

```
