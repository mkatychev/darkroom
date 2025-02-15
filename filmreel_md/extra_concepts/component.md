# Component Reels

* reels that  get initialized before the specified reel.
* allow different flows to reuse the same preludes.
* component reels will only run add their **success frames** to the prelude.

  ```
  create
  ├── create.01s.register.fr.json // send email/password
  └── create.02s.create.fr.json   // create user entity

  validate
  ├── validate.01.email.fr.json   // email validated
  ├── validate.01e.phone.fr.json  // wrong response
  └── validate.01s.phone.fr.json  // phone validated
  ```
  **Ex**: By specifying the `create` reel as a component of the `validate` reel,
  a user will be first created before email and phone frames are run.
