# Zed internals — study notes for the Plan feature

Research pass executed against this fork (commit `3e76813825`, branch `main`) per PRD
Part II §3 (ten study questions) and the four riskiest spikes from Part II §7. Every
claim below is anchored to a file path; line numbers are from the fork at time of
writing and may drift with upstream merges. This is desk research only — no feature code
was written.

Legend: ✅ confirms a build-plan assumption · ⚠️ nuance/recalibration · ❌ breaks an
assumption.

---

## Study Q1 — The `Panel` trait (dock.rs) and existing panels

**Files:** `crates/workspace/src/dock.rs`, `crates/git_ui/src/git_panel.rs`,
`crates/terminal_view/src/terminal_panel.rs`.

`Panel` (`dock.rs:36`) has supertrait bounds `Focusable + EventEmitter<PanelEvent> +
Render + Sized` and **10 required methods** (no default body):

```rust
pub trait Panel: Focusable + EventEmitter<PanelEvent> + Render + Sized {
    fn persistent_name() -> &'static str;
    fn panel_key() -> &'static str;
    fn position(&self, window: &Window, cx: &App) -> DockPosition;
    fn position_is_valid(&self, position: DockPosition) -> bool;
    fn set_position(&mut self, position: DockPosition, window, cx);
    fn default_size(&self, window: &Window, cx: &App) -> Pixels;
    fn icon(&self, window, cx) -> Option<ui::IconName>;
    fn icon_tooltip(&self, window, cx) -> Option<&'static str>;
    fn toggle_action(&self) -> Box<dyn Action>;
    fn activation_priority(&self) -> u32;
    // ...~16 more provided (defaulted) methods: min_size, is_zoomed, set_zoomed,
    //    set_active, starts_open, pane, remote_id, enabled, hide_button_setting, flex…
}
```

- `PanelEvent` (`dock.rs:27`) = `{ ZoomIn, ZoomOut, Activate, Close }`. A non-zooming
  panel just needs an empty `impl EventEmitter<PanelEvent>`.
- `PanelHandle` (`dock.rs:98`) is the object-safe mirror; you **never** implement it —
  there's a blanket `impl<T: Panel> PanelHandle for Entity<T>` (`dock.rs:144`). It
  provides `move_to_next_position` (`dock.rs:127`), which powers right-click "move" by
  walking `[Left, Bottom, Right]` filtered by `position_is_valid`.
- **Right-click-to-move persists only if `set_position` writes to settings** and
  `position` reads them back. `Dock::add_panel` (`dock.rs:580`) installs an
  `observe_global_in::<SettingsStore>` that physically re-homes the panel when the
  setting changes (`dock.rs:589`). GitPanel/TerminalPanel both do this. A hard-coded
  `position` breaks move.
- GitPanel is the minimal reference (`git_panel.rs:7909`): reads `dock` from its
  settings, `position_is_valid` excludes `Bottom` (Plan must return `true` for `Bottom`),
  `set_position` writes settings, `toggle_action` → `Box::new(ToggleFocus)`,
  `activation_priority` = 3.

**Minimal "hello" panel** needs: the 10 methods, a `FocusHandle` for `Focusable`, empty
`EventEmitter<PanelEvent>`, `Render`, plus (not part of the trait) a `ToggleFocus`
action, a `register(workspace)` wiring the toggle, an async `load()` constructor, and the
`add_panel_when_ready` entry in `zed.rs`.

---

## Study Q2 — `Item` + `SerializableItem` (item.rs) and DB round-trip

**Files:** `crates/workspace/src/item.rs`, `crates/workspace/src/persistence.rs`,
`crates/workspace/src/workspace.rs`, `crates/terminal_view/src/terminal_view.rs`.

