# Fork diff manifest

This fork adds the **Plan** feature (see `docs/PRD.md`). Fork discipline (PRD Part II §2,
§5, Part III):

- **Everything is additive crates.** New behaviour lives in `crates/plan_core`,
  `crates/plan_ui`, and out-of-tree components (`plan-server/`, `plan-agent/`).
- **Upstream files are touched as little as possible** — only registration/wiring lines.
  Every such touch is listed below.
- **Nothing outside `plan_ui`/`plan_core` may depend on them.**
- On a weekly `upstream` merge, a conflict in a file **not** on this list means a leak —
  fix the leak, don't route around it. `script/merge-upstream.sh` cross-checks against
  this manifest.

## Upstream files modified (the conflict surface)

| File | Change | Milestone | Why |
|---|---|---|---|
| `Cargo.toml` (root) | Added `crates/plan_core`/`plan_ui`/`plan_server` to `members`; added `plan_core`/`plan_ui`/`plan_server` + `rmcp` to `[workspace.dependencies]` | M0, M2 | Register the new crates + the rmcp dep in the workspace |
| `crates/zed/Cargo.toml` | Added `plan_ui.workspace = true` | M0 | `zed` depends on `plan_ui` to init + register it |
| `crates/zed/src/main.rs` | Added `plan_ui::init(cx);` | M0 | Initialize Plan UI (no-op unless `ZED_PLAN` is set) |
| `crates/zed/src/zed.rs` | Flag-gated `PlanPanel` registration in `initialize_panels`; flag-gated `PlanPill` in the status bar; `plan_enabled(cx)` now reads the `"plan".enabled` setting too | M0, M4, M9 | Add the dock panel + status-bar pill when enabled |
| `crates/settings_content/src/settings_content.rs` | Added `PlanSettingsContent` + `PlanDefaultLens`/`PlanRevisionMode` enums; added `pub plan: Option<PlanSettingsContent>` to `SettingsContent` | M9 | Register the `"plan"` settings key in Zed's typed settings model (F12.1) |
| `assets/settings/default.json` | Added the `"plan"` defaults block | M9 | Every settings field needs a default (`from_settings` panics otherwise) |
| `crates/settings/src/vscode_import.rs` | Added `plan: None` to the exhaustive `SettingsContent` initializer | M9 | Required by the new `SettingsContent.plan` field (no VS Code equivalent) |

The M0/M4 changes are inert unless the feature is enabled (`ZED_PLAN` env **or** `"plan".enabled`).

**M9 fork-discipline note:** the settings key is the first upstream touch beyond registration/wiring
(zed-notes study #8) — a well-trodden path (mirrors `GitPanelSettingsContent`) but not purely
additive. The `settings_content` + `default.json` + `vscode_import` touches are the accepted
conflict surface; the settings-page section (M9b) adds one more (`settings_ui/page_data.rs`). Keep
these edits to the exact fields/rows the feature needs.

## Additive (new) files — never a merge conflict

Upstream has no such paths, so these never conflict on merge. Listed for completeness.

- `crates/plan_core/**` — GPUI-free schema/IO/lint/anchor core (M1+).
- `crates/plan_ui/**` — GPUI surfaces: tab, dock panel, status pill (M3/M4).
- `crates/plan_server/**` — the Rust MCP server (rmcp stdio) + hooks (M2).
- `plan-agent/**` — the planning skill, hooks wiring, Claude Code settings fragment (M2).
- `script/merge-upstream.sh` — weekly upstream sync helper (fetch + report only).
- `docs/PRD.md`, `docs/zed-notes.md`, `docs/milestones/**` — product + build docs.
- `FORK_DIFF.md` — this manifest.

## `pub` accessors added to upstream crates

None yet. If a future milestone needs an internal API made public, add the *minimal*
accessor, never restructure upstream code, and record it here with the milestone and
reason (PRD Part III hard rule). Study notes indicate the near-term surfaces
(`acp_thread` events, `AgentPanel` active-thread accessors, git repo state, theme tokens)
are **already public**, so no accessors are expected before the settings work in M9.
