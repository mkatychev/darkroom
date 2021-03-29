use crate::{error::FrError, utils::ordered_val_map};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, ops::Range};

/// Holds Cut Variables and their corresponding values stored in a series of
/// key/value pairs.
///
/// [Cut Register](https://github.com/Bestowinc/filmReel/blob/master/cut.md#cut-register)
#[derive(Serialize, Clone, Deserialize, Default, Debug, PartialEq)]
pub struct Register {
    #[serde(serialize_with = "ordered_val_map", flatten)]
    vars: Variables,
}

const VAR_NAME_ERR: &str = "Only alphanumeric characters, dashes, and underscores are permitted \
                            in Cut Variable names => [A-Za-z_]";

/// The Register's map of [Cut Variables]
/// (https://github.com/Bestowinc/filmReel/blob/master/cut.md#cut-variable)
type Variables = HashMap<String, Value>;

impl Register {
    /// Creates a Register from a string ref
    pub fn from<T: AsRef<str>>(json_string: T) -> Result<Register, FrError> {
        let reg: Register = serde_json::from_str(json_string.as_ref())?;
        // reg.validate()?;
        Ok(reg)
    }

    // Alias for Register::default
    pub fn new() -> Self {
        Register::default()
    }

    /// Pretty json formatting for Register serialization
    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialization error")
    }

    /// Inserts entry into the Register's Cut Variables
    fn insert<T>(&mut self, key: T, val: Value) -> Option<Value>
    where
        T: ToString,
    {
        self.vars.insert(key.to_string(), val)
    }

    /// Removes a single key value
    fn remove(&mut self, key: &str) -> Option<Value> {
        self.vars.remove(key)
    }

    /// Gets a reference to the string slice value for the given var name.
    ///
    /// [Cut Variable](https://github.com/Bestowinc/filmReel/blob/master/cut.md#cut-variable)
    pub fn get_key_value<K: AsRef<str>>(&self, key: K) -> Option<(&String, &Value)> {
        self.vars.get_key_value(key.as_ref())
    }

    /// Gets a reference to the string slice value for the given var name.
    ///
    /// [Cut Variable](https://github.com/Bestowinc/filmReel/blob/master/cut.md#cut-variable)
    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<&Value> {
        self.vars.get(key.as_ref())
    }

    /// An iterator visiting all Cut Variables in arbitrary order.
    pub fn iter(&self) -> std::collections::hash_map::Iter<String, Value> {
        self.vars.iter()
    }

    /// Returns a boolean indicating whether Register.vars contains a given key.
    ///
    /// [Cut Variable](https://github.com/Bestowinc/filmReel/blob/master/cut.md#cut-variable)
    pub fn contains_key(&self, key: &str) -> bool {
        self.vars.contains_key(key)
    }

    /// Merges a foreign Cut register into the caller, overriding any values in self with other
    pub fn destructive_merge<I>(&mut self, others: I)
    where
        I: IntoIterator<Item = Register>,
    {
        for other in others.into_iter() {
            for (k, v) in other.iter() {
                self.insert(k.to_string(), v.clone());
            }
        }
    }

    /// Returns a vector of Match enums enums found in the string provided for
    /// use in cut operations.
    ///
    /// [Read Operation](https://github.com/Bestowinc/filmReel/blob/master/cut.md#read-operation)
    pub fn read_match(&self, json_string: &str) -> Result<Vec<Match>, FrError> {
        lazy_static! {
            static ref VAR_MATCH: Regex = Regex::new(
                r"(?x)
                (?P<esc_char>\\)?   # escape character
                (?P<leading_b>\$\{) # leading brace
                (?P<cut_var>[A-za-z_0-9]+) # Cut Variable
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
                return Err(FrError::FrameParsef(
                    "Missing trailing brace for Cut Variable",
                    full_match.as_str().to_string(),
                ));
            }

            match self.get_key_value(mat.name("cut_var").expect("cut_var error").as_str()) {
                Some((k, v)) => {
                    // push valid match onto Match vec
                    matches.push(Match::Variable {
                        name:  k,
                        value: v.clone(),
                        range: full_match.range(),
                    });
                }
                None => continue,
            };
        }

        // sort matches by start of each match range and reverse valid matches
        // so an early match index range does not shift when matches found
        // later in the string are replaced during a .iter() loop
        matches.sort_by_key(|k| std::cmp::Reverse(k.range().start));

        Ok(matches)
    }

    /// Replaces a byte range in a given string with the range given in the
    /// ::Match provided.
    ///
    /// [Read Operation](https://github.com/Bestowinc/filmReel/blob/master/cut.md#read-operation)
    pub fn read_operation(
        &self,
        mat: Match,
        value: &mut Value,
        hide_vars: bool,
    ) -> Result<(), FrError> {
        if let Some(name) = mat.name() {
            if self.get_key_value(name).is_none() {
                return Err(FrError::ReadInstructionf(
                    "Key not present in Cut Register",
                    name.to_string(),
                ));
            }
            if hide_vars && name.starts_with('_') {
                let expected = format!("{}{}{}", "${", name, "}");
                if let Value::String(val) = value {
                    if val.contains(&expected) {
                        Match::Hide.read_operation(value)?;
                        return Ok(());
                    }
                }
            }
        }

        mat.read_operation(value)?;
        Ok(())
    }

    // ensures string slice past is a singular declaration of a `"${VARIABLE}"`
    pub fn expect_standalone_var(var_name: &str, frame_str: &str) -> Result<(), FrError> {
        let expected = format!("{}{}{}", "${", var_name, "}");
        if expected != frame_str {
            return Err(FrError::FrameParsef(
                "Singe variable mismatch -",
                format!("Expected:{}, Got:{}", expected, frame_str),
            ));
        }
        Ok(())
    }

    /// Takes a frame string value and compares it against a payload string value
    /// returning any declared cut variables found
    pub fn write_match(
        var_name: &str,
        frame_str: &str,
        payload_str: &str,
    ) -> Result<Option<String>, FrError> {
        let re = Regex::new(&format!(
            r"(?x)
                (?P<head_val>.*)   # value preceding cut var
                (?P<esc_char>\\)?  # escape character
                (?P<cut_decl>\$\{{
                {}
                \}})               # Cut Variable Declaration
                (?P<tail_val>.*)   # value following cut var
                ",
            var_name
        ))
        .expect("write-match regex error");

        let mut matches: Vec<&str> = Vec::new();
        for mat in re.captures_iter(frame_str) {
            // continue if the leading brace is escaped but strip "\\" from the match
            if mat.name("esc_char").is_some() {
                continue;
            }

            let head_val = mat.name("head_val").expect("head_val error").as_str();
            let tail_val = mat.name("tail_val").expect("tail_val error").as_str();
            if !(payload_str.starts_with(head_val) && payload_str.ends_with(tail_val)) {
                return Err(FrError::WriteInstruction(
                    "Frame String templating mismatch",
                ));
            }

            matches.push(
                payload_str
                    .trim_start_matches(head_val)
                    .trim_end_matches(tail_val),
            );
        }

        // `_ =>` is not possible for now, but guard with panic
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(
                matches.pop().expect("missing match value").to_string(),
            )),
            _ => unreachable!(
                "Multiple variable matches in string not permitted for write instruction"
            ),
        }
    }

    /// Inserts a Value entry into the Register's Cut Variables
    ///
    /// Returns an Err if the key value is does not consist solely of characters, dashes, and underscores.
    pub fn write_operation(&mut self, key: &str, val: Value) -> Result<Option<Value>, FrError> {
        lazy_static! {
            // Permit only alphachars dashes and underscores for variable names
            static ref KEY_CHECK: Regex = Regex::new(r"^[A-Za-z_]+$").unwrap();
        }
        if !KEY_CHECK.is_match(key) {
            return Err(FrError::FrameParsef(VAR_NAME_ERR, key.to_string()));
        }
        Ok(self.insert(key, val))
    }

    /// Flushes lowercase/ignored variable patters
    pub fn flush_ignored(&mut self) {
        lazy_static! {
        // if key value consists of only lowercase letters and underscores
            static ref KEY_IGNORE: Regex = Regex::new(r"^[a-z_]+$").unwrap();
        }
        let mut remove: Vec<String> = vec![];
        for (k, _) in self.vars.iter() {
            if KEY_IGNORE.is_match(k) {
                remove.push(k.to_owned());
            }
        }
        for k in remove.iter() {
            self.remove(&k);
        }
    }
}