`Item` (`item.rs:170`) = `Focusable + EventEmitter<Self::Event> + Render + Sized`. The
**only** required members are `type Event` and `fn tab_content_text(&self, detail, cx) ->
SharedString` (`item.rs:186`). Everything else is defaulted: `tab_content` (`:177`,
renders a `Label` by default — override for the `Plan — <id>` title + status dot),
`tab_icon`, `tab_tooltip_content`, `to_item_events` (maps `Self::Event` → generic
`ItemEvent` so the workspace learns dirty/close/update), `buffer_kind` → `ItemBufferKind`.

> ⚠️ There is **no `is_singleton`** method. Singleton behaviour (one view, retargeted) is
> achieved with the `items_of_type::<Self>().next()` + `activate_item` pattern, *not*
> `ItemBufferKind::Singleton` (that governs buffer-backed dedup — e.g. a file already
> open — not view uniqueness). See spike (c).

`SerializableItem` (`item.rs:407`) requires all five: `serialized_item_kind()`,
`cleanup(workspace_id, alive_items, …)`, `deserialize(project, workspace, workspace_id,
item_id, …)`, `serialize(&mut self, workspace, item_id, closing, …)`,
`should_serialize(&self, event) -> bool`.

**Two-layer persistence:**
1. The **Workspace DB** (`persistence.rs:584`, table `items(workspace_id, pane_id,
   position, kind, item_id, active, preview)`) stores only *which item lives in which
   pane*, keyed by a `kind` string — not the item's payload.
2. **Each item crate keeps its own DB table** for its payload keyed by `(workspace_id,
   item_id)`. `TerminalView` is the canonical example (`terminal_view.rs:1842`):
   `serialized_item_kind()` = `"Terminal"`, `serialize` writes to `TerminalDb`,
   `deserialize` reads it back and rebuilds, `cleanup` deletes unloaded rows,
   `should_serialize` gates on a dirty flag.

Registration: `workspace::register_serializable_item::<I>(cx)` (`workspace.rs:1094`),
called from the **crate's own `init(cx)`** (`terminal_view.rs:110`), which stores a
descriptor in a global `SerializableItemRegistry`. Restore + GC happens in
`Workspace::load_workspace` (`workspace.rs:7262`), which calls `cleanup` for anything not
reloaded.

> ⚠️ `item_id`s are explicitly documented as **not stable across reloads**
> (`workspace.rs:7358`), so the Plan view must implement `cleanup` or its persisted
> thread-binding row gets garbage-collected.

---

## Study Q3 — Active-thread tracking (agent_panel) + acp_thread events

**Files:** `crates/agent_ui/src/agent_panel.rs`, `crates/acp_thread/src/acp_thread.rs`,
`crates/acp_tools/src/acp_tools.rs`.

**Active thread is already publicly observable — no fork patch needed.** `AgentPanel`
holds the active surface in a private `BaseView` enum (`agent_panel.rs:1115`), but exposes:

- `AgentPanel::active_agent_thread(&self, cx) -> Option<Entity<AcpThread>>`
  (`agent_panel.rs:4088`) — already `pub`.
- `active_thread_id`, `active_conversation_view` — also `pub`.
- `impl EventEmitter<AgentPanelEvent>` (`agent_panel.rs:4953`) emitting
  `AgentPanelEvent::{ActiveViewChanged, ActiveViewFocused, EntryChanged, TerminalClosed,
  ThreadInteracted}`. `ActiveViewChanged` fires on every thread switch.

Reach it from any crate via `workspace.panel::<AgentPanel>(cx)` — the pattern the
external `acp_tools` crate already uses (`acp_tools.rs:34`). There is no workspace
*global* for the active thread; go through the panel.

**acp_thread event enum** — `AcpThread` (`acp_thread.rs`, lib root `src/acp_thread.rs`)
`impl EventEmitter<AcpThreadEvent>` (`:2169`). Full enum (`:2144`):

