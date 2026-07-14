//! `plan_server` — the Plan feature's MCP server (stdio). It is the agent's
//! interface to `.plans/<id>.plan.json`, reusing `plan_core` for the schema,
//! atomic store, and validation so there is a single source of truth (no
//! re-implementation, no drift). See docs/milestones/M2-plan.md.
//!
//! `plan.json` is the only IPC: tools mutate it through `plan_core::store`
//! (atomic temp+rename) and `plan_get` always reads fresh from disk. Tool logic
//! lives in [`tools`] as plain functions over a plans directory so it unit-tests
//! without rmcp or async; the `#[tool]` methods here are thin wrappers.

use std::path::PathBuf;

use rmcp::{
    ErrorData, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::Deserialize;

pub mod tools;

/// The directory holding `<id>.plan.json` files. Defaults to `.plans` under the
/// server's working directory (the project root Claude Code runs in);
/// `PLAN_PLANS_DIR` overrides it (used by tests and non-standard layouts).
fn plans_dir() -> PathBuf {
    std::env::var_os("PLAN_PLANS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .join(".plans")
        })
}

fn to_error(error: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(error.to_string(), None)
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PlanGetArgs {
    /// The plan id, e.g. "LED-212".
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PlanCreateArgs {
    /// Stable plan id; also the `<id>.plan.json` filename.
    id: String,
    /// One-line human title.
    title: String,
    /// The plan's goal (Spec lens).
    goal: String,
    /// The owning thread id (F1.0: one thread, one plan).
    thread: String,
}

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

    /// Read a plan fresh from disk by id, returning it as JSON.
    #[tool(description = "Read a plan fresh from disk by id")]
    async fn plan_get(&self, Parameters(args): Parameters<PlanGetArgs>) -> Result<String, ErrorData> {
        let plan = tools::get(&plans_dir(), &args.id).map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// Create a new draft plan for a thread and return it as JSON.
    #[tool(description = "Create a new draft plan for a thread")]
    async fn plan_create(
        &self,
        Parameters(args): Parameters<PlanCreateArgs>,
    ) -> Result<String, ErrorData> {
        let plan = tools::create(&plans_dir(), &args.id, &args.title, &args.goal, &args.thread)
            .map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
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

/// Serve the MCP protocol over stdio until the client disconnects.
pub async fn run() -> anyhow::Result<()> {
    let service = PlanServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
