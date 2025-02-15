#### **Frame type** - defined by the return body of a Frame/filename

* #### **Error Frame** - Frame with a return body holding an error status code.
  - represented by the letter `e`.
  - precedes a **Success Frame** sharing the same whole sequence number.
* #### **Success Frame** - Frame with a return body holding a success status code:
  - represented by the letter `s`.
  - Typically indicates a state transition.
  - A sequence number should aim to be referenced by only one success Frame.
* #### **Post Success Error Frame** aka **P.S. Error Frame** -  an Error Frame that must be preceded by a Success Frame sharing the same whole sequence number:
  - represented by the letters `se`.

  ```
  user_reel
  ├── usr.01e_1.createuser.fr.json // no first name
  ├── usr.01e_2.createuser.fr.json // no email
  └── usr.01s.createuser.fr.json   // user created successfully
  ```
  **Ex**: A *sequence number* with more than one **Error Frame**.

* **Whole sequence number** - the float representation of a sequence number indicating an object state.
* ex: `01_e1` => 1.1,  `01_e2` => 1.2,
