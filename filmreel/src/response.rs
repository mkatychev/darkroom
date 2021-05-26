use crate::{
    cut::Register,
    error::FrError,
    frame::*,
    utils::{new_mut_selector, select_value, MutSelector},
};
use serde::{Deserialize, Serialize};
use serde_hashkey::{
    to_key_with_ordered_float as to_key, Error as HashError, Key, OrderedFloatPolicy as Hash,
};
use serde_json::{json, to_value, Map, Value};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
};

const INVALID_INSTRUCTION_TYPE_ERR: &str =
    "Frame write instruction did not correspond to a string object";

const MISSING_SELECTION_ERR: &str = "selection missing from Frame body";

/// Encapsulates the expected response payload.
///
/// [Request Object](https://github.com/Bestowinc/filmReel/blob/master/frame.md#request)
#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Response<'a> {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body:       Option<Value>,
    //
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub etc:        Option<Value>, // https://github.com/serde-rs/serde/issues/1626
    #[serde(skip_serializing)]
    pub validation: Option<Validation<'a>>,
    pub status:     u32,
}

impl<'a> Response<'a> {
    /// Cast to a serialized Frame as [`serde_json::Value`] object for consistency in jql object
    /// traversal: `"response"."body"` should always traverse a serialized [`Frame`] struct
    fn to_frame_value(&self) -> Result<Value, FrError> {
        Ok(json!({"response":to_value(self)?}))
    }

    pub(crate) fn validate(&self) -> Result<(), FrError> {
        if self.validation.is_none() {
            return Ok(());
        }
        // for now hardcode checking only response body
        for k in self.validation.as_ref().unwrap().keys() {
            if !k.trim_start_matches('.').starts_with("'response'.'body'") {
                return Err(FrError::ReadInstruction(
                    "validation options currently only support the response body",
                ));
            }
        }
        Ok(())
    }

    /// Using the write instructions found in the frame InstructionSet, look for matches to be
    /// passed to write operations
    pub fn match_payload_response(
        &self,
        set: &'a InstructionSet,
        payload_response: &Response,
    ) -> Result<Option<HashMap<&'a str, Value>>, FrError> {
        let frame_response: Value = self.to_frame_value()?;
        let payload_response: Value = payload_response.to_frame_value()?;

        let mut write_matches: HashMap<&str, Value> = HashMap::new();
        for (k, query) in set.writes.iter() {
            // ensure frame jql query returns a string object
            let frame_str = match select_value(&frame_response, query) {
                Ok(Value::String(v)) => Ok(v),
                Ok(_) => Err(FrError::FrameParsef(
                    INVALID_INSTRUCTION_TYPE_ERR,
                    query.to_string(),
                )),
                Err(e) => Err(e),
            }?;
            let payload_val = select_value(&payload_response, query)?;

            if let Value::String(payload_str) = &payload_val {
                let write_match = Register::write_match(k, &frame_str, payload_str)?;
                if let Some(mat) = write_match {
                    write_matches.insert(k, to_value(mat)?);
                }
                continue;
            }
            // handle non string payload values returned by the jql query
            Register::expect_standalone_var(k, &frame_str)?;
            write_matches.insert(k, payload_val);
        }

        if write_matches.iter().next().is_some() {
            return Ok(Some(write_matches));
        }

        Ok(None)
    }

    /// Applies the validations using the BTree key as the Value selector
    pub fn apply_validation(&mut self, other: &mut Self) -> Result<(), FrError> {
        if self.body.is_none() || other.body.is_none() || self.validation.is_none() {
            return Ok(());
        }
        for (k, v) in self.validation.as_ref().unwrap().iter() {
            // if no validator operations are needed
            if !v.partial && !v.unordered {
                continue;
            }

            let selector = new_mut_selector(strip_query(k))?;
            if v.unordered {
                v.apply_unordered(
                    k,
                    &selector,
                    self.body.as_mut().unwrap(),
                    other.body.as_mut().unwrap(),
                )?;
            }
            if v.partial {
                v.apply_partial(
                    k,
                    &selector,
                    self.body.as_mut().unwrap(),
                    other.body.as_mut().unwrap(),
                )?;
            }
        }

        // for comparison's sake set validtion to None once applying is finished
        self.validation = None;

        Ok(())
    }
}

