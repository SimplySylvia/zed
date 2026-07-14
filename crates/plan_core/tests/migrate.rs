//! schema_version handling: v1 is unchanged, a missing version is treated as
//! current (lenient, for hand-written plans), and a newer version is rejected
//! rather than silently mangled.

use plan_core::{SCHEMA_VERSION, migrate};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("fixtures/led-212.plan.json");

#[test]
fn v1_plan_migrates_unchanged() {
    let raw: Value = serde_json::from_str(FIXTURE).unwrap();
    let migrated = migrate(raw.clone()).unwrap();
    assert_eq!(migrated, raw);
}

#[test]
fn missing_schema_version_defaults_to_current() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value.as_object_mut().unwrap().remove("schema_version");
    let migrated = migrate(value).unwrap();
    assert_eq!(migrated.get("schema_version"), Some(&json!(SCHEMA_VERSION)));
}

#[test]
fn future_schema_version_is_rejected() {
    let mut value: Value = serde_json::from_str(FIXTURE).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("schema_version".into(), json!(SCHEMA_VERSION + 1));
    let err = migrate(value).unwrap_err();
    assert!(
        err.to_string().contains("schema_version"),
        "error should mention schema_version: {err}"
    );
}
