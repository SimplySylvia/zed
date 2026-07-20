//! plan_lint: the agent runs lint, findings persist as plan-lint comments, and
//! the tool returns them so the agent can auto-fix and re-run (F9.1).

// Synchronous integration test driving the server binary over stdio; the
// disallowed-methods guard on std::process::Command (async-blocking) doesn't apply.
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use plan_core::{Plan, store};
use plan_server::tools;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_lint_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed(dir: &PathBuf, id: &str) {
    // A backend task with no linked acceptance trips every-task-has-tests (blocker).
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": id, "title": "t", "status": "in_review", "rev": 1,
        "thread": "a", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "system": "backend", "acceptance": [] }]
    }))
    .unwrap();
    store::save(dir, &plan).unwrap();
}

#[test]
fn lint_returns_findings_and_persists_plan_lint_comments() {
    let dir = scratch("run");
    seed(&dir, "P");
    let value = tools::lint(&dir, "P").unwrap();
    let findings = value["findings"].as_array().expect("findings array");
    assert!(
        findings
            .iter()
            .any(|finding| finding["rule"] == "every-task-has-tests"),
        "expected every-task-has-tests: {value}"
    );

    let plan = store::load(&dir, "P").unwrap();
    assert!(
        plan.comments
            .iter()
            .any(|comment| comment.author.as_deref() == Some("plan-lint")
                && comment.severity.as_deref() == Some("blocker")),
        "expected a persisted plan-lint blocker comment"
    );
}

#[test]
fn lint_over_mcp() {
    let dir = scratch("mcp");
    seed(&dir, "LED-9");
    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"plan_lint","arguments":{"id":"LED-9"}}}"#,
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
    assert!(
        out.contains("every-task-has-tests"),
        "plan_lint should surface the finding over MCP:\n{out}"
    );

    let plan = store::load(&dir, "LED-9").unwrap();
    assert!(
        plan.comments
            .iter()
            .any(|comment| comment.author.as_deref() == Some("plan-lint"))
    );
}
