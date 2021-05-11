#### `0.7.0`:
* added `dark man` command for additional documentation, can be excluded from build with the `--no-default-features` flag
* added partial and unordered response validations to filmreel
* implemented subset of `jql` for mutable selections of the resposne json
  the `jql` library can be included with the `--features=filmreel/full_jql` flag
* an empty body in the request is now appropriately represented as a missing `body` key rather than `"body": {}`
* `take` and `record` can now accept a mixture of filepaths and json strings:
  `dark take ./test_data/post.01s.body.fr.json '{"new":"value"}' ./test_data/post.cut.json`
* `dark take` command simplified, if `--cut` not provided, will try to look for a cut file in the same directory as the given frame json

#### `0.6.0`:
* added `--import-path` for `dark --proto` arguments specifying paths for proto definition lookup
* added `dark record --duration` to display total time elapsed in a record session

#### `0.5.0`:

* timestamps are added to recordings: `dark record record ./test_data post --timestamp`
* 30 sec default timeout can now be overridden: `dark record record ./test_data post --timeout 2`
* reel sequence numbers are now checked for duplicates


#### `0.4.0`:

* range is added to recordings: `dark record --range "<start_u32>:<end_u32>" ./dir/ my_reel_name`


#### `0.3.3`:

* range is added to recordings: `dark record --range "<start_u32>:<end_u32>" ./dir/ my_reel_name`
* `grpcurl` errors propagate to stdout properly
* `"request"["form"]` request building URL functionality moved to `"request"["query"]`
* `"request"["form"]` now properly bulids the form data of the HTTP request


#### `0.3.2`:

* request retry attempts now include a `process_response` comparison
* `ToTakeHiddenColouredJson` is now a generic trait
* `ToStringHidden` is now a generc trait
* moved styler out of take.rs and into lib.rs


#### `0.3.1`:

* frame response body is now optional


#### `0.3.0`:

* removed YAML deserialization now that `grpcurl` properly emits JSON errors
* added retry `attempts` to frame requests: `{"request":{"attempts": {"times": 5, "ms": 500}}}`


#### `0.2.3`:

* added component reel support, component reels are generated as a prelude to the provided reel   `dark record --component "<dir>&<reel_name>" ./dir/ my_reel_name`
* added anyhow error handling
* `--cut-out` can now be returned on a failed `record` or `take`


#### `0.2.1`:

* added hidden variable support, hidden variables are defined with a leading underscore: `${_HIDDEN}`
* added `dark version` command
* moved common parameters into the main `dark` command to be shared across subcommands


#### `0.2`:

* HTTP support
* added `form` key to HTTP frame requests: `{"request":{"uri":"POST post","form":{"key":"val","array[0]":"val0"}}}`
* full JSON object storage and retrieval, the cut register is no longer a flat associative array, strings are still used to map to JSON objects for templating
* variable discarding: `${lowercase}` variables will only be kept around for the duration of the frame
* headers and entrypoints can be stored and read on a per JSON frame basis
* SOPS/JSON secrets support