// For now selector queries are only used on the reponse body
// selector logic takes the body Value object while mainting a valid
// "whole file" query for reference's sake
// `"'response'.'body'" => "."`
// `"'response'.'body'.'key'" => ".'key'"`
fn strip_query(query: &str) -> &str {
    let body_query = query
        .trim_start_matches('.')
        .trim_start_matches("'response'.'body'");

    if body_query.is_empty() {
        return ".";
    }
    body_query
}

impl Default for Response<'_> {
    fn default() -> Self {
        Self {
            body:       None,
            etc:        Some(json!({})),
            validation: None,
            status:     0,
        }
    }
}

/// PartialEq needs to exlcude [`Response.validation`] to ensure that [`Response::apply_validation`] can
/// diffentiatiate between the parent `Response` (the one pulled directle from the filmReel file)
/// and the child [`Response`] (one deserialized from returned data) since the client validations
/// should always be[`Option::None`]
impl<'a> PartialEq for Response<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.body.eq(&other.body) && self.etc.eq(&other.etc) && self.status.eq(&other.status)
    }
}

impl<'a> Eq for Response<'a> {}

type Validation<'a> = BTreeMap<Cow<'a, str>, Validator>;

/// Validator represents one validation ruleset applied to a single JSON selection
#[derive(Serialize, Clone, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct Validator {
    partial:   bool,
    unordered: bool,
}

impl Validator {
    fn apply_partial(
        &self,
        query: &str,
        selector: &MutSelector,
        self_body: &mut Value,
        other_body: &mut Value,
    ) -> Result<(), FrError> {
        let selection = selector(self_body)
            .ok_or_else(|| FrError::ReadInstructionf(MISSING_SELECTION_ERR, query.to_string()))?;
        match selection {
            Value::Object(o) => {
                let preserve_keys = o.keys().collect::<Vec<&String>>();
                // if the response selection is not an object or selects nothing (None is returned)
                // return early
                let other_selection = match selector(other_body) {
                    Some(Value::Object(o)) => o,
                    _ => return Ok(()),
                };

                for k in other_selection
                    .keys() // retain keys that are not found in preserve_keys
                    .filter(|k| !preserve_keys.contains(&k))
                    .cloned()
                    .collect::<Vec<String>>()
                {
                    other_selection.remove(&k);
                }
            }
            Value::Array(self_selection) => {
                let other_selection = match selector(other_body) {
                    Some(Value::Array(o)) => o,
                    _ => return Ok(()),
                };

                let self_len = self_selection.len();
                // do not mutate if self_len is greater that other_selection
                if self_len >= other_selection.len() {
                    return Ok(());
                }

                // do a rolling check seeing if self_selection is a subset of other_selection
                // Self:  [            A, B, C]
                // Other: [A, B, B, C, A, B, C]
                //
                // i=0; [ABB] != [ABC]
                // i=1; [BBC] != [ABC]
                // i=2; [BCA] != [ABC]
                // i=3; [CAB] != [ABC]
                // i=4; [ABC] == [ABC]
                //
                // NOTE: Array partial matches need to be ordered as well as contiguous.
                // The example below would not result in a match:
                // Other: [A, B, B, C, A, B, B, C]
                for i in (0..other_selection.len()).into_iter() {
                    if i + self_len > other_selection.len() {
                        // other_selection[i..] is already larger than self_selection here
                        // cannot find a partial match at this point
                        return Ok(());
                    }
                    if &other_selection[i..i + self_len] == self_selection.as_slice() {
                        *other_selection = self_selection.clone();
                        return Ok(()); // partial match has been found, no need to iterate further
                    }
                }
            }
            _ => {
                return Err(FrError::ReadInstruction(
                    "validation selectors must point to a JSON object or array",
                ))
            }
        }
        Ok(())
    }

