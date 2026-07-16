# M9a · Settings — the `"plan"` key + consumption — milestone note

**Status:** COMPLETE. `plan_ui` tests + clippy green; full `zed` build green. The **first upstream
touch beyond registration** (the settings key) — accepted, tracked in FORK_DIFF. No §13 here (the
settings-behavior + page visual check folds into M9b).

**Feature IDs:** F12.1 (personal keys), F12.2 (precedence), + the durable enable flag (M0 deferred).

## What works (built)
- **The `"plan"` settings key** — `PlanSettingsContent` + `PlanDefaultLens`/`PlanRevisionMode`
  enums in upstream `settings_content`, defaults in `default.json`, `pub plan` on `SettingsContent`;
  `crates/plan_ui/src/plan_settings.rs` maps it via `#[derive(RegisterSetting)] PlanSettings` +
  `from_settings` (mirrors `GitPanelSettings`). Auto-registers via inventory. **1 mapping test.**
- **Consumption (all 5 MVP keys are live — no dead keys):**
  - **`enabled`** — `plan_enabled(cx)` returns true on `"plan".enabled` **or** `ZED_PLAN`; gates the
    panel + pill (zed.rs) and `plan_ui::init`. (Toggling takes effect next launch — registration is
    once at startup.)
  - **`dock`** — the panel's `position()` reads it; `set_position()` persists via
    `update_settings_file` (right-click "move" now survives restarts).
  - **`auto_open`** — the panel's `starts_open()` returns it.
  - **`default_lens`** — `resolved_default_lens` honors it (`auto` follows status, else the chosen
    lens) on retarget.
  - **`revisions`** — `auto_apply` auto-applies a *staged* revision on arrival in `reload`;
    **amendments always stay staged** (F9.3) — guarded by the `kind: "amendment"` check.

## Decisions recorded
- **Enable = setting OR env** — `ZED_PLAN` stays for dev/CI; `"plan".enabled` is the durable path.
  Registration is startup-once (toggling needs a relaunch) — acceptable for MVP.
- **MVP keys only** — enabled/dock/auto_open/default_lens/revisions (each gates shipped behavior).
  v1-feature keys (peek-on-hover, rehearse, worktree, attention, thresholds) land with their
  features, not as dead no-ops now (open question 3).
- **`from_settings` unwraps** (matches the git-panel template) — relies on `default.json` having
  every default; all 5 are present + the `settings` crate's default-validation tests pass (31 green).

## Fork discipline (the M9 caveat)
This is the milestone that **deliberately breaks the near-zero fork diff**, along a well-trodden
path. Upstream files touched (all in FORK_DIFF): `settings_content.rs` (+key), `default.json`
(+defaults), `vscode_import.rs` (+`plan: None` for the exhaustive initializer), and the existing
`zed.rs` (the `plan_enabled(cx)` signature). The settings-page section (M9b) adds one more
(`settings_ui/page_data.rs`). `plan_ui/src/plan_settings.rs` is additive.

## Definition of done
- [x] `PlanSettingsContent` + `default.json` defaults + `PlanSettings`/`from_settings`, test-first.
- [x] `plan_ui` consumes enabled/dock/default_lens/auto_open/revisions; unset = current defaults.
- [x] Full `zed` build green; clippy clean; **FORK_DIFF updated** (3 new upstream files + zed.rs);
      `docs/milestones/M9a.md`.

## Next: M9b — settings-page section + Careful/Balanced/Fast presets (§13 lands there, covering the
M9a behaviors too). Then M9c — §12 hardening drills → MVP complete.
