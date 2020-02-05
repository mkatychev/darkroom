use jql;
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Serializes a HashMap into a BTreeMap, sorting key order for serialization.
pub fn ordered_map<S>(map: &HashMap<&str, &str>, serializer: S) -> Result<S::Ok, S::Error>
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
///
/// Serializes a HashMap into a BTreeMap, sorting key order for serialization.
pub fn ordered_string_map<S>(map: &HashMap<&str, String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = map.iter().collect();
    ordered.serialize(serializer)
}

/// test_ser_de tests the serialization and deserialization of frame structs
///
/// ```edition2018
///  test_ser_de!(
///      protocol_grpc_ser,  // serialization test name
///      protocol_grpc_de,   // deserialization test name
///      Protocol::GRPC,     // struct/enum to test
///      PROTOCOL_GRPC_JSON  // json string to test
/// );
/// ```
#[cfg(test)]
#[macro_export]
macro_rules! test_ser_de {
    ($ser:ident, $de:ident, $struct:expr, $str_json:expr) => {
        #[test]
        fn $ser() {
            let str_val: serde_json::Value = serde_json::from_str($str_json).unwrap();
            let actual = serde_json::value::to_value(&$struct).unwrap();
            assert_eq!(str_val, actual);
        }
        #[test]
        fn $de() {
            dbg!(&$str_json);
            crate::utils::test_deserialize($struct, $str_json)
        }
    };
}

#[cfg(test)]
pub fn test_deserialize<'a, T>(de_json: T, str_json: &'a str)
where
    T: serde::Deserialize<'a> + PartialEq + std::fmt::Debug,
{
    let actual = serde_json::from_str(str_json).unwrap();
    assert_eq!(de_json, actual);
}

pub fn get_jql_string(val: &Value, query: &str) -> Result<String, String> {
    dbg!(val);
    let selectors = query.replace("'", "\"");
    match jql::walker(val, Some(&selectors)) {
        Ok(v) => match v {
            Value::String(string) => Ok(string),
            v => {
                dbg!(v);
                panic!("get_jql_string: invalid match");
            }
        },
        Err(e) => return Err(e),
    }
}
