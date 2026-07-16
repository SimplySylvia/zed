//! Ticket coverage + drift (F2.4f/g): every ticket AC maps to a plan criterion,
//! uncovered ACs block Done, and a fresh fetch is comparable to the stored snapshot.

use plan_core::lint::Severity;
use plan_core::tickets::{self, CoverageState};
use plan_core::{Plan, Ticket};

const FIXTURE: &str = include_str!("fixtures/led-212.plan.json");

fn plan(value: serde_json::Value) -> Plan {
    serde_json::from_value(value).unwrap()
}

fn ticket(value: serde_json::Value) -> Ticket {
    serde_json::from_value(value).unwrap()
}

#[test]
fn ticket_ac_id_is_key_hash_one_based() {
    assert_eq!(tickets::ticket_ac_id("LED-212", 1), "LED-212#1");
    assert_eq!(tickets::ticket_ac_id("LED-212", 4), "LED-212#4");
}

#[test]
fn uncovered_lists_ticket_acs_without_a_mapped_criterion() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "tickets": [{ "source": "jira", "key": "LED-1", "ac": ["one", "two"] }],
        "spec": { "goal": "g", "acceptance": [{ "id": "a1", "ticket_ac": "LED-1#1" }] }
    }));
    let uncovered = tickets::uncovered_ticket_acs(&plan);
    assert_eq!(uncovered.len(), 1);
    assert_eq!(uncovered[0].ticket_key, "LED-1");
    assert_eq!(uncovered[0].index, 2);
    assert_eq!(uncovered[0].text, "two");
}

#[test]
fn ticketless_plan_has_no_uncovered_acs() {
    let plan = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" }
    }));
    assert!(tickets::uncovered_ticket_acs(&plan).is_empty());
}

#[test]
fn the_fixture_is_fully_covered() {
    // The LED-212 fixture is the clean/complete baseline: every ticket AC maps.
    let plan: Plan = serde_json::from_str(FIXTURE).unwrap();
    assert!(
        tickets::uncovered_ticket_acs(&plan).is_empty(),
        "fixture has uncovered ticket ACs: {:?}",
        tickets::uncovered_ticket_acs(&plan)
    );
}

#[test]
fn coverage_state_is_unmapped_covered_or_needs_update() {
    // a1 maps LED-1#1 (covered); LED-1#2 is unmapped.
    let mapped = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "tickets": [{ "source": "jira", "key": "LED-1", "ac": ["one", "two"] }],
        "spec": { "goal": "g", "acceptance": [{ "id": "a1", "ticket_ac": "LED-1#1" }] }
    }));
    let coverage = tickets::coverage(&mapped);
    assert_eq!(coverage[0].state, CoverageState::Covered);
    assert_eq!(coverage[1].state, CoverageState::Unmapped);

    // A ticket carrying drift flips its mapped ACs to needs-update.
    let drifted = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "tickets": [{ "source": "jira", "key": "LED-1", "ac": ["one"], "drift": { "ac_changed": true } }],
        "spec": { "goal": "g", "acceptance": [{ "id": "a1", "ticket_ac": "LED-1#1" }] }
    }));
    assert_eq!(tickets::coverage(&drifted)[0].state, CoverageState::NeedsUpdate);
}

#[test]
fn ticket_drift_detects_status_and_ac_changes() {
    let stored = ticket(serde_json::json!({
        "source": "jira", "key": "LED-1", "status": "To Do", "ac": ["a", "b"]
    }));
    let same = stored.clone();
    assert!(tickets::ticket_drift(&stored, &same).is_none());

    let status = ticket(serde_json::json!({
        "source": "jira", "key": "LED-1", "status": "In Progress", "ac": ["a", "b"]
    }));
    let drift = tickets::ticket_drift(&stored, &status).expect("status drift");
    assert!(drift.status_changed && !drift.ac_changed);

    let acs = ticket(serde_json::json!({
        "source": "jira", "key": "LED-1", "status": "To Do", "ac": ["a", "b", "c"]
    }));
    let drift = tickets::ticket_drift(&stored, &acs).expect("ac drift");
    assert!(drift.ac_changed && !drift.status_changed);
}

#[test]
fn coverage_blocks_done_unless_covered_or_disabled() {
    let uncovered = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "executing", "rev": 1,
        "thread": "a", "tickets": [{ "source": "jira", "key": "LED-1", "ac": ["one", "two"] }],
        "spec": { "goal": "g", "acceptance": [{ "id": "a1", "ticket_ac": "LED-1#1" }] }
    }));
    assert!(tickets::coverage_blocks_done(&uncovered, Severity::Blocker).is_some());
    // Disabled severity never blocks.
    assert!(tickets::coverage_blocks_done(&uncovered, Severity::Off).is_none());

    // A waived-but-mapped criterion counts as a recorded decision → covered.
    let covered = plan(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "executing", "rev": 1,
        "thread": "a", "tickets": [{ "source": "jira", "key": "LED-1", "ac": ["one", "two"] }],
        "spec": { "goal": "g", "acceptance": [
            { "id": "a1", "ticket_ac": "LED-1#1" },
            { "id": "a2", "ticket_ac": "LED-1#2", "waived": { "by": "me", "reason": "out of scope" } }
        ] }
    }));
    assert!(tickets::coverage_blocks_done(&covered, Severity::Blocker).is_none());
}
