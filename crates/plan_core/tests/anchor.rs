//! Re-anchoring invariants (§7): a comment anchor stays put when its block is
//! unchanged, goes outdated when the quoted text is rewritten, moves when the
//! block is renamed but the quote survives, and detaches when both are gone.

use plan_core::anchor::{ReanchorResult, reanchor};
use plan_core::{Anchor, Plan};

fn plan_with(step_id: &str, text: &str) -> Plan {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "steps": [{ "id": step_id, "text": text }] }]
    }))
    .unwrap()
}

fn anchor(block: &str, quote: &str) -> Anchor {
    serde_json::from_value(serde_json::json!({
        "lens": "tasks", "block": block, "quote": quote
    }))
    .unwrap()
}

#[test]
fn unchanged_block_stays_anchored() {
    let plan = plan_with("s1", "alpha step");
    assert_eq!(
        reanchor(&anchor("t1.s1", "alpha step"), &plan),
        ReanchorResult::Anchored
    );
}

#[test]
fn appended_text_still_anchored() {
    let plan = plan_with("s1", "alpha step, now with more detail");
    assert_eq!(
        reanchor(&anchor("t1.s1", "alpha step"), &plan),
        ReanchorResult::Anchored
    );
}

#[test]
fn rewritten_text_is_outdated() {
    let plan = plan_with("s1", "totally different wording");
    assert_eq!(
        reanchor(&anchor("t1.s1", "alpha step"), &plan),
        ReanchorResult::Outdated
    );
}

#[test]
fn renamed_block_moves_to_where_the_quote_is() {
    let plan = plan_with("s9", "alpha step");
    assert_eq!(
        reanchor(&anchor("t1.s1", "alpha step"), &plan),
        ReanchorResult::Moved {
            block: "t1.s9".into()
        }
    );
}

#[test]
fn gone_block_and_quote_detaches() {
    let plan = plan_with("s9", "totally different wording");
    assert_eq!(
        reanchor(&anchor("t1.s1", "alpha step"), &plan),
        ReanchorResult::Detached
    );
}
