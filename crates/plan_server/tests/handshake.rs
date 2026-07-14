//! End-to-end: the built binary speaks the MCP stdio handshake — the exact flow
//! Claude Code runs when it connects (initialize -> tools/list -> tools/call).

use std::io::{Read, Write};
use std::process::{Command, Stdio};

const HANDSHAKE: &str = concat!(
    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#,
    "\n",
    r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    "\n",
    r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    "\n",
    r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"plan_ping","arguments":{}}}"#,
    "\n",
);

#[test]
fn mcp_stdio_handshake_lists_and_calls_plan_ping() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_plan_server"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn plan_server");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(HANDSHAKE.as_bytes())
        .unwrap(); // stdin dropped here -> EOF -> server shuts down

    let mut out = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    let status = child.wait().unwrap();

    assert!(status.success(), "server exited unsuccessfully: {status:?}");
    assert!(
        out.contains(r#""name":"plan_ping""#),
        "tools/list should advertise plan_ping:\n{out}"
    );
    assert!(
        out.contains(r#""text":"pong""#),
        "tools/call should return pong:\n{out}"
    );
}
