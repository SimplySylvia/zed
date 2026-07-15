//! Comment mutations: add/state/batch-send/apply-suggestion round-trips.

use plan_core::{Anchor, Plan, comments};

fn plan() -> Plan {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "steps": [{ "id": "s1", "text": "old text" }] }]
    }))
    .unwrap()
}

fn anchor() -> Anchor {
    serde_json::from_value(serde_json::json!({
        "lens": "tasks", "block": "t1.s1", "quote": "old text"
    }))
    .unwrap()
}

#[test]
fn add_comment_appends_open_and_bumps_rev() {
    let mut plan = plan();
    comments::add_comment(&mut plan, "c1", "flag", "user", Some("blocker"), anchor(), "please fix");
    assert_eq!(plan.comments.len(), 1);
    assert_eq!(plan.comments[0].state.as_deref(), Some("open"));
    assert_eq!(plan.comments[0].severity.as_deref(), Some("blocker"));
    assert_eq!(plan.rev, 2);
}

#[test]
fn mark_sent_batch_moves_open_to_sent() {
    let mut plan = plan();
    comments::add_comment(&mut plan, "c1", "comment", "user", None, anchor(), "a");
    comments::add_comment(&mut plan, "c2", "comment", "user", None, anchor(), "b");
    assert_eq!(comments::mark_sent_batch(&mut plan), 2);
    assert!(plan.comments.iter().all(|c| c.state.as_deref() == Some("sent")));
}

#[test]
fn apply_suggestion_replaces_block_text() {
    let mut plan = plan();
    comments::add_suggestion(&mut plan, "c1", "user", anchor(), "old text", "new shiny text");
    assert!(comments::apply_suggestion(&mut plan, "c1"));
    assert_eq!(plan.tasks[0].steps[0].text.as_deref(), Some("new shiny text"));
    assert_eq!(plan.comments[0].state.as_deref(), Some("addressed"));
}

#[test]
fn set_comment_state_updates() {
    let mut plan = plan();
    comments::add_comment(&mut plan, "c1", "comment", "user", None, anchor(), "a");
    assert!(comments::set_comment_state(&mut plan, "c1", "resolved"));
    assert_eq!(plan.comments[0].state.as_deref(), Some("resolved"));
}

#[test]
fn reply_appends_a_thread_entry() {
    let mut plan = plan();
    comments::add_comment(&mut plan, "c1", "comment", "user", None, anchor(), "a");
    assert!(comments::reply(&mut plan, "c1", "agent", "on it", Some("revised")));
    assert_eq!(plan.comments[0].thread.len(), 2);
    assert_eq!(plan.comments[0].thread[1].author.as_deref(), Some("agent"));
}
