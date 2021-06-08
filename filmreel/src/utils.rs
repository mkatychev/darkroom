use crate::error::FrError;
use pest::Parser;
use pest_derive::*;
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
};

/// Serializes a HashMap into a BTreeMap, sorting key order for serialization.
pub fn ordered_str_map<S>(
    map: &HashMap<Cow<str>, Cow<str>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = map.iter().collect();
    ordered.serialize(serializer)
}

/// Serializes a HashSet into a BTreeSet, sorting entry order for serialization.
pub fn ordered_set<S>(set: &HashSet<Cow<str>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeSet<_> = set.iter().collect();
    ordered.serialize(serializer)
}

/// Serializes a HashMap into a BTreeMap, sorting key order for serialization.
pub fn ordered_val_map<S>(map: &HashMap<String, Value>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = map.iter().collect();
    ordered.serialize(serializer)
}

#[cfg(feature = "full_jql")]
pub fn select_value(val: &Value, query: &str) -> Result<Value, FrError> {
    let selectors = query.replace("'", "\"");
    match jql::walker(val, Some(&selectors)) {
        Ok(v) => match v {
            Value::String(_) => Ok(v),
            v => Ok(v),
        },
        Err(e) => Err(FrError::ReadInstructionf("get_jql_value", e)),
    }
}

#[cfg(not(feature = "full_jql"))]
pub fn select_value(val: &Value, query: &str) -> Result<Value, FrError> {
    let selector = new_selector(query)?;
    match selector(val) {
        Some(v) => match v {
            Value::String(_) => Ok(v.clone()),
            v => Ok(v.clone()),
        },
        None => Err(FrError::ReadInstructionf(
            "get_jql_value did not return a selection: {}",
            query.to_string(),
        )),
    }
}

#[derive(Parser)]
#[grammar = "selector.pest"]
pub struct SelectorParser;

