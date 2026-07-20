//! Gates + step guards enforced at the PreToolUse hook, cleared via the tools
//! (F4.5/F4.5b).

// Synchronous integration test driving the server binary over stdio; the
// disallowed-methods guard on std::process::Command (async-blocking) doesn't apply.
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use plan_core::{Plan, store};
use plan_server::{hooks, tools};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_guards_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed(dir: &PathBuf) {
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": "executing", "rev": 1,
        "thread": "acp-1", "executor": { "thread": "acp-1" },
        "spec": { "goal": "g", "acceptance": [{ "id": "a1", "tasks": ["t1"] }] },
        "tasks": [
            { "id": "t1", "status": "in_progress", "acceptance": ["a1"], "steps": [
                { "id": "s1", "text": "Run the DB migration",
                  "guard": { "type": "approve", "prompt": "about to run prisma migrate — approve to run", "state": "pending", "evidence_for": ["a1"] } }
            ] },
            { "id": "t2", "title": "Sign-off", "gate": true, "status": "pending" }
        ]
    }))
    .unwrap();
    store::save(dir, &plan).unwrap();
}

#[test]
fn holding_guard_blocks_edits_until_cleared() {
    let dir = scratch("hold");
    seed(&dir);
    tools::hold_guard(&dir, "P", "t1", "s1").unwrap();
    assert!(hooks::pretooluse_gate(&dir, "Edit", None).is_err());
    tools::clear_guard(&dir, "P", "t1", "s1", None).unwrap();
    assert!(hooks::pretooluse_gate(&dir, "Edit", None).is_ok());
}

#[test]
fn require_on_command_blocked_until_a_covering_guard_is_cleared() {
    let dir = scratch("require");
    seed(&dir);
    fs::write(
        dir.join("policy.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "guards": { "require_on": ["prisma migrate"], "default_type": "approve" }
        }))
        .unwrap(),
    )
    .unwrap();

    // The command matches require_on and no covering guard is cleared yet → denied.
    assert!(hooks::pretooluse_gate(&dir, "Bash", Some("npx prisma migrate dev")).is_err());
    // A non-matching command is fine.
    assert!(hooks::pretooluse_gate(&dir, "Bash", Some("ls -la")).is_ok());
    // Clearing the guard (its prompt covers the pattern) allows the command.
    tools::clear_guard(&dir, "P", "t1", "s1", None).unwrap();
    assert!(hooks::pretooluse_gate(&dir, "Bash", Some("npx prisma migrate dev")).is_ok());
}

#[test]
fn gate_hold_blocks_until_approved() {
    let dir = scratch("gate");
    seed(&dir);
    // Mark the gate task in progress → it becomes the active hold.
    let mut plan = store::load(&dir, "P").unwrap();
    plan.tasks
        .iter_mut()
        .find(|task| task.id == "t2")
        .unwrap()
        .status = plan_core::TaskStatus::InProgress;
    store::save(&dir, &plan).unwrap();

    assert!(hooks::pretooluse_gate(&dir, "Edit", None).is_err());
    tools::approve_gate(&dir, "P", "t2").unwrap();
    assert!(hooks::pretooluse_gate(&dir, "Edit", None).is_ok());
}

#[test]
fn clear_guard_over_mcp() {
    let dir = scratch("mcp");
    seed(&dir);
    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"plan_clear_guard","arguments":{"id":"P","task":"t1","step":"s1","response":"saw 0 rows"}}}"#,
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

    let plan = store::load(&dir, "P").unwrap();
    let guard = plan.tasks[0].steps[0].guard.as_ref().unwrap();
    assert_eq!(guard.state.as_deref(), Some("cleared"));
    assert_eq!(guard.response.as_ref().unwrap(), &serde_json::json!("saw 0 rows"));
}
