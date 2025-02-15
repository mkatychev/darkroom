# Reel 

* the sum of all *Frame*s in a directory that share the same *Reel name*.

```sh
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

# Reel membership

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

- ##### **Reel name** - describes the functionality of the entire flow (subjective)
- ##### **Sequence number** - a number representing a particular step in an object's state transition:
- ##### **Frame type** - defined by the return body of a *Frame*:
- ##### **Command name** - describes the functionality of the *Frame* JSON file
