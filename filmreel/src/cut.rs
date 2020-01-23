use crate::utils::ordered_map;

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{Index, Range};

/// Holds cut variables and their corresonding values stored in a series of key/value pairs.
///
/// [Cut Register](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#cut-register)
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct Register<'a> {
    #[serde(serialize_with = "ordered_map", borrow, flatten)]
    vars: Vars<'a>,
}

type Vars<'a> = HashMap<&'a str, &'a str>;

impl<'a> Register<'a> {
    /// Creates a new Register struct with default values.
    pub fn new() -> Self {
        Default::default()
    }

    /// Inserts entry into the Register's cut variables/
    ///
    /// Returns an Err if the key value is does not consist solely of characters, dashes, and underscores.
    pub fn insert(&mut self, key: &'a str, val: &'a str) -> Result<Option<&'a str>, &str> {
        lazy_static! {
            // Permit only alphachars dashes and underscores for variable names
            static ref KEY_CHECK: Regex = Regex::new(r"^[A-za-z_]+$").unwrap();
        }
        if !KEY_CHECK.is_match(key) {
            return Err(
                "Only alphanumeric characters, dashes, and underscores are permitted in cut variable names: [A-za-z_]");
        }
        Ok(self.vars.insert(key, val))
    }

    /// Gets a reference to the string slice value for the given var name
    pub fn get(&self, key: &str) -> Option<&&str> {
        self.vars.get(key)
    }

    /// An iterator visiting all cut variables in arbitrary order.
    pub fn iter(&self) -> std::collections::hash_map::Iter<&str, &str> {
        self.vars.iter()
    }

    /// Returns a boolean idicating whether the register contais a Cut Variable
    pub fn contains_key(&self, key: &str) -> bool {
        self.vars.contains_key(key)
    }

    /// Replaces a cut variable reference with the corresonding value if a match is found
    /// with a valid cut delimiter.
    pub fn from(&self, json_string: &mut String) -> Result<(), &str> {
        lazy_static! {
            static ref VAR_MATCH: Regex = Regex::new(
                r"(?x)
                (?P<esc_char>\\)?   # escape character
                (?P<leading_b>\$\{) # leading brace
                (?P<var>[A-za-z_]+) # cut variable
                (?P<trailing_b>})?  # trailing brace
                "
            )
            .unwrap();
        }

        // store matches to replace and the range index of the match
        type Replacement<'a> = (&'a str, Range<usize>);
        let mut valid_matches: Vec<Replacement> = Vec::new();

        for mat in VAR_MATCH.captures_iter(json_string) {
            dbg!(&mat);
            // continue if the leading brace is escaped but strip "\\" from the match
            if mat.name("esc_char").is_some() {
                valid_matches.push(("", mat.name("esc_char").expect("Capture missing").range()));
                continue;
            }

            let var: &str = mat
                .name("var")
                .expect("MatchError: Cut Variable missing")
                .as_str();

            let val: &str = match self.get(var) {
                Some(&i) => i,
                None => {
                    dbg!(self.get(var));
                    return Err("ReadInstructionError: Key is not present in the Cut Register");
                }
            };

            // error if no trailing brace was found
            if mat.name("trailing_b").is_none() {
                return Err("ReadInstructionError: Missing trailing brace for Cut Variable");
            }

            // push valid match onto Replacement vec
            valid_matches.push((val, mat.get(0).expect("Match missing").range()));
        }
        // sort matches by start of each match range
        valid_matches.sort_by_key(|k| k.1.start);

        // reverse valid matches so early match index range does not change when matches found
        // later in the string are replaced
        for mat in valid_matches.iter().rev() {
            json_string.replace_range(mat.1.clone(), mat.0);
        }

        Ok(())
    }
}

