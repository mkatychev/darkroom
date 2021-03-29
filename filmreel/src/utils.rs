use crate::error::FrError;
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Serializes a HashMap into a BTreeMap, sorting key order for serialization.
pub fn ordered_str_map<S>(map: &HashMap<&str, &str>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = map.iter().collect();
    ordered.serialize(serializer)
}

/// Serializes a HashSet into a BTreeSet, sorting entry order for serialization.
pub fn ordered_set<S>(set: &HashSet<&str>, serializer: S) -> Result<S::Ok, S::Error>
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

pub fn get_jql_value(val: &Value, query: &str) -> Result<Value, FrError> {
    let selectors = query.replace("'", "\"");
    match jql::walker(val, Some(&selectors)) {
        Ok(v) => match v {
            Value::String(_) => Ok(v),
            v => Ok(v),
        },
        Err(e) => Err(FrError::ReadInstructionf("get_jql_value", e)),
    }
}