    fn apply_unordered(
        &self,
        query: &str,
        selector: &MutSelector,
        self_body: &mut Value,
        other_body: &mut Value,
    ) -> Result<(), FrError> {
        let selection = selector(self_body)
            .ok_or_else(|| FrError::ReadInstructionf(MISSING_SELECTION_ERR, query.to_string()))?;
        match selection {
            Value::Object(_) => Ok(()),
            Value::Array(self_selection) => {
                let other_selection = match selector(other_body) {
                    Some(Value::Array(o)) => o,
                    _ => return Ok(()),
                };
                // https://gist.github.com/daboross/976978d8200caf86e02acb6805961195
                let mut other_idx_map: HashMap<Key<Hash>, Vec<usize>> = HashMap::new();
                other_selection
                    .iter()
                    .enumerate()
                    .try_for_each(|(i, v)| match hash_value(v) {
                        Ok(k) => {
                            // if .entry(k) returns a Vec, push i to it
                            // if .entry(k) returns None, insert Vec::new() then push i to it
                            other_idx_map.entry(k).or_insert_with(Vec::new).push(i);
                            Ok(())
                        }
                        Err(e) => Err(e),
                    })?;

                // elements removed form OtherIdxMap are put in
                // placeholder_indices so that Other can be later drained
                // of the appropriate Value::Null elements using `other_selection.retain(...);`
                let mut placeholder_indices: HashSet<usize> = HashSet::new();
                // sink collects successful matches of elements from Other
                // in the sequence (aka ordered but non-consecutive) they are present in Self
                // sink is then prepended to other_selection
                // thus all successful matches (those that are found in both Self and Other)
                // will found at the front of other_selection in sequence
                let mut sink: BTreeMap<usize, Value> = BTreeMap::new();
                /*
                remove from other_selection starting with the last index of other
                and insert into the index of where it is found in self
                ----------------
                Self:  [A, B, C, C]
                Other: [B, A, C, C]
                Sink:  []
                OtherIdxMap/IdxMap: {B: [0], A: [1]:, C: [2, 3]}

                Expected iterations:
                                                                    Sink[       ];Other[B,   A,   C,   C   ];IdxMap{B:[0],A:[1]C:[2,3]}
                i=0 v=A:
                OtherIdxMap[A].remove(0)->1;Null->Other[1]->Sink[0];Sink[A      ];Other[B,   Null,C,   C   ];IdxMap{B:[0],C:[2,3]     }
                i=1 v=B:
                OtherIdxMap[B].remove(0)->0;Null->Other[0]->Sink[1];Sink[A,B    ];Other[Null,Null,C,   C   ];IdxMap{C:[2,3]           }
                i=2 v=C:
                OtherIdxMap[C].remove(0)->2;Null->Other[2]->Sink[2];Sink[A,B,C  ];Other[Null,Null,Null,C   ];IdxMap{C:[3]             }
                i=3 v=C:
                OtherIdxMap[C].remove(0)->3;Null->Other[3]->Sink[3];Sink[A,B,C,C];Other[Null,Null,Null,Null];IdxMap{                  }
                ----------------
                */
                for (to_idx, v) in self_selection.iter().enumerate() {
                    let v_hash = hash_value(v)?;

                    if let Some(other_indices) = other_idx_map.get_mut(&v_hash) {
                        // remove idx from Vec so that it is not reused
                        let from_idx = other_indices.remove(0);
                        // retain reference of key to be removed so we can swap it
                        // with a Null when doing substitution
                        let mut to_value = Value::Null;
                        // swap places with the Null in the match_sink so that values in
                        // Other maintain a valid index reference
                        std::mem::swap(&mut to_value, &mut other_selection[from_idx]);
                        sink.insert(to_idx, to_value);
                        // make sure to remove the Null after iteration
                        placeholder_indices.insert(from_idx);
                    }
                }

                // we've found no intersections; return early
                if sink.is_empty() {
                    return Ok(());
                }

                // retain other_selection elements where index is not in placeholder_indices
                let mut i = 0;
                other_selection.retain(|_| (!placeholder_indices.contains(&i), i += 1).0);
                // https://stackoverflow.com/questions/47037573/how-to-prepend-a-slice-to-a-vec#answer-47037876
                other_selection.splice(0..0, sink.into_iter().map(|(_, v)| v));

                Ok(())
            }
            _ => Err(FrError::ReadInstruction(
                "validation selectors must point to a JSON object or array",
            )),
        }
    }
}

