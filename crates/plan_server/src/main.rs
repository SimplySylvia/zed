//! Thin binary entry point. `plan_server` serves the MCP protocol over stdio;
//! `plan_server hook <event>` runs a Claude Code hook (reads the hook payload on
//! stdin, emits the decision on stdout). All logic lives in the library so it
//! stays unit-testable.

use std::io::Read;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("hook") {
        let event = args.next().unwrap_or_default();
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer).ok();
        let input: serde_json::Value =
            serde_json::from_str(&buffer).unwrap_or(serde_json::Value::Null);
        let (output, code) = plan_server::hooks::run(&event, &input, &plan_server::plans_dir());
        if !output.is_empty() {
            println!("{output}");
        }
        std::process::exit(code);
    }
    plan_server::run().await
}
