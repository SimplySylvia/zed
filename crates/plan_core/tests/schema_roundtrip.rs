//! Schema fidelity: the LED-212 fixture parses to the expected typed model,
//! round-trips losslessly, and preserves fields the schema doesn't yet know.

use plan_core::Plan;

const FIXTURE: &str = include_str!("fixtures/led-212.plan.json");

#[test]
fn fixture_parses_to_expected_values() {
    let plan: Plan = serde_json::from_str(FIXTURE).expect("fixture should parse");
    assert_eq!(plan.id, "LED-212");
    assert_eq!(plan.rev, 5);
    assert_eq!(plan.tasks.len(), 2);
    assert_eq!(plan.spec.acceptance.len(), 2);
    assert_eq!(plan.tickets.len(), 1);

    // The approval guard on t2.s2 is captured with its type.
    let guard = plan.tasks[1].steps[1]
        .guard
        .as_ref()
        .expect("t2.s2 should carry a guard");
    assert_eq!(guard.guard_type.as_deref(), Some("approve"));

    // Completeness: every field in the fixture was recognized by a typed field,
    // so nothing fell through to the `extra` forward-compat bucket.
    assert!(
        plan.extra.is_empty(),
        "unexpected unmodeled top-level fields: {:?}",
        plan.extra.keys().collect::<Vec<_>>()
    );
    assert!(plan.spec.extra.is_empty(), "unmodeled spec fields");
    assert!(plan.tasks[0].extra.is_empty(), "unmodeled task fields");
}

#[test]
fn fixture_round_trips_losslessly() {
    let plan: Plan = serde_json::from_str(FIXTURE).expect("fixture should parse");
    let v1 = serde_json::to_value(&plan).unwrap();
    let plan2: Plan = serde_json::from_value(v1.clone()).unwrap();
    let v2 = serde_json::to_value(&plan2).unwrap();
    assert_eq!(v1, v2, "serialization must be a stable fixed point");
    assert_eq!(plan, plan2, "typed model must survive a round trip unchanged");
}

#[test]
fn unknown_fields_are_preserved() {
    // A newer writer adds a field this schema version doesn't model; it must
    // survive a load -> save cycle rather than being silently dropped (§11).
    let mut value: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("future_field".into(), serde_json::json!({ "kept": true }));

    let plan: Plan = serde_json::from_value(value).unwrap();
    let reserialized = serde_json::to_value(&plan).unwrap();
    assert_eq!(
        reserialized.get("future_field"),
        Some(&serde_json::json!({ "kept": true })),
        "unknown field must be preserved through round trip"
    );
}
