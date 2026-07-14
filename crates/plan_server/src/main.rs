//! Thin binary entry point; all logic lives in the `plan_server` library so it
//! stays unit-testable.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    plan_server::run().await
}