pub type Selector = Box<dyn Fn(&'_ Value) -> Option<&'_ Value>>;
pub type MutSelector = Box<dyn Fn(&'_ mut Value) -> Option<&'_ mut Value>>;

pub fn new_mut_selector(query: &str) -> Result<MutSelector, FrError> {
    let pairs = SelectorParser::parse(Rule::selector, query)?
        .next()
        .unwrap();

    // check for token string length to invalidate instances where selector_str is "" or "''", "''.''",
    // etc...
    if pairs.as_str().is_empty() {
        return Err(FrError::ReadInstruction(
            "validation selector cannot have an empty query",
        ));
    }

    let mut generator: Vec<MutSelector> = vec![];
    for pair in pairs.into_inner() {
        match pair.as_rule() {
            Rule::string => {
                let key = pair.as_str().replace("\\'", "'");
                let key_selector: MutSelector =
                    Box::new(move |x: &mut Value| x.get_mut(key.to_owned()));
                generator.push(key_selector);
            }
            Rule::int => {
                let index = pair
                    .as_str()
                    .parse::<usize>()
                    .map_err(|x| FrError::Parse(x.to_string()))?;
                let index_selector: MutSelector = Box::new(move |x: &mut Value| x.get_mut(index));
                generator.push(index_selector);
            }
            // selector will always be the only pair at the top level of the genreated AST
            // all other rules are "silent" and never tokenized, this is represented by the leading
            // underscore in pest:
            //
            // step = _{ outer | index }
            //
            // Therefore all other rules should be unreachable
            Rule::selector | Rule::step | Rule::outer | Rule::char | Rule::index => {
                unreachable!()
            }
        }
    }

    let selector_fn: MutSelector = Box::new(move |x: &mut Value| -> Option<&mut Value> {
        let mut drilled_value = x;
        for sel in generator.iter() {
            drilled_value = sel(drilled_value)?;
        }
        Some(drilled_value)
    });

    Ok(selector_fn)
}

pub fn new_selector(query: &str) -> Result<Selector, FrError> {
    let pairs = SelectorParser::parse(Rule::selector, query)?
        .next()
        .unwrap();

    // check for token string length to invalidate instances where selector_str is "" or "''", "''.''",
    // etc...
    if pairs.as_str().is_empty() {
        return Err(FrError::ReadInstruction(
            "validation selector cannot have an empty query",
        ));
    }

    let mut generator: Vec<Selector> = vec![];
    for pair in pairs.into_inner() {
        match pair.as_rule() {
            Rule::string => {
                let key = pair.as_str().replace("\\'", "'");
                let key_selector: Selector = Box::new(move |x: &Value| x.get(key.to_owned()));
                generator.push(key_selector);
            }
            Rule::int => {
                let index = pair
                    .as_str()
                    .parse::<usize>()
                    .map_err(|x| FrError::Parse(x.to_string()))?;
                let index_selector: Selector = Box::new(move |x: &Value| x.get(index));
                generator.push(index_selector);
            }
            // selector will always be the only pair at the top level of the genreated AST
            // all other rules are "silent" and never tokenized, this is represented by the leading
            // underscore in pest:
            //
            // step = _{ outer | index }
            //
            // Therefore all other rules should be unreachable
            Rule::selector | Rule::step | Rule::outer | Rule::char | Rule::index => {
                unreachable!()
            }
        }
    }

    let selector_fn: Selector = Box::new(move |x: &Value| -> Option<&Value> {
        let mut drilled_value = x;
        for sel in generator.iter() {
            drilled_value = sel(drilled_value)?;
        }
        Some(drilled_value)
    });

    Ok(selector_fn)
}

#[derive(Clone)]
pub(crate) struct MatchQuery {
    pub(crate) query: String,
    // kind: crate::cut::Match<'a>,
}

impl MatchQuery {
    pub(crate) fn new() -> Self {
        Self {
            query: String::new(),
        }
    }
    pub(crate) fn append<T: Into<MatchIndex>>(&mut self, i: T) {
        self.query.push('.');
        self.query.push_str(i.into().to_str());
    }
}

pub struct MatchIndex(String);

impl MatchIndex {
    fn to_str(&self) -> &str {
        self.to_str()
    }
}

impl From<&String> for MatchIndex {
    fn from(i: &String) -> Self {
        MatchIndex(format!("'{}'", i.to_string()))
    }
}

impl From<usize> for MatchIndex {
    fn from(i: usize) -> Self {
        MatchIndex(format!("[{}]", i.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde_json::value::Index;

    const OBJ_JSON: &str = r#"
{
  "key": {
    "obj": true,
    "array": [
      false,
      true
    ]
  },
  "\\'\\'": "value from escaped quotes"
}
"#;
    const ARR_JSON: &str = r#"[{"key":"value"},1,[],[true]]"#;

    type IndexList = Vec<Box<dyn Index>>;
    fn selector_case(case: u32) -> (&'static str, &'static str, IndexList) {
        match case {
            1 => (
                OBJ_JSON,
                "'key'.'array'.[1]",
                vec![Box::new("key"), Box::new("array"), Box::new(1)],
            ),
            2 => (
                OBJ_JSON,
                ".", // returns entire OBJ_JSON
                vec![],
            ),
            3 => (
                OBJ_JSON,
                "'key'.'array'.[3]", // should panic: out of bounds array index of 3
                vec![Box::new("key"), Box::new("array"), Box::new(1)],
            ),
            4 => (
                OBJ_JSON,
                "", // should panic: None is returned due to empty query
                vec![],
            ),
            5 => (ARR_JSON, "[3].[0]", vec![Box::new(3), Box::new(0)]),
            6 => (ARR_JSON, "[0].'key'", vec![Box::new(0), Box::new("key")]),
            // should panic: acessing index 1 of empty array
            7 => (ARR_JSON, "[2].[1]", vec![Box::new(2), Box::new(1)]),
            8 => (OBJ_JSON, r#"'\'\''"#, vec![Box::new("\\'\\'")]),
            _ => panic!(),
        }
    }

    #[rstest(selector_case,
        case(selector_case(1)),
        case(selector_case(2)),
        #[should_panic]
        case(selector_case(3)),
        #[should_panic]
        case(selector_case(4)),
        case(selector_case(5)),
        case(selector_case(6)),
        #[should_panic]
        case(selector_case(7))
            )]
    fn test_obj_selection(selector_case: (&str, &str, IndexList)) {
        let (json_str, query, index_list) = selector_case;
        // 1. create two representations of the deserialized JSON
        let mut actual_value: Value = serde_json::from_str(json_str).expect("from_str error");
        let expected_value: Value = serde_json::from_str(json_str).expect("from_str error");
        // 2, create our selector closure;
        let selector = new_mut_selector(query).unwrap();

        // 3. loop through a list of array and object indices to simulate successive
        // indices so that       | value[a][b][c]
        // can be represented as | value.get(a).unwrap().get(b).unwrap().get(c).unwrap()
        let index_iter = move |value: &Value| -> Value {
            let mut output = value;
            for v in index_list.iter() {
                output = v.index_into(output).unwrap();
            }
            output.clone()
        };
        let expected_selection = index_iter(&expected_value);

        let mut selected_value = selector(&mut actual_value).unwrap();
        // 4. assert that the selector_str matches the expected result using successive
        // Index.index_into(Value) calls
        assert_eq!(&expected_selection, selected_value);

        // 5. assert that our selection can be validly mutated and reflected in the original value
        selected_value = selector(&mut actual_value).unwrap();
        *selected_value = "new_value".into();
        assert_eq!(index_iter(&actual_value), "new_value".to_string());
    }
}
