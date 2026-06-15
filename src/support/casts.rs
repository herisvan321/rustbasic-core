use serde::{Deserialize, Deserializer, Serializer};
use serde_json::Value;

/// Custom deserializer for boolean that handles SQLite/MySQL 0/1 integers
pub fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::Bool(b) => Ok(b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i != 0)
            } else {
                Ok(false)
            }
        }
        Value::String(ref s) => {
            Ok(s == "1" || s.to_lowercase() == "true")
        }
        _ => Ok(false),
    }
}

pub fn serialize_bool<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bool(*value)
}

/// Custom deserializer for Option<bool> that handles SQLite/MySQL 0/1 integers
pub fn deserialize_option_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<Value>::deserialize(deserializer)?;
    match opt {
        Some(Value::Bool(b)) => Ok(Some(b)),
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                Ok(Some(i != 0))
            } else {
                Ok(Some(false))
            }
        }
        Some(Value::String(ref s)) => {
            Ok(Some(s == "1" || s.to_lowercase() == "true"))
        }
        _ => Ok(None),
    }
}

pub fn serialize_option_bool<S>(value: &Option<bool>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(b) => serializer.serialize_some(b),
        None => serializer.serialize_none(),
    }
}

/// Custom deserializer for JSON TEXT columns in database
pub fn deserialize_json<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::String(ref s) => {
            if s.is_empty() {
                Ok(Value::Null)
            } else {
                serde_json::from_str(s).map_err(serde::de::Error::custom)
            }
        }
        other => Ok(other),
    }
}

pub fn serialize_json<S>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = serde_json::to_string(value).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&s)
}

/// Custom deserializer for Option<Value> JSON TEXT columns in database
pub fn deserialize_option_json<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<Value>::deserialize(deserializer)?;
    match opt {
        Some(Value::String(ref s)) => {
            if s.is_empty() || s == "null" {
                Ok(None)
            } else {
                let val = serde_json::from_str(s).map_err(serde::de::Error::custom)?;
                Ok(Some(val))
            }
        }
        Some(other) => Ok(Some(other)),
        None => Ok(None),
    }
}

pub fn serialize_option_json<S>(value: &Option<Value>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(val) => {
            let s = serde_json::to_string(val).map_err(serde::ser::Error::custom)?;
            serializer.serialize_some(&s)
        }
        None => serializer.serialize_none(),
    }
}
