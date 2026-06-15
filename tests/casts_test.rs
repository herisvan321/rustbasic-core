use rustbasic_core::support::casts::{
    deserialize_bool, deserialize_json, deserialize_option_bool, deserialize_option_json,
    serialize_bool, serialize_json, serialize_option_bool, serialize_option_json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TestModel {
    #[serde(
        deserialize_with = "deserialize_bool",
        serialize_with = "serialize_bool"
    )]
    is_admin: bool,

    #[serde(
        default,
        deserialize_with = "deserialize_option_bool",
        serialize_with = "serialize_option_bool"
    )]
    is_active: Option<bool>,

    #[serde(
        deserialize_with = "deserialize_json",
        serialize_with = "serialize_json"
    )]
    config: Value,

    #[serde(
        default,
        deserialize_with = "deserialize_option_json",
        serialize_with = "serialize_option_json"
    )]
    preferences: Option<Value>,
}

#[test]
fn test_casts_deserialization() {
    // 1. Test deserialization from SQLite/MySQL integer boolean representations and JSON string representations
    let data = json!({
        "is_admin": 1,
        "is_active": 0,
        "config": "{\"theme\":\"dark\",\"notifications\":true}",
        "preferences": "{\"lang\":\"id\"}"
    });

    let model: TestModel = serde_json::from_value(data).unwrap();
    assert!(model.is_admin);
    assert_eq!(model.is_active, Some(false));
    assert_eq!(model.config["theme"], "dark");
    assert_eq!(model.config["notifications"], true);
    assert_eq!(model.preferences.as_ref().unwrap()["lang"], "id");

    // 2. Test deserialization from native boolean and JSON object representations
    let data_native = json!({
        "is_admin": false,
        "is_active": true,
        "config": {
            "theme": "light",
            "notifications": false
        },
        "preferences": {
            "lang": "en"
        }
    });

    let model_native: TestModel = serde_json::from_value(data_native).unwrap();
    assert!(!model_native.is_admin);
    assert_eq!(model_native.is_active, Some(true));
    assert_eq!(model_native.config["theme"], "light");
    assert_eq!(model_native.config["notifications"], false);
    assert_eq!(model_native.preferences.as_ref().unwrap()["lang"], "en");

    // 3. Test string values representing "1" / "true" / "0" / "false"
    let data_strings = json!({
        "is_admin": "1",
        "is_active": "true",
        "config": "", // empty string deserializes to Value::Null
        "preferences": "null" // null string deserializes to None
    });

    let model_strings: TestModel = serde_json::from_value(data_strings).unwrap();
    assert!(model_strings.is_admin);
    assert_eq!(model_strings.is_active, Some(true));
    assert_eq!(model_strings.config, Value::Null);
    assert_eq!(model_strings.preferences, None);
}

#[test]
fn test_casts_serialization() {
    let model = TestModel {
        is_admin: true,
        is_active: Some(false),
        config: json!({ "theme": "dark" }),
        preferences: Some(json!({ "lang": "id" })),
    };

    let serialized = serde_json::to_value(&model).unwrap();
    
    // Serialization for bool should be native bool
    assert_eq!(serialized["is_admin"], json!(true));
    assert_eq!(serialized["is_active"], json!(false));
    
    // Serialization for JSON/Option<JSON> columns should serialize to stringified JSON representation for DB storage
    assert_eq!(serialized["config"], json!("{\"theme\":\"dark\"}"));
    assert_eq!(serialized["preferences"], json!("{\"lang\":\"id\"}"));
}
