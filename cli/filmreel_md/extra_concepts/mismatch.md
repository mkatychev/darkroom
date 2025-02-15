Form Mismatch 🌋: 
* output during `run_request` when the returned JSON does not match the expected structure/shape
* Ex: `{"body":["array"]}` and `{"body":"string"}` should return a _Form Mismatch_

Value Mismatch 🤷:
 * output during `process_response` when the returned JSON values do not match
* Ex: `{"body":["array"]}` and `{"body":["some_other_data"]}` should return a _Value Mismatch_

