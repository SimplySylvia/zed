//! Execution control (pause/resume/stop), amendments, recovery, and the Stop loop
//! guard (F4.7/F11.4/F5.1/F5.6/F11.1/F6.3).

// Synchronous integration test driving the server binary over stdio; the
// disallowed-methods guard on std::process::Command (async-blocking) doesn't apply.
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use plan_core::rev::HunkSpec;
use plan_core::{Plan, Status, TaskStatus, store};
use plan_server::{hooks, tools};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_control_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed(dir: &PathBuf) {
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "executing", "rev": 1,
        "thread": "acp-1", "executor": { "thread": "acp-1" },
        "spec": { "goal": "g" },
        "tasks": [
            { "id": "t1", "status": "in_progress" },
            { "id": "t2", "status": "pending" }
        ]
    }))
    .unwrap();
    store::save(dir, &plan).unwrap();
}

#[test]
fn pause_resume_stop_over_tools() {
    let dir = scratch("control");
    seed(&dir);
    tools::pause(&dir, "P").unwrap();
    assert_eq!(store::load(&dir, "P").unwrap().status, Status::Paused);
    tools::resume(&dir, "P").unwrap();
    assert_eq!(store::load(&dir, "P").unwrap().status, Status::Executing);
    tools::stop(&dir, "P").unwrap();
    let plan = store::load(&dir, "P").unwrap();
    assert_eq!(plan.status, Status::Paused);
    assert!(plan.executor.is_none());
}

#[test]
fn propose_amendment_stages_a_tagged_revision() {
    let dir = scratch("amend");
    seed(&dir);
    tools::propose_amendment(
        &dir,
        "P",
        "t1",
        vec![HunkSpec {
            target: Some("t1".into()),
            old: None,
            new: Some("revised".into()),
            from: None,
        }],
    )
    .unwrap();
    let plan = store::load(&dir, "P").unwrap();
    let pending = plan.pending_revision.expect("staged");
    assert_eq!(
        pending.extra.get("kind").and_then(|v| v.as_str()),
        Some("amendment")
    );
}

#[test]
fn recover_task_over_tool() {
    let dir = scratch("recover");
    seed(&dir);
    tools::recover_task(&dir, "P", "t1", "manual").unwrap();
    let plan = store::load(&dir, "P").unwrap();
    assert!(plan.tasks[0].manual);
    assert_eq!(plan.tasks[0].status, TaskStatus::Skipped);
}

#[test]
fn stop_loop_guard_nudges_while_work_remains_but_not_when_active() {
    let dir = scratch("stopguard");
    seed(&dir);
    // Executing with unfinished tasks and no hold → nudge to continue.
    assert!(hooks::stop_loop_guard(&dir, false).is_some());
    // Runaway protection: once the stop hook already fired, allow the stop.
    assert!(hooks::stop_loop_guard(&dir, true).is_none());
    // Paused → no nudge.
    tools::pause(&dir, "P").unwrap();
    assert!(hooks::stop_loop_guard(&dir, false).is_none());
}

#[test]
fn pause_over_mcp() {
    let dir = scratch("mcp");
    seed(&dir);
    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"plan_pause","arguments":{"id":"P"}}}"#,
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
    assert_eq!(store::load(&dir, "P").unwrap().status, Status::Paused);
}
