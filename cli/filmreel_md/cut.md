# The Cut

[From Wikipedia](https://en.wikipedia.org/wiki/Director%27s_cut):
> Traditionally, the "director's cut" is not, by definition, the director's ideal or preferred cut. The editing process of a film is broken into stages:
> * First is the assembly/rough cut, where all selected takes are put together in the order in which they should appear in the film.
> * Next, the editor's cut is reduced from the rough cut; the editor may be guided by his own tastes or following notes from the director or producers.
> * Eventually is the final cut, which actually gets released or broadcast.
> * In between the editor's cut and the final cut can come any number of fine cuts, including the director's cut.

## Register for seeding template data

A *Cut* allows data to be stored and propagated to Frames in a *Reel* sequence
using instructions held in a Frame's
[*Cut Instruction Set*](frame.md#cut-instruction-set).

<a name="cut"></a>

`Cut File` - a JSON file starting with a [*Reel name*](reel.md#reel-name) and
ending with the suffix `.cut.json`.
* Ex: For a Reel with a name of `product`, the associated Cut file will be named `product.cut.json`.
* Holds a single *Cut Register* object.

<a name="cut-register"></a>

`Cut Register` - The JSON object stored in the *Cut* file.
* Is an [associative array](https://en.wikipedia.org/wiki/Associative_array)
containing valid JSON objects.
* Holds *Cut Variables* and their corresponding values stored in a series of
key/value pairs.

<a name="listing-1"></a>


```json
{
  "FIRST_NAME": "Primus",
  "RESPONSE": "ALRIGHT"
}
```

**Listing 1:** The *Cut* file `usr.cut.json` with a *Cut Register* holding the
string values for the *Cut Variables* `FIRST_NAME` and `RESPONSE`.

<a name="cut-variable"></a>

`Cut Variable`- a named variable that references a single string value in the
*Cut Register*.

* Cut Variables referenced in a Frame's request/response bodies must be :
  * prepended with a left delimiter - `${`
  * and terminated with a right delimiter - `}`.
* Ex: The Cut Variable `USER_ID` found in the Cut Register will be referenced
as-is in a Frame's Cut instruction set and as `${USER_ID}` in the
request/response objects.
* Mimics [shell parameter expansion](https://ss64.com/bash/syntax-expand.html) syntax.
* The left delimiter can be escaped by placing two backslashes before the
dollar sign: `\\$`.

<a name="listing-2"></a>

```json
{
  "protocol": "HTTP",
  "cut": {
    "from": [
      "USER_ID"
    ]
  },
  "request": {
    "uri": "POST /logout/${USER_ID}"
  },
  "response": {
    "body": "User:${USER_ID} logged out",
    "status": 200
  }
}
```

**Listing 2:** An example *Frame* that logs out a user using a *Cut Variable* to build the request URI.


<a name="cut-operation"></a>

`Cut operation` - A procedure that uses instructions held in a Frame's
[*Cut Instruction Set*](frame.md#cut-instruction-set) to read and write
*Cut Variables* to the *Cut Register*.
<a name="from-to"></a>
* Reads `"from"` and writes `"to"` the *Cut Register* .

<a name="read-operation"></a>
`Read operation` - A Cut operation that uses a *read instruction* to insert a
*Cut Variable* into a *Frame's* *request/response* objects.
* Uses the read instructions held in a Cut Instruction Set to populate
Cut Variable references found in a Frame.
* Executed *before* a Frame' request payload is sent.

<a name="write-operation"></a>
`Write operation` - A Cut operation that takes a *Frame's* *Write instruction* object and creates or reassigns the matching key value found in the *Cut Register*.
* Uses a syntax similar to
[jq filters](https://stedolan.github.io/jq/manual/#Basicfilters) for JSON
traversal.
* Executed *after* a Frame's request payload is sent.
* Operates on variable references inside response object only.

<a name="listing-3"></a>

```json
{
  "protocol": "HTTP",
  "request": {
    "uri": "POST /logout/\\${USER_ID}"
  },
  "response": {
    "status": 200
  }
}
```

**Listing 3:** When interpreting the Frame file above, the *request uri* value
will be processed as `"POST /logout/${USER_ID}"` due to the escaped left
delimiter.

## Cut Errors

`Register Parse Error` - returned when:

* A Cut Register key maps to a non-string data type.

`Frame Parse Error` - returned when:

* A Frame string contains a dollar sign and open brace `${` that is not later
followed by a matching close brace `}`.
* A Frame's request/response objects hold Cut Variable references not found in
the Cut Instruction Set.
* The `"from"` key in a Frame instruction set does not map to an array of
strings.
* The `"to"` key in a Frame instruction set does not map to an associative
array of strings.

`General Instruction Error` - returned when:
* A Cut instruction fails to return a string holding the equivalent Cut
Variable reference.

`Read Instruction Error` - returned when:

* A Frame's read instruction holds a Cut Variable reference that is missing
from the Frame's request/response objects.
* The variable referenced in a Frame's Cut instruction is not found in the Cut
Register.

`Write Instruction Error` - returned when:

* A Write operation attempts to handle a non-string data type.
* A Write instruction returns multiple references of the same Cut Variable.

<a name="listing-4"></a>

 ```jsonc
{
  "protocol": "HTTP",
  "request": {
    "body": {
      "full_name": "Primus ${LAST_NAME" // Frame Parse Error
    },
    "uri": "GET /email/by_name"
  },
  "response": {
    "body": {
      "email": "p.ultimus@humanmail.com"
    },
    "status": 200
  }
}
```

**Listing 4:** While the object above is valid JSON, one should expect it to
return a *Frame Parse Error* if used in a filmReel implementation.


## filmReel concepts:

* [The Frame](frame.md)
* [The Reel](reel.md)
* [The Cut](cut.md)