/// Describes the types of matches during a read operation.
#[derive(Debug)]
pub enum Match<'a> {
    Escape(Range<usize>),
    Variable {
        name:  &'a str,
        value: Value,
        range: Range<usize>,
    },
    Hide,
}

impl<'a> Match<'a> {
    /// the range over the starting and ending byte offsets for the corresponding
    /// Replacement.
    fn range(&self) -> Range<usize> {
        match self {
            Match::Escape(range) => range.clone(),
            Match::Variable { range: r, .. } => r.clone(),
            Match::Hide => panic!("range called on Match::Hide"),
        }
    }

    // return name string slice of Match enum
    pub fn name(&self) -> Option<&'a str> {
        match self {
            Match::Escape(_) => None,
            Match::Hide => None,
            Match::Variable { name: n, .. } => Some(*n),
        }
    }

    // replaces json_value with Match.value
    fn read_operation(self, json_value: &mut Value) -> Result<(), FrError> {
        // TODO refactor cthulu looking match arms
        match self {
            Match::Escape(range) => match json_value {
                Value::String(json_str) => {
                    json_str.replace_range(range, "");
                    Ok(())
                }
                _ => Err(FrError::ReadInstruction(
                    "Match::Escape.value is a non string Value",
                )),
            },
            Match::Variable {
                value: match_val,
                range: r,
                ..
            } => match match_val {
                // if the match value is a string
                Value::String(match_str) => match json_value {
                    // and the json value is as well, replace the range within
                    Value::String(str_val) => {
                        str_val.replace_range(r, &match_str);
                        Ok(())
                    }
                    _ => Err(FrError::ReadInstruction(
                        "Match::Variable given a non string value to replace",
                    )),
                },
                _ => {
                    *json_value = match_val.clone();
                    Ok(())
                }
            },
            Match::Hide => match json_value {
                Value::String(json_str) => {
                    *json_str = "${_HIDDEN}".to_string();
                    Ok(())
                }
                _ => Err(FrError::ReadInstruction(
                    "Match::Hide.value is a non string Value",
                )),
            },
        }
    }
}

