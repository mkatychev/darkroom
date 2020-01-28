use crate::error::Error;
use crate::utils::ordered_map;

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;

/// Holds Cut Variables and their corresonding values stored in a series of key/value pairs.
///
/// [Cut Register](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#cut-register)
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct Register<'a> {
    #[serde(serialize_with = "ordered_map", borrow, flatten)]
    vars: Variables<'a>,
}

const VAR_NAME_ERR: &'static str ="Only alphanumeric characters, dashes, and underscores are permitted in Cut Variable names => [A-za-z_]";
/// The Register's map of [Cut Variables](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#cut-variable)
type Variables<'a> = HashMap<&'a str, &'a str>;

#[allow(dead_code)] // FIXME
impl<'a> Register<'a> {
    /// Inserts entry into the Register's Cut Variables/
    ///
    /// Returns an Err if the key value is does not consist solely of characters, dashes, and underscores.
    pub fn insert(&mut self, key: &'a str, val: &'a str) -> Result<Option<&'a str>, Error> {
        lazy_static! {
            // Permit only alphachars dashes and underscores for variable names
            static ref KEY_CHECK: Regex = Regex::new(r"^[A-za-z_]+$").unwrap();
        }
        if !KEY_CHECK.is_match(key) {
            return Err(Error::FrameParsef(VAR_NAME_ERR, key.to_string()));
        }
        Ok(self.vars.insert(key, val))
    }

    /// Gets a reference to the string slice value for the given var name.
    ///
    /// [Cut Variable](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#cut-variable)
    pub fn get_key_value(&self, key: &str) -> Option<(&&str, &&str)> {
        self.vars.get_key_value(key)
    }

    /// An iterator visiting all Cut Variables in arbitrary order.
    pub fn iter(&self) -> std::collections::hash_map::Iter<&str, &str> {
        self.vars.iter()
    }

    /// Returns a boolean indicating whether Register.vars contains a given key.
    ///
    /// [Cut Variable](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#cut-variable)
    pub fn contains_key(&self, key: &str) -> bool {
        self.vars.contains_key(key)
    }

    /// Returns a vector of Match enums enums found in the string provided for use in cut
    /// operations.
    ///
    /// [Read Operation](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#read-operation)
    pub fn read_match(&self, json_string: &String) -> Result<Vec<Match>, Error> {
        lazy_static! {
            static ref VAR_MATCH: Regex = Regex::new(
                r"(?x)
                (?P<esc_char>\\)?   # escape character
                (?P<leading_b>\$\{) # leading brace
                (?P<cut_var>[A-za-z_]+) # Cut Variable
                (?P<trailing_b>})?  # trailing brace
                "
            )
            .unwrap();
        }

        let mut matches: Vec<Match> = Vec::new();

        for mat in VAR_MATCH.captures_iter(json_string) {
            // continue if the leading brace is escaped but strip "\\" from the match
            if let Some(esc_char) = mat.name("esc_char") {
                matches.push(Match::Escape(esc_char.range().clone()));
                continue;
            }

            let full_match = mat.get(0).expect("capture missing");

            // error if no trailing brace was found
            if mat.name("trailing_b").is_none() {
                return Err(Error::FrameParsef(
                    "Missing trailing brace for Cut Variable",
                    String::from(full_match.as_str()),
                ));
            }

            let (name, value) =
                match self.get_key_value(mat.name("cut_var").expect("cut_var missing").as_str()) {
                    Some((&k, &v)) => (k, v),
                    None => {
                        return Err(Error::ReadInstructionf(
                            "Key is not present in the Cut Register",
                            String::from(mat.name("cut_var").expect("cut_var missing").as_str()),
                        ));
                    }
                };

            // push valid match onto Match vec
            matches.push(Match::Variable {
                name,
                value,
                range: full_match.range(),
            });
        }

        // sort matches by start of each match range and reverse valid matches
        // so an early match index range does not shift when matches found
        // later in the string are replaced during a .iter() loop
        matches.sort_by_key(|k| std::cmp::Reverse(k.range().start));

        Ok(matches)
    }

    /// Replaces a byte range in a given string with the range given in the ::Match provided.
    ///
    /// [Read Operation](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#read-operation)
    pub fn read_operation(&self, mat: Match, json_string: &mut String) {
        mat.read_operation(json_string)
    }
}

/// Describes the types of matches during a read operation.
#[derive(Debug)]
pub enum Match<'a> {
    Escape(Range<usize>),
    Variable {
        name: &'a str,
        value: &'a str,
        range: Range<usize>,
    },
}

#[allow(dead_code)] // FIXME
impl<'a> Match<'a> {
    /// the range over the starting and ending byte offsets for the corresonding Replacement.
    fn range(&self) -> Range<usize> {
        match self {
            Match::Escape(range) => range.clone(),
            Match::Variable {
                name: _,
                value: _,
                range: r,
            } => r.clone(),
        }
    }

    pub fn name(&self) -> Option<&'a str> {
        match self {
            Match::Escape(_) => None,
            Match::Variable {
                name: n,
                value: _,
                range: _,
            } => Some(*n),
        }
    }

    fn read_operation(self, json_string: &mut String) {
        match self {
            Match::Escape(range) => json_string.replace_range(range, ""),
            Match::Variable {
                name: _,
                value: val,
                range: r,
            } => json_string.replace_range(r, val),
        }
    }
}

/// Constructs a [Cut Register](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#cut-register)
/// from the provided series of key value pairs.
#[macro_export]
macro_rules! register {
    ({$( $key: expr => $val: expr ),*}) => {{
        use crate::cut::Register;

        let mut reg = Register::default();
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

        assert_eq!(
            reg.insert("INVALID%STRING", r#"¯\_(ツ)_/¯"#).unwrap_err(),
            Error::FrameParsef(VAR_NAME_ERR, "INVALID%STRING".to_string())
        );

        reg.insert("FIRST_NAME", "Pietre").unwrap();
        assert_eq!(
            register!({
                "FIRST_NAME"=> "Pietre",
                "RESPONSE"=> "ALRIGHT"
            }),
            reg
        );
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
        let matches: Vec<Match> = reg.read_match(&str_with_var).unwrap();
        for mat in matches.into_iter() {
            reg.read_operation(mat, &mut str_with_var);
        }
        assert_eq!(expected.to_string(), str_with_var)
    }

    #[rstest(
        input,
        expected,
        case(
            "My name is ${MIDDLE_NAME} ${LAST_NAME}",
            Error::ReadInstructionf("Key is not present in the Cut Register", "MIDDLE_NAME".to_string())
        ),
        case(
            "My name is ${FIRST_NAME} ${LAST_NAME",
            Error::FrameParsef("Missing trailing brace for Cut Variable", "${LAST_NAME".to_string())
        )
    )]
    fn test_read_op_err(input: &str, expected: Error) {
        let reg = register!({
            "FIRST_NAME"=> "Slim",
            "LAST_NAME"=> "Shady"
        });
        let mut str_with_var = input.to_string();
        assert_eq!(expected, reg.read_match(&mut str_with_var).unwrap_err())
    }
}

#[cfg(test)]
mod serde_tests {
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
