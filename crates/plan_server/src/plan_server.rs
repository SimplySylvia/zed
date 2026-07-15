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

pub mod hooks;
pub mod tools;

/// The directory holding `<id>.plan.json` files. Defaults to `.plans` under the
/// server's working directory (the project root Claude Code runs in);
/// `PLAN_PLANS_DIR` overrides it (used by tests and non-standard layouts).
pub fn plans_dir() -> PathBuf {
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

#[derive(Debug, Deserialize, JsonSchema)]
struct SetStatusArgs {
    id: String,
    /// New lifecycle status (intake|drafting|in_review|…|done|abandoned).
    status: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddTaskArgs {
    id: String,
    /// New task id (unique within the plan).
    task_id: String,
    title: String,
    /// Optional system tag (backend|frontend|integration|testing).
    system: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct TaskUpdateArgs {
    id: String,
    task_id: String,
    /// New task status (pending|in_progress|done|failed|skipped|interrupted).
    status: Option<String>,
    /// Optional timeline note to append.
    detail: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AnswerQuestionArgs {
    id: String,
    question_id: String,
    answer: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateSectionArgs {
    id: String,
    /// Section path: "spec.goal" | "spec.scope.in" | "spec.scope.out" | "design.risks".
    section: String,
    /// New value (string for spec.goal; array of strings for the others).
    value: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListCommentsArgs {
    id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ReplyCommentArgs {
    id: String,
    comment_id: String,
    text: String,
    /// Optional agent action: revised | pushback | answered.
    action: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CommentRefArgs {
    id: String,
    comment_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PlanLaunchArgs {
    id: String,
    /// The executing thread's ACP session id; takes the executor lease (F11.3).
    thread: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct HunkArg {
    /// Block id this hunk rewrites, e.g. "t2.s1" (step) or "t3" (task title).
    target: Option<String>,
    /// The current text, for display/provenance in the change card.
    old: Option<String>,
    /// The proposed replacement text.
    new: Option<String>,
    /// Source comment id this change answers, e.g. "c1" (provenance, F3.5).
    from: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ProposeRevisionArgs {
    id: String,
    /// Proposed hunks; each rewrites one block. Staged as a pending diff — the
    /// user applies or rejects them in the UI.
    hunks: Vec<HunkArg>,
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

    /// Set the plan's lifecycle status.
    #[tool(description = "Set the plan's lifecycle status")]
    async fn plan_set_status(
        &self,
        Parameters(args): Parameters<SetStatusArgs>,
    ) -> Result<String, ErrorData> {
        let plan = tools::set_status(&plans_dir(), &args.id, &args.status).map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// Append a new pending task to the plan.
    #[tool(description = "Append a new pending task to the plan")]
    async fn plan_add_task(
        &self,
        Parameters(args): Parameters<AddTaskArgs>,
    ) -> Result<String, ErrorData> {
        let plan = tools::add_task(
            &plans_dir(),
            &args.id,
            &args.task_id,
            &args.title,
            args.system.as_deref(),
        )
        .map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// Update a task's status and/or append a timeline note.
    #[tool(description = "Update a task's status and/or append a timeline note")]
    async fn task_update(
        &self,
        Parameters(args): Parameters<TaskUpdateArgs>,
    ) -> Result<String, ErrorData> {
        let plan = tools::task_update(
            &plans_dir(),
            &args.id,
            &args.task_id,
            args.status.as_deref(),
            args.detail.as_deref(),
        )
        .map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// Record an answer to an open question.
    #[tool(description = "Record an answer to an open question")]
    async fn plan_answer_question(
        &self,
        Parameters(args): Parameters<AnswerQuestionArgs>,
    ) -> Result<String, ErrorData> {
        let plan = tools::answer_question(&plans_dir(), &args.id, &args.question_id, &args.answer)
            .map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// Update a named spec/design section.
    #[tool(description = "Update a named spec/design section")]
    async fn plan_update_section(
        &self,
        Parameters(args): Parameters<UpdateSectionArgs>,
    ) -> Result<String, ErrorData> {
        let plan = tools::update_section(&plans_dir(), &args.id, &args.section, args.value)
            .map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// List the plan's open/sent comments (with re-anchor status) for review.
    #[tool(description = "List open/sent review comments on a plan")]
    async fn plan_list_comments(
        &self,
        Parameters(args): Parameters<ListCommentsArgs>,
    ) -> Result<String, ErrorData> {
        let value = tools::list_comments(&plans_dir(), &args.id).map_err(to_error)?;
        serde_json::to_string_pretty(&value).map_err(to_error)
    }

    /// Reply to a review comment (optionally recording revised/pushback/answered).
    #[tool(description = "Reply to a review comment")]
    async fn plan_reply_comment(
        &self,
        Parameters(args): Parameters<ReplyCommentArgs>,
    ) -> Result<String, ErrorData> {
        let plan = tools::reply_comment(
            &plans_dir(),
            &args.id,
            &args.comment_id,
            &args.text,
            args.action.as_deref(),
        )
        .map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// Mark a review comment addressed.
    #[tool(description = "Mark a review comment addressed")]
    async fn plan_mark_addressed(
        &self,
        Parameters(args): Parameters<CommentRefArgs>,
    ) -> Result<String, ErrorData> {
        let plan = tools::mark_addressed(&plans_dir(), &args.id, &args.comment_id)
            .map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// Apply a suggestion's replacement to its anchored block.
    #[tool(description = "Apply a suggested edit to its anchored block")]
    async fn plan_apply_suggestion(
        &self,
        Parameters(args): Parameters<CommentRefArgs>,
    ) -> Result<String, ErrorData> {
        let plan = tools::apply_suggestion(&plans_dir(), &args.id, &args.comment_id)
            .map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// Stage a proposed plan revision as a pending diff (F9.3).
    #[tool(description = "Stage a proposed plan revision as a pending diff (user applies/rejects hunks)")]
    async fn plan_propose_revision(
        &self,
        Parameters(args): Parameters<ProposeRevisionArgs>,
    ) -> Result<String, ErrorData> {
        let hunks = args
            .hunks
            .into_iter()
            .map(|hunk| plan_core::rev::HunkSpec {
                target: hunk.target,
                old: hunk.old,
                new: hunk.new,
                from: hunk.from,
            })
            .collect();
        let plan = tools::propose_revision(&plans_dir(), &args.id, hunks).map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// Launch an approved plan (F4.1).
    #[tool(description = "Launch an approved plan (→ executing) and take the executor lease")]
    async fn plan_launch(
        &self,
        Parameters(args): Parameters<PlanLaunchArgs>,
    ) -> Result<String, ErrorData> {
        let plan = tools::launch(&plans_dir(), &args.id, &args.thread).map_err(to_error)?;
        serde_json::to_string_pretty(&plan).map_err(to_error)
    }

    /// Run policy lint and return the findings (F9.1).
    #[tool(description = "Run policy lint; flags findings as plan-lint comments and returns them")]
    async fn plan_lint(
        &self,
        Parameters(args): Parameters<PlanGetArgs>,
    ) -> Result<String, ErrorData> {
        let value = tools::lint(&plans_dir(), &args.id).map_err(to_error)?;
        serde_json::to_string_pretty(&value).map_err(to_error)
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