```rust
pub enum AcpThreadEvent {
    StatusChanged, PromptUpdated, NewEntry, TitleUpdated, TokenUsageUpdated,
    EntryUpdated(usize), EntriesRemoved(Range<usize>),
    ToolAuthorizationRequested(acp::ToolCallId), ToolAuthorizationReceived(acp::ToolCallId),
    ElicitationRequested(..), ElicitationResponded(..), Retry(RetryStatus),
    SubagentSpawned(acp::SessionId), Stopped(acp::StopReason), Error, LoadError(..),
    PromptCapabilitiesUpdated, Refusal, AvailableCommandsUpdated(..),
    ModeUpdated(..), ConfigOptionsUpdated(..), WorkingDirectoriesUpdated,
}
```

See spike (a) for the tool-call granularity analysis.

> ⚠️ **acp_thread has its own `plan()` model** (`Plan`/`PlanEntry`/`PlanStats`,
> `:1942`) — the agent's live TODO list. `update_plan` (`:3567`) mutates it and only
> calls `cx.notify()` (no event). This is the ACP session's ephemeral plan, **distinct
> from our durable `plan.json`** — but the F6.4 "TodoWrite/plan updates reconciled to
> plan.json" bridge lands here. A surface that mirrors it must `cx.observe` (not
> `cx.subscribe`) and read `thread.read(cx).plan()`.

---

## Study Q4 — Registration wiring in zed.rs / main.rs

**Files:** `crates/zed/src/zed.rs`, `crates/zed/src/main.rs`, root `Cargo.toml`.

- **Panels** are added, not "registered": `initialize_panels` (`zed.rs:775`) does
  `let x = XPanel::load(handle, cx);` then an `add_panel_when_ready(x, …)` entry in the
  `futures::join!`. That's **~2 lines** per panel, but presupposes an async `load()` fn
  and a `register(workspace)` for the toggle action living in the crate's `init`.
