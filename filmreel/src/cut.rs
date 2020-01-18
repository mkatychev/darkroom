use std::collections::HashMap;

struct Register {
    vars: HashMap<&'a str, &'a str>,
}

type Variable = &'a str;
