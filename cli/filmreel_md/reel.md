# The Reel

## Representing state flow through file sequences

A *Reel* ties together Frames into a linear state flow model. The successful
execution of all [*Frames*](frame.md#frame) belonging to a Reel signifies the
end of a complete object state transition.

<a name="reel"></a>
`Reel` - the sum of all *Frame*s in a directory that share the same *Reel name*:

* Represents a linear state flow diagram.

<a name="final-frame"></a>
`Final Frame` - the last *Success Frame* of a *Reel* sequence:

* Contains a return body holding the final desired state of a particular object.
* A Reel's intent is tied to the existence of a Final Frame, in other words,
the return body of the final Frame is the keystone of any Reel sequence.

The directory shown below holds the Frames needed to create a fully populated
"User" object starting with user creation and ending with a query that returns
the expected "User":

<a name="listing-1"></a>

```
user_reel
├── usr.01e.createuser.fr.json
├── usr.01s.createuser.fr.json
├── usr.01se.createuser.fr.json
├── usr.02s.changeaddress.fr.json
├── usr.03s.changebirthdate.fr.json
├── usr.04s.changebirthlocation.fr.json
├── usr.06e.changeemail.fr.json
├── usr.06s.changeemail.fr.json
├── usr.07s.confirmemail.fr.json
├── usr.08s.changename.fr.json
└── usr.09s.getuser.fr.json
```

**Listing 1:** An example *Reel* in the `user_reel` directory. The *Reel*
sequence indicates the steps taken to eventually return a representation of a
"User" object in the final state.

## Reel nomenclature
<a name="listing-2"></a>

```
┌─────────── Reel name              // usr
│   ┌─────── Sequence number        // 01
│   │ ┌───── Frame type             // se
│   │ │  ┌── Command name           // createuser
▼   ▼ ▼  ▼
usr.01se.createuser.fr.json
                    ▲
                    └─ Frame suffix // .fr.json
```

**Listing 2**: A breakdown of a *Frame*'s filename.

<a name="sequence-number"></a>
`Sequence number` - a number representing a particular step in an object's
state transition:

* The presence of a Frame does not necessarily allude to a state transition.
Most `Error Frame`s, for example, do not modify state.
* A sequence number must be a whole number unless there are multiple `Frame type`s
associated with a particular sequence number, then the sequence number must be
suffixed with a decimal value delimited by an underscore:

<a name="listing-3"></a>

  ```
  user_reel
  ├── usr.01e_1.createuser.fr.json // no first name
  ├── usr.01e_2.createuser.fr.json // no email
  └── usr.01s.createuser.fr.json   // user created successfully
  ```
  **Listing 3**: A *sequence number* with more than one `Error Frame`.

* `Whole sequence number` - the integer representation of a sequence number
indicating an object state.

<a name="frame-type"></a>
`Frame type` - defined by the return body of a *Frame*:

   1. `Error Frame` - *Frame* with a return body holding an error status code.
      * represented by the letter `e`.
      * precedes a `Success Frame` sharing the same whole sequence number.
   1. `Success Frame` - *Frame* with a return body holding a success status code:
      * represented by the letter `s`.
      * Typically indicates a state transition.
      * A sequence number should aim to be referenced by only one success Frame.
   1. `Post Success Error Frame` aka `P.S. Error Frame` -  an *Error Frame*
      that must be preceded by a
      *Success Frame* sharing the same *whole sequence number*:
      * represented by the letters `se`.

<a name="listing-4"></a>

   ```sh
     $ jq '.response.body' user_reel/01se.usr.createuser.fr.json
     "error: unable to create user, entry already exists"
   ```

  **Listing 4**: An example error message for a *post success Error Frame*.


<a name="reel-name"></a>
`Reel name` - indicates the *Reel* that a *Frame* belongs to:

* A Frame must belong to only one Reel.

<a name="reel-prefix"></a>
`Reel prefix` - the concatenation of a *Frame*'s *sequence number*,
*Frame type*, and *Reel name*:

<a name="listing-5"></a>

  ```
  usr.01se.createuser.fr.json
  └──────┤
         └─ Reel prefix // usr.01se
  ```

  **Listing 5**: Delineation of a *Frame* file's *Reel prefix*.

* Fully represents a Frame's relative placement in a *Reel* sequence.

<a name="method-name"></a>
`Method name` - represents the particular RPC command or REST endpoint
coinciding with a *Frame*'s [request method](frame.md#request).

<a name="type-suffix"></a>
`Type suffix` - a filename ending in `.fr.json` indicates said file is a
*filmReel* template *Frame*.


## filmReel concepts:

* [The Frame](frame.md)
* [The Reel](reel.md)
* [The Cut](cut.md)
