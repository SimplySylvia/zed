//! Typed `plan.json` schema (PRD Appendix A), `schema_version` handling, and
//! forward migration.
//!
//! Every object carries a `#[serde(flatten)] extra` bucket so fields written by
//! a newer schema version survive a load -> save round trip instead of being
//! dropped (§11 "unknown fields preserved"). Optional/absent-capable fields use
//! `#[serde(default)]` so partial or older documents still load. Closed
//! lifecycle vocabularies (`Status`, `TaskStatus`) are typed enums; smaller
//! open-ended codes (op, system, severity, guard kind, …) stay `String` for
//! forward-compatibility and are refined in later milestones as the UI needs them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Free-form additional fields preserved verbatim for forward-compatibility.
type Extra = BTreeMap<String, Value>;

/// Bring a raw `plan.json` value up to [`crate::SCHEMA_VERSION`] before typed
/// deserialization. A missing `schema_version` is treated as the current
/// version (lenient, for hand-written plans); a newer version is rejected so we
/// never silently mangle a document a future app wrote. Ordered migrations for
/// older versions are applied here as the schema evolves.
pub fn migrate(mut value: Value) -> anyhow::Result<Value> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("plan.json root must be a JSON object"))?;
    let version = match object.get("schema_version") {
        None => {
            object.insert("schema_version".into(), Value::from(crate::SCHEMA_VERSION));
            crate::SCHEMA_VERSION
        }
        Some(raw) => u32::try_from(
            raw.as_u64()
                .ok_or_else(|| anyhow::anyhow!("schema_version must be a non-negative integer"))?,
        )
        .map_err(|_| anyhow::anyhow!("schema_version is out of range"))?,
    };
    if version > crate::SCHEMA_VERSION {
        anyhow::bail!(
            "plan.json schema_version {version} is newer than supported {}; upgrade the app",
            crate::SCHEMA_VERSION
        );
    }
    // Future: apply ordered migrations for `version < SCHEMA_VERSION` here.
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub status: Status,
    pub rev: u32,
    pub thread: String,
    #[serde(default)]
    pub executor: Option<Executor>,
    #[serde(default)]
    pub tickets: Vec<Ticket>,
    pub spec: Spec,
    #[serde(default)]
    pub design: Design,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub pending_revision: Option<PendingRevision>,
    #[serde(default)]
    pub git: Option<Git>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Intake,
    Drafting,
    InReview,
    Revising,
    Approved,
    Executing,
    Paused,
    Gate,
    Amending,
    Done,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Executor {
    pub thread: String,
    #[serde(default)]
    pub taken_at: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ticket {
    pub source: String,
    pub key: String,
    #[serde(rename = "type", default)]
    pub ticket_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub ac: Vec<String>,
    #[serde(default)]
    pub fetched_at: Option<String>,
    #[serde(default)]
    pub drift: Option<Value>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spec {
    pub goal: String,
    #[serde(default)]
    pub scope: Scope,
    #[serde(default)]
    pub acceptance: Vec<Acceptance>,
    #[serde(default)]
    pub open_questions: Vec<OpenQuestion>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    #[serde(default)]
    pub r#in: Vec<String>,
    #[serde(default)]
    pub out: Vec<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Acceptance {
    pub id: String,
    #[serde(default)]
    pub when: Option<String>,
    #[serde(default)]
    pub shall: Option<String>,
    #[serde(default)]
    pub ticket_ac: Option<String>,
    #[serde(default)]
    pub tasks: Vec<String>,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub waived: Option<Value>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    #[serde(rename = "type", default)]
    pub evidence_type: Option<String>,
    #[serde(rename = "ref", default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub captured_at: Option<String>,
    #[serde(default)]
    pub stale: bool,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenQuestion {
    pub id: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub assumption: Option<String>,
    #[serde(default)]
    pub assumed_blocks: Vec<String>,
    #[serde(default)]
    pub answer: Option<Value>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Design {
    #[serde(default)]
    pub contracts: Vec<Contract>,
    #[serde(default)]
    pub previews: Vec<Preview>,
    #[serde(default)]
    pub decisions: Vec<Decision>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contract {
    pub id: String,
    #[serde(default)]
    pub rows: Vec<ContractRow>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractRow {
    #[serde(default)]
    pub r#in: Option<String>,
    #[serde(default)]
    pub out: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preview {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub attached_to: Option<String>,
    #[serde(default)]
    pub body: Value,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub gate: bool,
    #[serde(default)]
    pub ticket: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub files: Vec<FileChange>,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub commit_message: Option<String>,
    #[serde(default)]
    pub preview: Option<String>,
    #[serde(default)]
    pub acceptance: Vec<String>,
    #[serde(default)]
    pub status: TaskStatus,
    #[serde(default)]
    pub manual: bool,
    #[serde(default)]
    pub artifacts: Artifacts,
    #[serde(default)]
    pub timeline: Vec<TimelineEntry>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Done,
    Failed,
    Skipped,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    #[serde(default)]
    pub op: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub assumes: Option<String>,
    #[serde(default)]
    pub guard: Option<Guard>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guard {
    #[serde(rename = "type", default)]
    pub guard_type: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub response: Option<Value>,
    #[serde(default)]
    pub cleared_at: Option<String>,
    #[serde(default)]
    pub evidence_for: Vec<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Artifacts {
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub diffstat: Option<String>,
    #[serde(default)]
    pub tests: Option<Value>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEntry {
    #[serde(default)]
    pub at: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub anchor: Option<Anchor>,
    #[serde(default)]
    pub outdated: bool,
    #[serde(default)]
    pub suggestion: Option<Value>,
    #[serde(default)]
    pub alternatives: Option<Value>,
    #[serde(default)]
    pub thread: Vec<ThreadEntry>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub caused_changes: Vec<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    #[serde(default)]
    pub lens: Option<String>,
    #[serde(default)]
    pub block: Option<String>,
    #[serde(default)]
    pub range: Option<Vec<u32>>,
    #[serde(default)]
    pub quote: Option<String>,
    #[serde(default)]
    pub code_refs: Vec<CodeRef>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeRef {
    pub path: String,
    #[serde(default)]
    pub line: Option<u32>,
    #[serde(default)]
    pub quote: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadEntry {
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub rev: Option<u32>,
    #[serde(default)]
    pub cites: Vec<CodeRef>,
    #[serde(default)]
    pub pushback: Option<Value>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingRevision {
    #[serde(default)]
    pub rev: Option<u32>,
    #[serde(default)]
    pub hunks: Vec<Hunk>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hunk {
    pub id: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub old: Option<String>,
    #[serde(default)]
    pub new: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Git {
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub worktree: Option<String>,
    #[serde(default)]
    pub ahead: u32,
    #[serde(default)]
    pub behind: u32,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default)]
    pub commits: Vec<Commit>,
    #[serde(default)]
    pub pr: Option<Value>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Commit {
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub diffstat: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    #[serde(default)]
    pub rev: Option<u32>,
    #[serde(default)]
    pub by: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub at: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}
