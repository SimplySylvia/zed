//! End-to-end over MCP: create a plan then read it back through the server's
//! stdio interface, exercising schemars arg parsing + plan_core reuse.

// Synchronous integration test driving the server binary over stdio; the
// disallowed-methods guard on std::process::Command (async-blocking) doesn't apply.
#![allow(clippy::disallowed_methods)]

use std::io::{Read, Write};
use std::process::{Command, Stdio};

fn messages() -> String {
    [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"plan_create","arguments":{"id":"LED-7","title":"Pilot","goal":"Prove the loop","thread":"acp-x"}}}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"plan_get","arguments":{"id":"LED-7"}}}"#,
    ]
    .join("\n")
        + "\n"
}

#[test]
fn create_then_get_over_mcp() {
    let dir = std::env::temp_dir().join("plan_server_mcp_tools");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

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
        .write_all(messages().as_bytes())
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
        out.contains("Pilot"),
        "plan_create/plan_get round trip should surface the title:\n{out}"
    );
    assert!(
        dir.join("LED-7.plan.json").exists(),
        "plan_create should have written the plan file"
    );
}
