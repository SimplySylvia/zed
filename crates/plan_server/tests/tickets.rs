//! Ticket store + acceptance authoring + resync + the Done coverage gate
//! (F2.4/F2.4f/F2.4g).

#![allow(clippy::disallowed_methods)]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use plan_core::{Plan, store};
use plan_server::tools;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plan_server_tickets_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A minimal drafting plan (no tickets, no acceptance yet).
fn seed(plans_dir: &Path, status: &str) {
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "P", "title": "t", "status": status, "rev": 1,
        "thread": "acp-1", "spec": { "goal": "g" },
        "tasks": [{ "id": "t1", "status": "in_progress" }]
    }))
    .unwrap();
    store::save(plans_dir, &plan).unwrap();
}

#[test]
fn set_tickets_stores_the_array() {
    let dir = scratch("set");
    seed(&dir, "drafting");
    tools::set_tickets(
        &dir,
        "P",
        serde_json::json!([{ "source": "jira", "key": "LED-1", "ac": ["one", "two"] }]),
    )
    .unwrap();
    let plan = store::load(&dir, "P").unwrap();
    assert_eq!(plan.tickets.len(), 1);
    assert_eq!(plan.tickets[0].key, "LED-1");
    assert_eq!(plan.tickets[0].ac, vec!["one", "two"]);
}

#[test]
fn add_acceptance_appends_a_mapped_criterion() {
    let dir = scratch("accept");
    seed(&dir, "drafting");
    tools::add_acceptance(
        &dir,
        "P",
        "a1",
        Some("the route loads"),
        Some("return the page slice"),
        Some("LED-1#1"),
        vec!["t1".to_string()],
    )
    .unwrap();
    let plan = store::load(&dir, "P").unwrap();
    let acceptance = &plan.spec.acceptance[0];
    assert_eq!(acceptance.id, "a1");
    assert_eq!(acceptance.ticket_ac.as_deref(), Some("LED-1#1"));
    assert_eq!(acceptance.tasks, vec!["t1"]);
}

#[test]
fn resync_ticket_stamps_drift_on_an_ac_change() {
    let dir = scratch("resync");
    seed(&dir, "executing");
    tools::set_tickets(
        &dir,
        "P",
        serde_json::json!([{ "source": "jira", "key": "LED-1", "status": "To Do", "ac": ["one"] }]),
    )
    .unwrap();
    // A fresh fetch adds an AC → drift stamped, snapshot updated.
    tools::resync_ticket(
        &dir,
        "P",
        "LED-1",
        serde_json::json!({ "source": "jira", "key": "LED-1", "status": "To Do", "ac": ["one", "three"] }),
    )
    .unwrap();
    let plan = store::load(&dir, "P").unwrap();
    let ticket = &plan.tickets[0];
    assert_eq!(ticket.ac, vec!["one", "three"]);
    let drift = ticket.drift.as_ref().expect("drift stamped");
    assert_eq!(drift.get("ac_changed").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn set_status_done_blocked_until_ticket_acs_covered() {
    let dir = scratch("donegate");
    seed(&dir, "executing");
    tools::set_tickets(
        &dir,
        "P",
        serde_json::json!([{ "source": "jira", "key": "LED-1", "ac": ["one", "two"] }]),
    )
    .unwrap();
    // Only #1 mapped → Done is blocked (default ticket-coverage = blocker).
    tools::add_acceptance(&dir, "P", "a1", None, None, Some("LED-1#1"), vec![]).unwrap();
    assert!(tools::set_status(&dir, "P", "done").is_err());

    // Cover #2 → Done allowed.
    tools::add_acceptance(&dir, "P", "a2", None, None, Some("LED-1#2"), vec![]).unwrap();
    assert!(tools::set_status(&dir, "P", "done").is_ok());
}

#[test]
fn set_tickets_over_mcp() {
    let dir = scratch("mcp");
    seed(&dir, "drafting");
    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"plan_set_tickets","arguments":{"id":"P","tickets":[{"source":"jira","key":"LED-1","ac":["one"]}]}}}"#,
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
    child.stdin.take().unwrap().write_all(messages.as_bytes()).unwrap();
    let mut out = String::new();
    child.stdout.take().unwrap().read_to_string(&mut out).unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(store::load(&dir, "P").unwrap().tickets[0].key, "LED-1");
}
