# Darkroom

<img src="darkroomlogo_mini.svg?sanitize=true" width="300"/>

A contract testing tool built in Rust.


```
Usage: target/debug/dark [-v] <command> [<args>]

Top-level command.

Options:
  -v, --verbose     enable verbose output
  --help            display usage information

Commands:
  take              Takes a single frame, sends the request and compares the
                    returned response
  record            Attemps to play through an entire Reel sequence

```
