//! Launch: the agent (or UI) transitions an approved plan to executing + takes the
//! lease, which flips the PreToolUse edit gate from denied to allowed (F4.1, F6.3).

// Synchronous integration test driving the server binary over stdio; the
// disallowed-methods guard on std::process::Command (async-blocking) doesn't apply.
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use plan_core::{Plan, Status, store};
use plan_server::{hooks, tools};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_exec_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed(dir: &PathBuf, id: &str) {
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": id, "title": "t", "status": "approved", "rev": 1,
        "thread": "acp-1", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "status": "pending" }]
    }))
    .unwrap();
    store::save(dir, &plan).unwrap();
}

#[test]
fn launch_sets_executing_and_lease() {
    let dir = scratch("launch");
    seed(&dir, "P");
    tools::launch(&dir, "P", "acp-1").unwrap();
    let plan = store::load(&dir, "P").unwrap();
    assert_eq!(plan.status, Status::Executing);
    assert_eq!(plan.executor.as_ref().unwrap().thread, "acp-1");
}

#[test]
fn pretooluse_gate_denies_before_and_allows_after_launch() {
    let dir = scratch("gate");
    seed(&dir, "P");
    // Approved (not executing) → edits are denied.
    assert!(hooks::pretooluse_allows_edit(&dir, "Edit").is_err());
    tools::launch(&dir, "P", "acp-1").unwrap();
    // Executing → edits are allowed.
    assert!(hooks::pretooluse_allows_edit(&dir, "Edit").is_ok());
}

#[test]
fn launch_over_mcp() {
    let dir = scratch("mcp");
    seed(&dir, "LED-9");
    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"plan_launch","arguments":{"id":"LED-9","thread":"acp-1"}}}"#,
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
    assert_eq!(plan.status, Status::Executing);
    assert_eq!(plan.executor.as_ref().unwrap().thread, "acp-1");
}