- **Serializable items** are *not* registered in zed.rs at all —
  `register_serializable_item` is called from the crate's own `init` (convention). The
  only zed-side line is `plan_ui::init(cx);` in `main.rs:761`-style init block (or 0 new
  lines if folded into an existing crate's init).
- Workspace members are an **explicit list** in root `Cargo.toml` (`members = [...]`,
  line 3), not a glob — adding `plan_core`/`plan_ui` requires editing it.

> ❌ **"+2 lines in zed.rs (register panel + register item)" is conflated.** The Plan
> *tab* is an `Item` (center pane), the Plan *panel* is a dock `Panel` — different
> mechanisms. Item registration lives in the crate `init`, not zed.rs. Realistic M0
> upstream footprint for the hello **panel**: root `Cargo.toml` (+2 members),
> `crates/zed/Cargo.toml` (+1 dep), `main.rs` (+1 init), `zed.rs` (+2 panel load) ≈
> **6 lines across 4 files**. The *spirit* ("tiny, mostly registration") holds; the
> literal "2 lines" does not. FORK_DIFF.md must track all of them.

---

## Study Q5 — editor + multi_buffer (read-only mini-buffers, diffs, blocks)

**Files:** `crates/multi_buffer/src/multi_buffer.rs`, `crates/multi_buffer/src/path_key.rs`,
`crates/editor/src/editor.rs`, `crates/editor/src/display_map/block_map.rs`,
`crates/acp_thread/src/diff.rs`, `crates/agent_ui/src/entry_view_state.rs`.

- Read-only knob: `Capability::{ReadWrite, Read, ReadOnly}` (`language/src/buffer.rs:79`).
- MultiBuffer constructors: `new(capability)` (`:1205`), `without_headers(capability)`
  (`:1216`, used by diff views), `singleton(buffer, cx)` (`:1226`).
- ⚠️ **The excerpt API is path-key based, NOT `push_excerpts`.** Use
  `set_excerpts_for_path(PathKey, buffer, ranges, context_line_count, cx)`
  (`path_key.rs:83`), `set_excerpts_for_buffer` (`:68`), or `singleton`. `ExcerptRange {
  context, primary }` (`multi_buffer.rs:842`). If the build plan says `push_excerpts`,
  update it.
- Diffs: `MultiBuffer::add_diff(Entity<BufferDiff>, cx)` (`:2205`) +
  `set_all_diff_hunks_expanded` (`:2256`). `BufferDiff` built from base+new text
  (`diff.rs:381`).
- Read-only editor: field `read_only` (`editor.rs:1039`), `set_read_only(true)`
  (`:3201`); `read_only()` (`:3197`) is true if *either* the flag or the multibuffer
  capability is read-only.
- Custom blocks (for GPUI chrome inside a buffer): `insert_blocks(BlockProperties…)`
  (`editor.rs:8382`); `RenderBlock = Arc<dyn Fn(&mut BlockContext) -> AnyElement>`
  (`block_map.rs:159`); `BlockPlacement::{Above, Below, Near, Replace}` (`:163`).

**Ready-made reusable factory:** `create_editor_diff` (`entry_view_state.rs:655`) builds
a read-only diff editor (gutter off, diagnostics off, scrollbar off, `set_read_only(true)`,
`set_expand_all_diff_hunks`). `acp_thread/src/diff.rs:25` builds the read-only diff
multibuffer. Copy these almost verbatim for staged-revision (F9.3) and preview (F1.2b)
rendering.

---

## Study Q6 — theme tokens

**Files:** `crates/theme/src/styles/colors.rs`, `crates/theme/src/styles/status.rs`,
`crates/theme/src/theme.rs`.

Access at render: `cx.theme().colors().<field>` and `cx.theme().status().<field>`
(`ActiveTheme` trait, `theme.rs:144`; `Context<T>` derefs to `App`).

**All 19 named design roles + version-control colors exist — zero MISSING.** Selected
mapping (see full table in agent research; summarized):

| Design role | Field | Struct |
|---|---|---|
| text / text_muted / text_placeholder / text_accent | `text`/`text_muted`/`text_placeholder`/`text_accent` | ThemeColors |
| element_hover / element_active / element_selected | same names | ThemeColors |
| border / border_variant | `border`/`border_variant` | ThemeColors |
| panel / editor / tab_bar / tab_active / title_bar / status_bar background | `*_background` | ThemeColors |
| created / modified / deleted / info (+ `_background`/`_border`) | same names | StatusColors |
| version_control added/deleted/modified/… | `version_control_*` | ThemeColors |

> ⚠️ **Mapping decision to record (per Part III hard rule):** `created`/`modified`/
> `deleted` have two valid sources — `StatusColors` (status-text semantics) and
> `ThemeColors::version_control_*` (VCS UI). For diff/plan chrome prefer
> `version_control_*` and the editor's own `editor_diff_hunk_added_*` /
> `editor_diff_hunk_deleted_*` row highlights (`colors.rs:244`); use `StatusColors` for
> lifecycle/status text (dots, pills).

---

## Study Q7 — git + git_ui (branch strip, commit rail, hunk reuse)

**Files:** `crates/git/src/repository.rs`, `crates/git/src/status.rs`,
`crates/project/src/git_store.rs`, `crates/git_ui/src/diff_multibuffer.rs`,
`crates/git_ui/src/project_diff.rs`.

> Repository state as **observable entities** lives in `project`, not `git`.

- `RepositorySnapshot` (`git_store.rs:401`): `branch: Option<Branch>`, `branch_list`,
  `head_commit: Option<CommitDetails>`, `statuses_by_path` (dirty), `merge`,
  `remote_*_url`, `linked_worktrees`. `Repository` entity (`:476`) `Deref`s to it.
- Branch/ahead-behind: `Branch` (`repository.rs:231`), `Branch::tracking_status()` →
  `UpstreamTrackingStatus { ahead: u32, behind: u32 }` (`:503`) — exactly the branch
  strip's `↑n ↓n`.
- Commits + diffstat: `CommitSummary` (`:509`), `CommitDetails` (`:519`, `short_sha()`),
  `GitRepository::diff_stat(&[RepoPath]) -> GitDiffStat` (`:1106`), `load_commit(sha) ->
  CommitDiff` (`:985`); commit-graph data via `InitialGraphCommitData` (`:110`) →
  `GraphDataResponse` — the commit-rail data source.
- **Observable:** `GitStore`/`Repository` are `EventEmitter`s (`git_store.rs:603`):
  `GitStoreEvent::{ActiveRepositoryChanged, RepositoryUpdated, …}`,
  `RepositoryEvent::{StatusesChanged, HeadChanged, BranchListChanged, …}`. Subscribe and
  re-render.
- **Hunk apply/reject is reusable.** Grouping is a multibuffer (excerpt per file); the
  actual stage/unstage/restore logic is in `editor` and wrapped by git_ui:
  `stage_or_unstage_selected_hunks` → `editor.stage_or_unstage_diff_hunks`
  (`diff_multibuffer.rs:345`), `restore_selected_hunks` → `editor.apply_restore`
  (`:368`). Per-hunk action gating uses pluggable `DiffHunkDelegate`s (e.g.
  `RestoreOnlyDiffHunkDelegate`) — exactly what staged-revision "reject" needs. ✅

---

## Study Q8 — settings registration + settings-page UI

**Files:** `crates/settings/src/settings_store.rs`,
`crates/settings_content/src/settings_content.rs`,
`crates/settings_macros/src/settings_macros.rs`,
`crates/settings_ui/src/page_data.rs`, `crates/settings_ui/src/settings_ui.rs`,
`assets/settings/default.json`.

The settings system was refactored to a **typed content model** (not per-setting free-form
JSON). Adding a `"plan"` key requires **three coordinated edits across upstream crates**:

1. `crates/settings_content/src/settings_content.rs`: add `pub plan:
   Option<PlanSettingsContent>` to `SettingsContent` (`:143`) + define
   `PlanSettingsContent` (all fields `Option<T>`), mirroring `GitPanelSettingsContent`
   (`:671`).
2. `assets/settings/default.json`: add defaults (every field must have one — `from_settings`
   is expected to panic on a missing default, `settings_store.rs:70`).
3. Consuming crate: `#[derive(RegisterSetting)]` + `impl Settings for PlanSettings { fn
   from_settings(content) -> Self { … } }` (`git_panel_settings.rs:60` template).
   `#[derive(RegisterSetting)]` auto-registers via `inventory::submit!` — no manual
   `register` call.

**Settings-page UI (F12.2c) does exist and is structured** — crate `settings_ui`.
`settings_data(cx) -> Vec<SettingsPage>` (`page_data.rs:65`) returns a **hardcoded Vec**.
`SettingsPage { title, items: [SettingsPageItem] }` (`settings_ui.rs:1051`);
`SettingsPageItem::{SectionHeader, SettingItem, SubPageLink, DynamicItem, ActionLink}`
(`:1057`). A "Plan" section = push a `SectionHeader("Plan")` + `SettingItem`s (each with
`pick`/`write` closures into `SettingsContent.plan` and a `json_path`) into an existing
page fn, or add a new `plan_page()` to the `settings_data` vec.

> ⚠️/❌ **F12.2c fork-discipline recalibration.** The settings page is *not* a plugin
> registry — pages are a hardcoded `Vec` in upstream `page_data.rs`, and each field must
> also be threaded through upstream `settings_content` + `default.json`. So the "Plan"
> settings section (M9) **touches ~3 upstream files and is not purely additive** — the
> opposite of the "isolated crate + registration lines" fork thesis. It's a well-trodden,
> low-risk path (dozens of examples) but must be tracked in FORK_DIFF and its
> conflict-surface accepted. Interim fallback: raw-JSON settings work today (there's an
> `unimplemented()` edit-in-JSON escape hatch, `settings_ui.rs:164`), so the structured
> page can be deferred without blocking functionality.

---

## Study Q9 — context server / MCP wiring

**Files:** `crates/agent_servers/src/acp.rs`, `crates/project/src/context_server_store.rs`,
`crates/context_server/src/context_server.rs`,
`crates/context_server/src/transport/stdio_transport.rs`,
`crates/settings_content/src/project.rs`, `docs/src/ai/mcp.md`,
`docs/src/ai/external-agents.md`.

- **Config shape** (`settings.json → context_servers`, keyed by id):
  `ContextServerSettingsContent` (`project.rs:392`, `#[serde(untagged)]`) with
  `Stdio { enabled, remote, #[flatten] command: ContextServerCommand }`, `Http`,
  `Extension`. `ContextServerCommand { path (JSON key "command"), args, env, timeout }`
  (`:479`).
- **Binary path resolution:** the `command` string runs through the **user's system
  shell** (`ShellBuilder`/`Shell::System`, `stdio_transport.rs:27`) — an absolute path is
  used verbatim; a bare name resolves via **PATH**. There is **no Zed-managed
  download/registry** for plain `Stdio` servers (only the `Extension` variant resolves a
  binary via `ContextServerDescriptorRegistry`).

See spike (d) for the critical ACP-forwarding finding and the registration
recommendation.

---

## Study Q10 — status bar (`StatusItemView`) — the pill

**Files:** `crates/workspace/src/status_bar.rs`, `crates/diagnostics/src/items.rs`,
`crates/zed/src/zed.rs`.

`StatusItemView` (`status_bar.rs:42`) = `Render` + **2 methods**:

```rust
pub trait StatusItemView: Render {
    fn set_active_pane_item(&mut self, active: Option<&dyn ItemHandle>, window, cx);
    fn hide_setting(&self, cx: &App) -> Option<HideStatusItem>;
}
```

- `set_active_pane_item` fires on active-pane-item change (`status_bar.rs:449`) — the hook
  for "follow the active editor". (It does **not** track an agent thread; thread-following
  is separate logic.)
- Registered via `status_bar().update(cx, |bar, cx| bar.add_left_item(view, …))` /
  `add_right_item` (`zed.rs:634`, methods at `status_bar.rs:356`). Returning
  `Some(HideStatusItem)` from `hide_setting` gives the right-click "Hide Button" menu for
  free.
- Example: `DiagnosticIndicator` (`diagnostics/src/items.rs:29` render, `:246`
  `StatusItemView`). The pill body is whatever `Render` returns.

✅ The pill-as-`StatusItemView` assumption holds.

---

# Spikes (Part II §7) — desk research

### (a) Does acp_thread expose tool-call-granular events for the activity feed?

**YES — this refutes the §7 risk "acp_thread events too opaque for activity feed."**

Tool activity flows through the entry list, `entries(): Vec<AgentThreadEntry>` where
`AgentThreadEntry::{UserMessage, AssistantMessage, ToolCall, Elicitation, CompletedPlan,
ContextCompaction}` (`acp_thread.rs:389`). The session-update handler
(`handle_session_update`, `:2538`) routes `acp::SessionUpdate::ToolCall` →
`upsert_tool_call` (new tool → `push_entry` → emits **`NewEntry`**, `:2989`) and
`ToolCallUpdate` → `update_tool_call` (status/diff/output change → emits
**`EntryUpdated(ix)`**, `:3162`).

The `ToolCall` entry struct (`:851`) carries everything a feed needs: `id`, `kind:
acp::ToolKind` (edit/execute/read/…), `status: ToolCallStatus` (`Pending`,
`WaitingForConfirmation`, `InProgress`, `Completed`, `Failed`, `Rejected`, `Canceled`,
`:1253`), `content`, `locations` (files touched), `raw_input`/`raw_output`, `tool_name`.
Permission prompts get dedicated `ToolAuthorizationRequested/Received(ToolCallId)`.

**Only ergonomics cost:** `NewEntry`/`EntryUpdated` are index-only — the subscriber reads
`thread.read(cx).entries()[ix]` to classify the change. Not opaque, just index-indirected.
**No pub accessor addition required.**

### (b) What does `Panel` require vs. what git_ui/terminal_view implement?

Covered in Q1. Delta a `plan_panel.rs` must supply: the 10 required methods (most 1-line),
`Focusable` (hold a `FocusHandle`), empty `EventEmitter<PanelEvent>`, `Render`. Plus
non-trait scaffolding both real panels have: a `ToggleFocus` action, a `register(workspace)`
wiring it, an async `load()`, the `add_panel_when_ready` entry in `zed.rs`, and (for
persistent right-click-move) a settings-backed `position`/`set_position`. ✅ Achievable;
⚠️ "default bottom + right-click move" needs settings backing, and `position_is_valid` must
allow `Bottom`.

### (c) Can one `Item` + `SerializableItem` follow the active thread cleanly?

**YES — feasible and well-precedented.**

- **Singleton retarget:** `ProjectDiff` (`git_ui/src/project_diff.rs:92`) uses
  `workspace.items_of_type::<Self>().next()` → `activate_item` to reuse the one instance
  instead of opening a second tab. Plan uses the same shape.
- **Follow the thread:** on `added_to_workspace`, grab `workspace.panel::<AgentPanel>(cx)`,
  `cx.subscribe` for `AgentPanelEvent::ActiveViewChanged`, then re-read
  `active_agent_thread(cx)` / `active_thread_id(cx)` and re-render. Store the
  `Subscription`.
- **Persist/restore the binding:** the AgentPanel already does exactly this —
  `SerializedActiveThread { session_id, thread_id, agent_type, title, work_dirs }`
  (`agent_panel.rs:359`), kvp keyed by `workspace_id` (`:299`). For the Plan
  `SerializableItem`, write the target `ThreadId` into a small per-crate `PlanDb` table
  keyed by `(workspace_id, item_id)`, read back in `deserialize` (mirror
  `TerminalView::deserialize`). `ThreadId` is `Serialize`/`Deserialize`.

> ⚠️ Two caveats: implement `cleanup` (item IDs unstable → row GC), and `deserialize` must
> tolerate a dangling `ThreadId` (fall back to the AgentPanel's current active thread).

### (d) How are context_servers configured/spawned; what determines the binary path?

**Critical architecture finding:** Zed does **not** run the plan-server's MCP process for
the agent. Zed reads its own `context_servers` and **forwards them to the ACP agent
(Claude Code) over the protocol**; the agent spawns the MCP process on its side.

- `mcp_servers_for_project(project, cx) -> Vec<acp::McpServer>` (`acp.rs:4279`) reads
  `context_server_store` and maps each configured server to `acp::McpServer::Stdio/Http`.
- That list is attached to every ACP session request — `new_session` (`acp.rs:1593`),
  `load_session`, `resume_session` — via `NewSessionRequest::new(cwd).mcp_servers(…)`.
- Docs confirm: *"MCP servers configured in Zed are forwarded to External Agents via the
  Agent Client Protocol"* (`mcp.md:163`), and agents may **also** read their own native
  MCP config (`external-agents.md:104`).

**Two valid registration paths (not either/or):**
- **Path A — Zed `context_servers`** (recommended): one config home in Zed
  `settings.json`, agent-agnostic, visible in Zed's MCP UI, forwarded over ACP.
- **Path B — Claude Code native `.mcp.json` / `claude mcp add`:** agent connects
  directly, invisible to Zed, Claude-Code-only.

> ⚠️ **Subtle correction to the PRD wording.** "Registered as a context server" does *not*
> mean Zed launches the MCP process — Zed forwards the *launch spec*, and **Claude Code
> spawns it**. Therefore the plan-server binary/PATH and working directory must resolve in
> the **agent subprocess's** environment, not Zed's. For dev, point `command`/`args` at an
> absolute build path (`node /abs/.../dist/index.js`). This directly affects the M2
> hook/spawn spikes.

---

# Design assumptions: confirmed / broken

### Confirmed (build as planned)
- **plan.json-as-IPC decoupling is sound.** Zed reads/watches the file; git repo state,
  acp_thread events, and theme are all readable without patching upstream internals.
- **Activity feed is viable at tool-call granularity** (spike a) — §7's opaqueness risk is
  refuted. Subscribe to `AcpThreadEvent::{NewEntry, EntryUpdated, ToolAuthorization*,
  StatusChanged, Stopped, Error, Refusal}`.
- **Thread-following needs no fork patch** (spike c / Q3) — `AgentPanel::active_agent_thread`,
  `active_thread_id`, and `AgentPanelEvent::ActiveViewChanged` are already `pub`. "Subscribe
  to agent_ui, never patch it" holds.
- **Singleton retarget + workspace-keyed persistence** (spike c / Q2) — precedented by
  `ProjectDiff` and `AgentPanel`/`TerminalView`.
- **Read-only mini-buffers + diff rendering come free** (Q5) — `create_editor_diff` /
  `acp_thread::diff` are copy-ready factories. Deferring them to GPUI chrome (M3–M5) is a
  valid, additive interim per §6.
- **Every mockup color role maps to a real theme field** (Q6) — zero MISSING; the only
  work is recording the `StatusColors` vs `version_control_*` choice for done/warn/fail.
- **Git state + hunk reuse** (Q7) — branch/ahead-behind/diffstat/commit-graph all readable
  and observable; git_ui's stage/restore hunk primitives + `DiffHunkDelegate` gating are
  reusable for staged revisions.
- **Pill = `StatusItemView`** (Q10) and **plan-server = stdio context server** (Q9/d) both
  hold.

### Broken / recalibrated (adjust the build plan)
- ❌ **"~2 registration lines in zed.rs" is optimistic and conflated** (Q4/b). Tab (`Item`)
  and panel (`Panel`) are different mechanisms; item registration lives in the crate
  `init`, not zed.rs. Realistic M0 hello-panel upstream footprint ≈ 6 lines across 4 files
  (root `Cargo.toml`, `crates/zed/Cargo.toml`, `main.rs`, `zed.rs`). Track all in
  FORK_DIFF; treat "1–2 lines" as an ideal, not a measured fact.
- ⚠️/❌ **The settings page (F12.2c) is not purely additive** (Q8). It touches ~3 upstream
  files (`settings_content`, `assets/settings/default.json`, `settings_ui/page_data.rs`),
  and pages are a hardcoded `Vec`, not a registry. Accept the conflict surface for M9 or
  defer the structured page (raw-JSON settings work today).
- ⚠️ **"Registered as a context server" ≠ Zed spawns it** (spike d). Zed forwards the spec;
  Claude Code spawns the process. Binary/PATH/cwd must be valid in the *agent's*
  environment. Recommendation: Path A (Zed `context_servers`).
- ⚠️ **Excerpt API is `set_excerpts_for_path`, not `push_excerpts`** (Q5) — a naming drift
  in the plan; update references.
- ⚠️ **acp_thread's `plan()` is the agent's ephemeral TODO list, distinct from our durable
  `plan.json`** (Q3). It updates via `cx.notify()` (no event); the F6.4 reconciliation
  bridge lives at this seam. An event-driven mirror would be the *one* place a minimal
  `PlanUpdated` emit patch might be justified — record in FORK_DIFF if taken.
- ⚠️ **Feature flags are server/staff-driven** (`crates/feature_flags`, `has_flag`/
  `observe_flag` via `FeatureFlagStore`) — awkward for a self-controlled personal fork. M0
  should gate the hello panel with a self-owned mechanism (env var or, later, the `"plan"`
  setting), not Zed's cloud feature flags. See M0 plan open questions.
