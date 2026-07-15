//! Staged revisions: the agent proposes a plan change via plan_propose_revision;
//! it lands as an inert pending diff (F9.3). Apply/Reject stay user-side (in the UI).

// The disallowed-methods lint on std::process::Command guards against blocking an
// async runtime; this is a synchronous integration test that drives the server
// binary over stdio, where blocking is intended.
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use plan_core::rev::HunkSpec;
use plan_core::{Plan, store};
use plan_server::tools;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_revision_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed(dir: &PathBuf) {
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "steps": [{ "id": "s1", "text": "old text" }] }]
    }))
    .unwrap();
    store::save(dir, &plan).unwrap();
}

#[test]
fn propose_revision_stages_an_inert_pending_diff() {
    let dir = scratch("propose");
    seed(&dir);
    tools::propose_revision(
        &dir,
        "P",
        vec![HunkSpec {
            target: Some("t1.s1".into()),
            old: Some("old text".into()),
            new: Some("new text".into()),
            from: None,
        }],
    )
    .unwrap();

    let plan = store::load(&dir, "P").unwrap();
    let pending = plan.pending_revision.expect("staged");
    assert_eq!(pending.hunks.len(), 1);
    assert_eq!(pending.hunks[0].id, "h1");
    assert_eq!(pending.hunks[0].state.as_deref(), Some("pending"));
    // Staging must be inert: the block and rev are unchanged until the user applies.
    assert_eq!(plan.tasks[0].steps[0].text.as_deref(), Some("old text"));
    assert_eq!(plan.rev, 1);
}

#[test]
fn propose_revision_over_mcp() {
    // Pre-seed the plan on disk: rmcp handles batched requests concurrently, so
    // chaining plan_create -> plan_propose_revision in one send would race. This
    // still exercises the full stdio/tool path for plan_propose_revision.
    let dir = scratch("mcp");
    let seeded: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "LED-9", "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "steps": [{ "id": "s1", "text": "old text" }] }]
    }))
    .unwrap();
    store::save(&dir, &seeded).unwrap();

    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"plan_propose_revision","arguments":{"id":"LED-9","hunks":[{"target":"t1.s1","new":"revised","from":"c1"}]}}}"#,
    ]
    .join("\n")
        + "\n";

    let mut child = Command::new(env!("CARGO_BIN_EXE_plan_server"))
        .env("PLAN_PLANS_DIR", &dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn plan_server");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(messages.as_bytes())
        .unwrap();
    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    assert!(child.wait().unwrap().success());

    let plan = store::load(&dir, "LED-9").unwrap();
    let pending = plan.pending_revision.expect("pending revision staged over MCP");
    assert_eq!(pending.hunks.len(), 1);
    assert_eq!(pending.hunks[0].new.as_deref(), Some("revised"));
}
