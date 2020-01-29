use crate::error::FrError;
use crate::utils::ordered_string_map;

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;

/// Holds Cut Variables and their corresonding values stored in a series of key/value pairs.
///
/// [Cut Register](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#cut-register)
#[derive(Serialize, Clone, Deserialize, Default, Debug, PartialEq)]
pub struct Register<'a> {
    #[serde(serialize_with = "ordered_string_map", borrow, flatten)]
    vars: Variables<'a>,
}

const VAR_NAME_ERR: &'static str ="Only alphanumeric characters, dashes, and underscores are permitted in Cut Variable names => [A-za-z_]";
/// The Register's map of [Cut Variables](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#cut-variable)
type Variables<'a> = HashMap<&'a str, String>;

#[allow(dead_code)] // FIXME
impl<'a> Register<'a> {
    /// Creates a new Register object running post deserialization validations
    pub fn new(json_string: &str) -> Result<Register, FrError> {
        let reg: Register = serde_json::from_str(json_string)?;
        // reg.validate()?;
        Ok(reg)
    }

    /// Pretty json formatting for Frame serialization
    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialization error")
    }

    /// Inserts entry into the Register's Cut Variables/
    ///
    /// Returns an Err if the key value is does not consist solely of characters, dashes, and underscores.
    pub fn insert(&mut self, key: &'a str, val: String) -> Result<Option<String>, FrError> {
        lazy_static! {
            // Permit only alphachars dashes and underscores for variable names
            static ref KEY_CHECK: Regex = Regex::new(r"^[A-za-z_]+$").unwrap();
        }
        if !KEY_CHECK.is_match(key) {
            return Err(FrError::FrameParsef(VAR_NAME_ERR, key.to_string()));
        }
        Ok(self.vars.insert(key, val))
    }

    /// Gets a reference to the string slice value for the given var name.
    ///
    /// [Cut Variable](https://github.com/Bestowinc/filmReel/blob/supra_dump/cut.md#cut-variable)
    pub fn get_key_value(&self, key: &str) -> Option<(&&str, &String)> {
        self.vars.get_key_value(key)
    }

    /// An iterator visiting all Cut Variables in arbitrary order.
    pub fn iter(&self) -> std::collections::hash_map::Iter<&str, String> {
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
    pub fn read_match(&self, json_string: &String) -> Result<Vec<Match>, FrError> {
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

        'matches: for mat in VAR_MATCH.captures_iter(json_string) {
            // continue if the leading brace is escaped but strip "\\" from the match
            if let Some(esc_char) = mat.name("esc_char") {
                matches.push(Match::Escape(esc_char.range().clone()));
                continue 'matches;
            }

            let full_match = mat.get(0).expect("capture missing");

            // error if no trailing brace was found
            if mat.name("trailing_b").is_none() {
                return Err(FrError::FrameParsef(
                    "Missing trailing brace for Cut Variable",
                    String::from(full_match.as_str()),
                ));
            }

            let (name, value) =
                match self.get_key_value(mat.name("cut_var").expect("cut_var missing").as_str()) {
                    Some((&k, v)) => (k, v.to_owned()),
                    None => continue 'matches,
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
        if let Some(name) = mat.name() {
            if let None = self.get_key_value(name) {
                panic!("Key not present in Cut Register");
            }
        }
        mat.read_operation(json_string)
    }

    /// Takes a return payload and writes to the Cut Register
    pub fn write_match(var_name: &str, frame_str: &str, payload_str: &'a str) -> Option<&'a str> {
        let re = Regex::new(&format!(
            r"(?x)
                (?P<head_val>.*)?  # value preceding cut var
                (?P<esc_char>\\)?  # escape character
                (?P<cut_decl>\$\{{
                {}                 # format! curly braces
                \}})               # Cut Variable Declaration
                (?P<tail_val>.*)?  # value following cut var
                ",
            var_name
        ))
        .expect("write-match regex error");
        for mat in re.captures_iter(frame_str) {
            // continue if the leading brace is escaped but strip "\\" from the match
            if let Some(esc_char) = mat.name("esc_char") {
                continue;
            }
            let full_match = mat.get(0).expect("capture missing");
            let head_val = mat.name("head_val").expect("head_val missing").as_str();
            let tail_val = mat.name("tail_val").expect("tail_val missing").as_str();
            if !(payload_str.starts_with(head_val) && payload_str.ends_with(tail_val)) {
                panic!("head tail mismatch");
            }
            // TODO for now terminate early but capture second iteration
            let write_val = payload_str
                .trim_start_matches(head_val)
                .trim_end_matches(tail_val);

            return Some(write_val);
        }
        None
    }
}

/// Describes the types of matches during a read operation.
#[derive(Debug)]
pub enum Match<'a> {
    Escape(Range<usize>),
    Variable {
        name: &'a str,
        value: String,
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
            } => json_string.replace_range(r, &val),
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
        $(reg.insert($key, $val.to_string()).expect("RegisterInsertError");)*
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
        reg.insert("LAST_NAME", "Secundus".to_string()).unwrap();
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
            reg.insert("INVALID%STRING", r#"¯\_(ツ)_/¯"#.to_string())
                .unwrap_err(),
            FrError::FrameParsef(VAR_NAME_ERR, "INVALID%STRING".to_string())
        );

        reg.insert("FIRST_NAME", "Pietre".to_string()).unwrap();
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
            kv_vec.push((k, v));
        }
        assert_eq!(
            vec![(&"FIRST_NAME", "Primus"), (&"RESPONSE", "ALRIGHT")].sort(),
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
        // case( TODO move this to read_operation test
        //     "My name is ${MIDDLE_NAME} ${LAST_NAME}",
        //     FrError::ReadInstructionf("Key is not present in the Cut Register", "MIDDLE_NAME".to_string())
        // ),
        case(
            "My name is ${FIRST_NAME} ${LAST_NAME",
            FrError::FrameParsef("Missing trailing brace for Cut Variable", "${LAST_NAME".to_string())
        )
    )]
    fn test_read_op_err(input: &str, expected: FrError) {
        let reg = register!({
            "FIRST_NAME"=> "Slim",
            "LAST_NAME"=> "Shady"
        });
        let mut str_with_var = input.to_string();
        assert_eq!(expected, reg.read_match(&mut str_with_var).unwrap_err())
    }

    #[rstest(
        var,
        frame,
        payload,
        expected,
        case("NAME", "My name is ${NAME}.", "My name is Slim Shady.", "Slim Shady"),
        case("SINGLE", "${SINGLE}", "my big hit", "my big hit")
    )]
    fn test_write_op(var: &str, frame: &str, payload: &str, expected: &str) {
        let reg = register!({});
        let actual = Register::write_match(var, frame, payload).unwrap();
        assert_eq!(expected, actual);
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
        reg.insert("LAST_NAME", "Secundus".to_string()).unwrap();
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
