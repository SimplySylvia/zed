//! `plan_server` — the Plan feature's MCP server (stdio). It is the agent's
//! interface to `.plans/<id>.plan.json`, reusing `plan_core` for the schema,
//! atomic store, and validation so there is a single source of truth (no
//! re-implementation, no drift). See docs/milestones/M2-plan.md.
//!
//! `plan.json` is the only IPC: tools mutate it through `plan_core::store`
//! (atomic temp+rename) and `plan_get` always reads fresh from disk.

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::router::tool::ToolRouter,
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};

#[derive(Clone)]
struct PlanServer {
    // Read by the `#[tool_handler]`-generated routing code.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl PlanServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Health check; confirms the server is reachable.
    #[tool(description = "Health check for the plan server")]
    async fn plan_ping(&self) -> String {
        "pong".to_string()
    }
}

#[tool_handler]
impl ServerHandler for PlanServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.instructions =
            Some("Plan feature MCP server — drives .plans/<id>.plan.json".to_string());
        info
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = PlanServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
