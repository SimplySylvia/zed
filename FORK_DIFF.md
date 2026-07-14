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
| `Cargo.toml` (root) | Added `crates/plan_core` + `crates/plan_ui` to `members`; added `plan_core`/`plan_ui` to `[workspace.dependencies]` | M0 | Register the new crates in the workspace |
| `crates/zed/Cargo.toml` | Added `plan_ui.workspace = true` | M0 | `zed` depends on `plan_ui` to init + register it |
| `crates/zed/src/main.rs` | Added `plan_ui::init(cx);` | M0 | Initialize Plan UI (no-op unless `ZED_PLAN` is set) |
| `crates/zed/src/zed.rs` | Added a flag-gated `PlanPanel` registration inside `initialize_panels` | M0 | Add the dock panel when `ZED_PLAN` is set |

All four changes are inert unless the `ZED_PLAN` environment flag is set, so an unflagged
build behaves exactly like upstream.

## Additive (new) files — never a merge conflict

Upstream has no such paths, so these never conflict on merge. Listed for completeness.

- `crates/plan_core/**` — GPUI-free schema/IO/lint/anchor core (M1+).
- `crates/plan_ui/**` — GPUI surfaces: panel, tab, pill, review (M0 placeholder panel).
- `script/merge-upstream.sh` — weekly upstream sync helper (fetch + report only).
- `docs/PRD.md`, `docs/zed-notes.md`, `docs/milestones/**` — product + build docs.
- `FORK_DIFF.md` — this manifest.

## `pub` accessors added to upstream crates

None yet. If a future milestone needs an internal API made public, add the *minimal*
accessor, never restructure upstream code, and record it here with the milestone and
reason (PRD Part III hard rule). Study notes indicate the near-term surfaces
(`acp_thread` events, `AgentPanel` active-thread accessors, git repo state, theme tokens)
are **already public**, so no accessors are expected before the settings work in M9.