#[macro_export]
macro_rules! register {
    ({$( $key: expr => $val: expr ),*}) => {{
        use crate::cut::Register;

        let mut reg = Register::new();
        $(reg.insert($key, $val).expect("RegisterInsertError");)*
        reg
    }}
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[test]
    fn test_insert() {
        let mut reg = register!({
            "FIRST_NAME"=> "Primus",
            "RESPONSE"=> "ALRIGHT"
        });
        reg.insert("LAST_NAME", "Secundus").unwrap();
        assert_eq!(
            register!({
                "FIRST_NAME"=> "Primus",
                "RESPONSE"=> "ALRIGHT",
                "LAST_NAME"=> "Secundus"
            }),
            reg
        );
    }

    #[test]
    fn test_update() {
        let mut reg = register!({
            "FIRST_NAME"=> "Primus",
            "RESPONSE"=> "ALRIGHT"
        });

        assert_eq!(reg.insert("INVALID%STRING", r#"¯\_(ツ)_/¯"#).unwrap_err(),
                "Only alphanumeric characters, dashes, and underscores are permitted in cut variable names: [A-za-z_]");

        reg.insert("FIRST_NAME", "Pietre").unwrap();
        assert_eq!(
            register!({
                "FIRST_NAME"=> "Pietre",
                "RESPONSE"=> "ALRIGHT"
            }),
            reg
        );
        // assert_eq!(
        //     Err("YARR"),
        // );
    }

    #[test]
    fn test_iter() {
        let reg = register!({
            "FIRST_NAME"=> "Primus",
            "RESPONSE"=> "ALRIGHT"
        });
        let mut kv_vec = vec![];

        for (k, v) in reg.iter() {
            kv_vec.push(k);
            kv_vec.push(v);
        }
        assert_eq!(
            vec![&"FIRST_NAME", &"Primus", &"RESPONSE", &"ALRIGHT"].sort(),
            kv_vec.sort()
        );
    }

    const TRAGIC_STORY: &str = "I thought not. It's not a story the Jedi would tell you.
        It's a Sith legend. Darth Plagueis was a Dark Lord of the Sith, \
        so powerful and so wise he could use the Force to influence the midichlorians to create life... \
        He had such a knowledge of the dark side that he could even keep the ones he cared about from dying. \
        The dark side of the Force is a pathway to many abilities some consider to be unnatural. \
        He became so powerful... the only thing he was afraid of was losing his power, which eventually, of course, \
        he did. Unfortunately, he taught his apprentice everything he knew, \
        then his apprentice killed him in his sleep. \
        It's ironic he could save others from death, but not himself.";

    #[rstest(
        input,
        expected,
        case("My name is ${FIRST_NAME} ${LAST_NAME}", "My name is Slim Shady"),
        case(
            "My name is \\${FIRST_NAME} ${LAST_NAME}",
            "My name is ${FIRST_NAME} Shady"
        ),
        case(
            "My name is ${FIRST_NAME} \\${LAST_NAME}",
            "My name is Slim ${LAST_NAME}"
        ),
        case(
            "My name is \\${FIRST_NAME} \\${LAST_NAME}",
            "My name is ${FIRST_NAME} ${LAST_NAME}"
        ),
        case("Did you ever hear the tragedy of Darth Plagueis the Wise? ${INANE_RANT}", 
            &["Did you ever hear the tragedy of Darth Plagueis the Wise? ", TRAGIC_STORY].concat())
    )]
    fn test_read_op(input: &str, expected: &str) {
        let reg = register!({
            "FIRST_NAME"=> "Slim",
            "LAST_NAME"=> "Shady",
            "INANE_RANT"=> TRAGIC_STORY
        });
        let mut str_with_var = input.to_string();
        reg.from(&mut str_with_var);
        assert_eq!(expected.to_string(), str_with_var)
    }

    #[rstest(
        input,
        expected,
        case(
            "My name is ${MIDDLE_NAME} ${LAST_NAME}",
            "ReadInstructionError: Key is not present in the Cut Register"
        ),
        case(
            "My name is ${FIRST_NAME} ${LAST_NAME",
            "ReadInstructionError: Missing trailing brace for Cut Variable"
        )
    )]
    fn test_read_op_err(input: &str, expected: &str) {
        let reg = register!({
            "FIRST_NAME"=> "Slim",
            "LAST_NAME"=> "Shady"
        });
        let mut str_with_var = input.to_string();
        assert_eq!(reg.from(&mut str_with_var).unwrap_err(), expected)
    }
}

#[cfg(test)]
mod serde_tests {
    use super::*;
    use crate::test_ser_de;

    const REGISTER_JSON: &str = r#"
        {
          "FIRST_NAME": "Primus",
          "RESPONSE": "ALRIGHT"
        }
    "#;
    test_ser_de!(
        register_ser,
        register_de,
        register!({
            "FIRST_NAME"=> "Primus",
            "RESPONSE"=> "ALRIGHT"
        }),
        REGISTER_JSON
    );

    #[test]
    fn test_insert() {
        let mut reg = register!({
            "FIRST_NAME"=> "Primus",
            "RESPONSE"=> "ALRIGHT"
        });
        reg.insert("LAST_NAME", "Secundus").unwrap();
        assert_eq!(
            register!({
                "FIRST_NAME"=> "Primus",
                "RESPONSE"=> "ALRIGHT",
                "LAST_NAME"=> "Secundus"
            }),
            reg
        );
    }
}