/// hash_value hashes [Value::Object] variants using only the key elements
/// thus partial equality can be done for the sake of ordering:
/// `[{"this":false}, false] ~= [false, {"this":true}]`
/// ---
/// `{"this":true}` will be hashed as `{"this":null}`
/// `{"this":false }` will be hashed as `{"this":null}`
fn hash_value(value: &Value) -> Result<Key<Hash>, HashError> {
    if let Value::Object(obj_map) = value {
        let null_map: Map<String, Value> =
            obj_map.keys().map(|k| (k.clone(), Value::Null)).collect();

        return to_key(&null_map);
    }
    to_key(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from, to};
    use rstest::*;
    use serde_json::json;

    #[test]
    fn test_match_payload_response() {
        let frame = Frame {
            protocol: Protocol::GRPC,
            cut:      InstructionSet {
                reads:          from![],
                writes:         to! ({
                    "USER_ID"=> "'response'.'body'.'id'",
                    "CREATED"=> "'response'.'body'.'created'",
                    "ignore"=> "'response'.'body'.'array'.[0].'ignore'"
                }),
                hydrate_writes: true,
            },
            request:  Request {
                ..Default::default()
            },
            response: Response {
                body: Some(json!({
                    "id": "${USER_ID}",
                    "created": "${CREATED}",
                    "array": [{"ignore":"${ignore}"}]
                })),
                status: 0,
                ..Default::default()
            },
        };

        let payload_response = Response {
            body: Some(json!({
                "id": "ID_010101",
                "created": 101010,
                "array": [{"ignore": "value"}]
            })),
            status: 0,
            ..Default::default()
        };
        let mat = frame
            .response
            .match_payload_response(&frame.cut, &payload_response)
            .unwrap();
        let mut expected_match = HashMap::new();
        expected_match.insert("USER_ID", to_value("ID_010101").unwrap());
        expected_match.insert("CREATED", to_value(101010).unwrap());
        expected_match.insert("ignore", to_value("value").unwrap());
        assert_eq!(expected_match, mat.unwrap());
    }

    const SIMPLE_FRAME: &str = r#"{ "body": %s, "status": 200 }"#;
    const PARTIAL_FRAME: &str = r#"
{
  "validation": {
    "'response'.'body'": {
      "partial": true
    }
  },
  "body": %s,
  "status": 200
}
    "#;
    fn partial_case(case: u32) -> (&'static str, &'static str, bool) {
        let with_obj = r#"{"A":true,"B":true,"C":true}"#;
        let with_arr = r#"["A","B","C"]"#;

        match case {
            1 => (with_obj, r#"{"A":true,"B":true,"C":true}"#, true),
            2 => (with_obj, r#"{"A":true,"B":true,"C":true,"D":true}"#, true),
            3 => (with_obj, r#"{"B":true,"C":true,"D":true}"#, false),
            4 => (
                // explicitly declare partial validation as false
                r#"{"validation":{"'response'.'body'":{"partial":false}},
                    "body":{"A": true,"B": true, "C": true}}"#,
                r#"{"B": true,"C": true, "D": true}"#,
                false,
            ),
            5 => (with_arr, r#"["A", "B", "C"]"#, true),
            6 => (with_arr, r#"["other_value", false, "A", "B", "C"]"#, true),
            7 => (with_arr, r#"["other_value", false, "B", "C"]"#, false),
            _ => panic!(),
        }
    }

    #[rstest(
        t_case,
        case(partial_case(1)),
        case(partial_case(2)),
        case(partial_case(3)),
        case(partial_case(4)),
        case(partial_case(5)),
        case(partial_case(6)),
        case(partial_case(7))
    )]
    fn test_partial_validation(t_case: (&str, &str, bool)) {
        let self_response = str::replace(PARTIAL_FRAME, "%s", t_case.0);
        let other_response = str::replace(SIMPLE_FRAME, "%s", t_case.1);

        let mut frame: Response = serde_json::from_str(&self_response).unwrap();
        let mut other_frame: Response = serde_json::from_str(&other_response).unwrap();
        let should_match = t_case.2;

        frame.apply_validation(&mut other_frame).unwrap();

        if should_match {
            pretty_assertions::assert_eq!(frame, other_frame);
        } else {
            pretty_assertions::assert_ne!(frame, other_frame);
        }
    }

    const UNORDERED_FRAME: &str = r#"
{
  "validation": {
    "'response'.'body'": {
      "unordered": true
    }
  },
  "body": %s,
  "status": 200
}
    "#;
    fn unordered_case(case: u32) -> (&'static str, &'static str, bool) {
        let map_arr = r#"{"A":true,"B":true,"C":true}"#;
        let string_arr = r#"["A","B","C"]"#;
        let with_f32 = r#"["A","B","C",13.37]"#;
        let with_dupes = r#"["A","B","C","A","A"]"#;

        match case {
            1 => (map_arr, r#"{"A":true,"B":true,"C":true}"#, true),
            2 => (map_arr, r#"{"A":true,"B":false,"C":true}"#, false),
            3 => (map_arr, r#"{"A":true,"B":true,"C":true,"D":true}"#, false),
            4 => (map_arr, r#"{"A":true,"B":true}"#, false),
            5 => (map_arr, r#"{"B":true,"C":true,"A":true}"#, true),
            6 => (string_arr, r#"["A","B","C"]"#, true),
            7 => (string_arr, r#"["other_value",false,"A","B","C"]"#, false),
            8 => (string_arr, r#"[false,false,"A","B","C"]"#, false),
            9 => (string_arr, r#"["B","A","C"]"#, true),
            10 => (string_arr, r#"["B","A","D","C"]"#, false),
            11 => (with_f32, r#"["C",13.37,"B","A"]"#, true),
            12 => (with_dupes, r#"["A","C","A","B","A"]"#, true),
            _ => panic!(),
        }
    }

    #[rstest(
        t_case,
        case(unordered_case(1)),
        case(unordered_case(2)),
        case(unordered_case(3)),
        case(unordered_case(4)),
        case(unordered_case(5)),
        case(unordered_case(6)),
        case(unordered_case(7)),
        case(unordered_case(8)),
        case(unordered_case(9)),
        case(unordered_case(10)),
        case(unordered_case(11)),
        case(unordered_case(12))
    )]
    fn test_unordered_validation(t_case: (&str, &str, bool)) {
        let self_response = str::replace(UNORDERED_FRAME, "%s", t_case.0);
        let other_response = str::replace(SIMPLE_FRAME, "%s", t_case.1);

        let mut frame: Response = serde_json::from_str(&self_response).unwrap();
        let mut other_frame: Response = serde_json::from_str(&other_response).unwrap();
        let should_match = t_case.2;

        frame.apply_validation(&mut other_frame).unwrap();
        if should_match {
            pretty_assertions::assert_eq!(frame, other_frame);
        } else {
            pretty_assertions::assert_ne!(frame, other_frame);
        }
    }

    const PARTIAL_UNORDERED: &str = r#"
{
  "validation": {
    "'response'.'body'": {
      "partial": true,
      "unordered": true
    }
  },
  "body": %s,
  "status": 200
}
    "#;

    fn partial_unordered_case(case: u32) -> (&'static str, &'static str, &'static str) {
        match case {
            1 => (
                r#"{"A":true,"B":true,"C":true}"#,
                r#"{"A":true,"B":true,"C":true}"#,
                r#"{"A":true,"B":true,"C":true}"#,
            ),
            2 => (
                r#"{"A":true,"B":[1,0],"C":true}"#,
                r#"{"A":true,"C":true,"B":[0,1]}"#,
                r#"{"A":true,"B":[0,1],"C":true}"#,
            ),
            3 => (
                r#"{"A":true,"B":true,"C":true}"#,
                r#"{"D":true,"B":true,"C":true,"A":true}"#,
                r#"{"A":true,"B":true,"C":true}"#,
            ),
            4 => (
                r#"{"A":true,"B":true,"C":true}"#,
                r#"{"B":true,"A":true,"A":true}"#,
                r#"{"A":true,"B":true}"#,
            ),
            5 => (
                r#"{"A":true,"B":true,"C":true}"#,
                r#"{"B":true,"C":true,"A":true}"#,
                r#"{"A":true,"B":true,"C":true}"#,
            ),
            6 => (r#"["A","B","C"]"#, r#"["F","C","C"]"#, r#"["C","F","C"]"#),
            7 => (
                r#"["A","B","C"]"#,
                r#"["other_value",false,"B","A","C","B"]"#,
                r#"["A","B","C"]"#,
            ),
            8 => (
                r#"["A","B","C"]"#,
                r#"[false,false,"A","B","C"]"#,
                r#"["A","B","C"]"#,
            ),
            9 => (
                r#"[0,"A",0,"C"]"#,
                r#"["B","B","A","C","C","A"]"#,
                r#"["A","C","B","B","C","A"]"#,
            ),
            10 => (
                r#"["A","B","C"]"#,
                r#"["B","A","D","C"]"#,
                r#"["A","B","C"]"#,
            ),
            11 => (
                r#"["A","B","C",13.37]"#,
                r#"["C",13.37,"B","A"]"#,
                r#"["A","B","C",13.37]"#,
            ),
            12 => (
                r#"["A","B","C","A","A"]"#,
                r#"["A","C","A","B","A"]"#,
                r#"["A","B","C","A","A"]"#,
            ),
            13 => (
                // test hash_value
                r#"[0,{"A":1},1,4,5]"#,
                r#"[1,{"A":0},0,2,3]"#,
                r#"[0,{"A":0},1,2,3]"#,
            ),
            14 => (
                // test hash_value, mutliple keys should not
                // have a matching hash of a single key
                r#"[0,{"A":false,"B":true},1]"#,
                r#"[1,{"B":true},0]"#,
                r#"[0,1,{"B":true}]"#,
            ),
            _ => panic!(),
        }
    }

    #[rstest(
        t_case,
        case(partial_unordered_case(1)),
        case(partial_unordered_case(2)),
        case(partial_unordered_case(3)),
        case(partial_unordered_case(4)),
        case(partial_unordered_case(5)),
        case(partial_unordered_case(6)),
        case(partial_unordered_case(7)),
        case(partial_unordered_case(8)),
        case(partial_unordered_case(9)),
        case(partial_unordered_case(10)),
        case(partial_unordered_case(11)),
        case(partial_unordered_case(12)),
        case(partial_unordered_case(13)),
        case(partial_unordered_case(14))
    )]
    fn test_partial_unordered_validation(t_case: (&str, &str, &str)) {
        let self_response = str::replace(PARTIAL_UNORDERED, "%s", t_case.0);
        let other_response = str::replace(SIMPLE_FRAME, "%s", t_case.1);
        let expected_response = str::replace(SIMPLE_FRAME, "%s", t_case.2);

        let mut frame: Response = serde_json::from_str(&self_response).unwrap();
        let mut other_frame: Response = serde_json::from_str(&other_response).unwrap();
        let expected_frame: Response = serde_json::from_str(&expected_response).unwrap();

        frame.apply_validation(&mut other_frame).unwrap();

        // we are matching against what other_frame should look like
        // even it if is not a _full_ match against our initial frame
        pretty_assertions::assert_eq!(other_frame, expected_frame);
    }
}
