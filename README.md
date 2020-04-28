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
  take              Takes a single frame, emitting the request then validating
                    the returned response
  record            Attempts to play through an entire Reel sequence running a
                    take for every frame in the sequence

```


`dark take`:

```
Usage: dark take <frame> [<address>] [--tls] [-H <header>] -c <cut> [-o <output>]

Takes a single frame, emitting the request then validating the returned response

Options:
  --tls             enable TLS
  -H, --header      fallback header passed to the specified protocol
  -c, --cut         filepath of input cut file
  -o, --output      output of take file
  --help            display usage information

```

`dark record`:

```
Usage: dark record <reel_path> <reel_name> [<merge_cuts...>] [--tls] [-a <address>] [-H <header>] [-c <cut>] [-o <output>] [-i]

Attempts to play through an entire Reel sequence running a take for every frame in the sequence

Options:
  --tls             enable TLS
  -a, --address     fallback address passed to the specified protocol if not
                    provided by the frame itself
  -H, --header      fallback header passed to the specified protocol if not
                    provided by the frame itself
  -c, --cut         filepath of input cut file
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


#### SOPS example:

```sh
# destructively merge FIFO sops "KEY_NAME" value into the in-memory cut register
dark record ./reel_path reel_name -c ./reel_name.cut.json \
    <(sops -d --extract '["KEY_NAME"]' path/to/myfile.enc.json)

# multiple merge cuts can be used, with values being overridden left to right (right will have newer values)
dark -v record -i ./test_data post -c ./post.cut.json \
    <(echo '{"new":"value"}') <(echo '{"newer": "value", "new":"overridden"}'
```