/// Constructs a [Cut Register](https://github.com/Bestowinc/filmReel/blob/master/cut.md#cut-register)
/// from the provided series of key value pairs.
#[macro_export]
macro_rules! register {
    ({$( $key: expr => $val: expr ),*}) => {{
        use $crate::cut::Register;

        let mut reg = Register::default();
        $(reg.write_operation($key, serde_json::value::to_value($val).expect("to_value error")).expect("RegisterInsertError");)*
        reg
    }}
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde_json::json;

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
            kv_vec.sort_by(|a, b| a.0.cmp(b.0))
        );
    }

    fn destructive_merge_case(case: u32) -> (Vec<Register>, Register) {
        match case {
            1 => (
                vec![register!({ "NEW_KEY"=> "NEW_VALUE" })],
                register!({"KEY"=>"VALUE","NEW_KEY"=>"NEW_VALUE"}),
            ),
            2 => (
                vec![register!({ "KEY"=> "NEW_VALUE", "NEW_KEY"=> "NEW_VALUE" })],
                register!({"KEY"=>"NEW_VALUE","NEW_KEY"=>"NEW_VALUE"}),
            ),
            3 => (
                vec![
                    register!({ "KEY"=> "NEW_VALUE", "NEW_KEY"=> "NEW_VALUE" }),
                    register!({ "NEW_KEY"=> json!({"new": "object"})}),
                    register!({ "ignored_key"=> "ignored_value"}),
                ],
                register!({"KEY"=>"NEW_VALUE","NEW_KEY"=> json!({"new": "object"})}),
            ),
            _ => (vec![], Register::default()),
        }
    }

    #[rstest(
        input_expected,
        case(destructive_merge_case(1)),
        case(destructive_merge_case(2)),
        case(destructive_merge_case(3))
    )]
    fn test_destructive_merge(input_expected: (Vec<Register>, Register)) {
        let mut reg = register!({ "KEY"=> "VALUE" });
        reg.destructive_merge(input_expected.0);
        reg.flush_ignored();
        assert_eq!(reg, input_expected.1);
    }

    const TRAGIC_STORY: &str = "I thought not. It's not a story the Jedi would tell you.
        It's a Sith legend. Darth Plagueis was a Dark Lord of the Sith, so powerful and so wise he \
         could use the Force to influence the midichlorians to create life... He had such a \
         knowledge of the dark side that he could even keep the ones he cared about from dying. \
         The dark side of the Force is a pathway to many abilities some consider to be unnatural. \
         He became so powerful... the only thing he was afraid of was losing his power, which \
         eventually, of course, he did. Unfortunately, he taught his apprentice everything he \
         knew, then his apprentice killed him in his sleep. It's ironic he could save others from \
         death, but not himself.";

    fn case_read_op(case: u32) -> (Value, Value) {
        return match case {
            1 => (
                json!("My name is ${FIRST_NAME} ${LAST_NAME}"),
                json!("My name is Slim Shady"),
            ),
            2 => (
                json!("My name is ${FIRST_NAME} \\${LAST_NAME}"),
                json!("My name is Slim ${LAST_NAME}"),
            ),
            3 => (
                json!("My name is \\${FIRST_NAME} \\${LAST_NAME}"),
                json!("My name is ${FIRST_NAME} ${LAST_NAME}"),
            ),
            4 => (
                json!("Did you ever hear the tragedy of Darth Plagueis the Wise? ${INANE_RANT}"),
                json!(&[
                    "Did you ever hear the tragedy of Darth Plagueis the Wise? ",
                    TRAGIC_STORY
                ]
                .concat()),
            ),
            5 => (json!("${OBJECT}"), json!({"key": "value"})),
            _ => (json!({}), json!({})),
        };
    }
    #[rstest(
        in_out,
        case(case_read_op(1)),
        case(case_read_op(2)),
        case(case_read_op(3)),
        case(case_read_op(4)),
        case(case_read_op(5))
    )]
    fn test_read_op(in_out: (Value, Value)) {
        let (mut input, expected) = in_out;
        let reg = register!({
            "FIRST_NAME"=>"Slim",
            "LAST_NAME"=> "Shady",
            "INANE_RANT"=> TRAGIC_STORY,
            "OBJECT"=> json!({"key": "value"})
        });
        let matches: Vec<Match> = reg
            .read_match(&input.as_str().unwrap())
            .expect("match error");
        for mat in matches.into_iter() {
            reg.read_operation(mat, &mut input, false).unwrap();
        }
        assert_eq!(expected, input)
    }

    #[rstest(
        input,
        expected,
        case(
            "My name is ${FIRST_NAME} ${LAST_NAME ${LAST_NAME}",
            FrError::FrameParsef("Missing trailing brace for Cut Variable", "${LAST_NAME".to_string())
        ),
        case(
            "My name is ${FIRST_NAME} ${LAST_NAME",
            FrError::FrameParsef("Missing trailing brace for Cut Variable", "${LAST_NAME".to_string())
        )
    )]
    fn test_read_match_err(input: &str, expected: FrError) {
        let reg = register!({
            "FIRST_NAME"=> "Slim",
            "LAST_NAME"=> "Shady"
        });
        assert_eq!(
            expected,
            reg.read_match(&mut input.to_string()).unwrap_err()
        )
    }

    #[rstest(
        var,
        frame,
        payload,
        expected,
        case("NAME", "My name is ${NAME}.", "My name is Slim Shady.", "Slim Shady"),
        case("SINGLE", "${SINGLE}", "my big hit", "my big hit"),
        case(
            "SINGLE",
            "${SINGLE}|${SINGLE}",
            "1|2",
            "WriteInstructionError: Frame String templating mismatch"
        ),
        case(
            "SINGLE",
            "${SINGLE}|",
            "|2",
            "WriteInstructionError: Frame String templating mismatch"
        )
    )]
    fn test_write_match(var: &str, frame: &str, payload: &str, expected: &str) {
        match Register::write_match(var, frame, &payload.to_string()) {
            Ok(mat) => assert_eq!(expected, mat.unwrap()),
            Err(err) => assert_eq!(expected, err.to_string()),
        }
    }

    #[test]
    fn test_write_op() {
        let mut reg = register!({
            "FIRST_NAME"=> "Primus",
            "RESPONSE"=> "ALRIGHT"
        });
        reg.write_operation(
            "LAST_NAME",
            serde_json::to_value("Secundus").expect("value parse error"),
        )
        .unwrap();
        assert_eq!(
            reg.get("LAST_NAME"),
            Some(&Value::String("Secundus".to_string()))
        );
    }

    #[test]
    fn test_write_op_err() {
        let mut reg = register!({
            "FIRST_NAME"=> "Primus",
            "RESPONSE"=> "ALRIGHT"
        });

        assert_eq!(
            reg.write_operation("INVALID%STRING", Value::String(r#"¯\_(ツ)_/¯"#.to_string()))
                .unwrap_err(),
            FrError::FrameParsef(VAR_NAME_ERR, "INVALID%STRING".to_string())
        );

        reg.write_operation("FIRST_NAME", Value::String("Pietre".to_string()))
            .unwrap();
        assert_eq!(
            register!({
                "FIRST_NAME"=> "Pietre",
                "RESPONSE"=> "ALRIGHT"
            }),
            reg
        );
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
        register,
        register!({
            "FIRST_NAME"=> "Primus",
            "RESPONSE"=> "ALRIGHT"
        }),
        REGISTER_JSON
    );
}
