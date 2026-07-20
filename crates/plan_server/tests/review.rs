//! Review tools: the agent lists comments, replies, marks addressed, and applies
//! suggestions — the server side of the comment round-trip.

use std::fs;
use std::path::{Path, PathBuf};

use plan_core::{Anchor, Plan, comments, store};
use plan_server::tools;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_review_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn base_plan() -> Plan {
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

fn seed_comment(dir: &Path) {
    let mut plan = base_plan();
    comments::add_comment(&mut plan, "c1", "flag", "user", Some("blocker"), anchor(), "please fix");
    store::save(dir, &plan).unwrap();
}

#[test]
fn list_comments_returns_open_items() {
    let dir = scratch("list");
    seed_comment(&dir);
    let value = tools::list_comments(&dir, "P").unwrap();
    let items = value.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], "c1");
    assert_eq!(items[0]["block"], "t1.s1");
}

#[test]
fn reply_comment_appends_a_reply() {
    let dir = scratch("reply");
    seed_comment(&dir);
    tools::reply_comment(&dir, "P", "c1", "on it", Some("revised")).unwrap();
    let plan = store::load(&dir, "P").unwrap();
    assert_eq!(plan.comments[0].thread.len(), 2);
}

#[test]
fn mark_addressed_sets_state() {
    let dir = scratch("mark");
    seed_comment(&dir);
    tools::mark_addressed(&dir, "P", "c1").unwrap();
    assert_eq!(
        store::load(&dir, "P").unwrap().comments[0].state.as_deref(),
        Some("addressed")
    );
}

#[test]
fn apply_suggestion_rewrites_the_block() {
    let dir = scratch("apply");
    let mut plan = base_plan();
    comments::add_suggestion(&mut plan, "c1", "user", anchor(), "old text", "new text");
    store::save(&dir, &plan).unwrap();

    tools::apply_suggestion(&dir, "P", "c1").unwrap();
    assert_eq!(
        store::load(&dir, "P").unwrap().tasks[0].steps[0].text.as_deref(),
        Some("new text")
    );
}
