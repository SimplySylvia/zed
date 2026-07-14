# M0 · Fork & rhythm — implementation plan

**Status:** awaiting sign-off (not started).
**Feature IDs:** none (infrastructure milestone).
**Goal (PRD Part II §4):** Fork, build, run. Set up `upstream` remote + a weekly
`merge-upstream.sh`. Create empty `plan_core`/`plan_ui` crates, register a "hello" panel
behind a flag. **Proves: the build loop + the fork-diff discipline.**

Commit convention: one commit per task, imperative mood, prefixed `[PLAN-M0]`.
Never commit with a failing build.

---

## Research-driven context (from `docs/zed-notes.md`)

- The `upstream` remote **already exists** (`https://github.com/zed-industries/zed.git`).
  M0 only needs to verify it and add the merge script.
- Workspace `members` is an **explicit list** in root `Cargo.toml` — new crates must be
  added there.
- Per CLAUDE.md: crates use `[lib] path = "…"` with a descriptive filename (e.g.
  `plan_core.rs`), never `lib.rs` or `mod.rs`.
- The "hello panel" is a dock `Panel` (`crates/workspace/src/dock.rs:36`). Minimal recipe
  and the full trait are in `zed-notes.md` Q1/(b). GitPanel (`git_panel.rs:7909`) is the
  copy template.
- ❌ **"2-line fork diff" recalibration:** registering a panel realistically touches ~6
  lines across 4 upstream files (root `Cargo.toml`, `crates/zed/Cargo.toml`, `main.rs`,
  `zed.rs`). This is expected and fine — the point is that everything *else* is additive
  crates. FORK_DIFF.md tracks the exact list.
- ⚠️ **Feature flag:** Zed's `feature_flags` crate is server/staff-driven and unsuitable
  for a self-controlled personal fork. M0 gates the panel with a self-owned mechanism.

---

## Tasks

### T1 — Verify the build & run loop
`[PLAN-M0]: verify baseline fork build and launch`
- Confirm toolchain resolves (`rust-toolchain.toml` pins 1.95.0 via rustup).
- `cargo build -p zed` (debug) completes from a clean tree; record wall-clock time.
- Launch the built binary; confirm the editor opens.
- Run `./script/clippy` once to capture the baseline lint state.
- **Files:** none (verification only). Notes captured in `docs/milestones/M0.md`.
- **Acceptance:** clean `cargo build -p zed` succeeds; app launches; clippy baseline
  recorded. Build time noted so future weekly builds have a reference.

### T2 — Document `upstream` remote + add `script/merge-upstream.sh`
`[PLAN-M0]: add weekly merge-upstream script`
- Verify `git remote -v` shows `upstream` → zed-industries/zed (it does).
- Write `script/merge-upstream.sh` implementing the §5 rhythm: `git fetch upstream`,
  fast-forward/merge `upstream/main` into the local mirror branch, then report any
  conflicts **outside** the FORK_DIFF.md manifest as "a leak to fix." Script is
  idempotent, prints a summary, and does **not** force-push or rewrite history.
- **Files:** `script/merge-upstream.sh` (new, executable).
- **Acceptance:** `script/merge-upstream.sh` runs clean on an up-to-date tree and prints a
  no-op summary; documented usage in the M0 note. (Does not perform the merge unattended
  in this milestone — dry-run / summary first.)

### T3 — Create empty `plan_core` crate (no GPUI)
`[PLAN-M0]: scaffold plan_core crate`
- `crates/plan_core/Cargo.toml` with `[lib] path = "src/plan_core.rs"`; minimal deps
  (`anyhow`, `serde`, `serde_json` — schema lands in M1, so keep it near-empty).
- `crates/plan_core/src/plan_core.rs` with a placeholder (a doc comment + maybe a
  `pub const SCHEMA_VERSION: u32 = 1;`). No GPUI dependency.
- Add `"crates/plan_core"` to root `Cargo.toml` `members`.
- **Files:** `crates/plan_core/Cargo.toml`, `crates/plan_core/src/plan_core.rs`, root
  `Cargo.toml`.
- **Acceptance:** `cargo build -p plan_core` succeeds; `cargo tree -p plan_core` shows no
  `gpui` dependency; nothing depends on `plan_core` yet.

### T4 — Create `plan_ui` crate with a flag-gated hello `Panel`
`[PLAN-M0]: scaffold plan_ui crate with hello panel`
- `crates/plan_ui/Cargo.toml` with `[lib] path = "src/plan_ui.rs"`; deps `gpui`,
  `workspace`, `ui`, `theme` (the allowed direction per §2: plan_ui may depend on
  workspace/agent_ui/editor/theme; nothing depends on plan_ui).
- `crates/plan_ui/src/plan_ui.rs`:
  - `PlanPanel` struct holding a `FocusHandle`; impls `Focusable`,
    `EventEmitter<PanelEvent>` (empty), `Render` (a themed `div().child("Plan")` using
    `cx.theme().colors()` — **no hardcoded colors**), and the 10 required `Panel` methods
    (copy GitPanel's shape; `position` → `DockPosition::Bottom`, `position_is_valid` →
    `true`, `activation_priority` → an unused value).
  - `actions!(plan_panel, [ToggleFocus])` and an async `PlanPanel::load(workspace, cx)`.
  - `pub fn init(cx: &mut App)` that registers the toggle action and gates panel
    availability behind the flag (see T5 / open question Q2).
- Add `"crates/plan_ui"` to root `Cargo.toml` `members`.
- **Files:** `crates/plan_ui/Cargo.toml`, `crates/plan_ui/src/plan_ui.rs`, root
  `Cargo.toml`.
- **Acceptance:** `cargo build -p plan_ui` succeeds; the render body resolves every color
  through `cx.theme()`; `cargo tree` confirms no crate depends on `plan_ui`.

### T5 — Wire the fork diff (register the panel behind the flag)
`[PLAN-M0]: register plan panel behind PLAN feature flag`
- `crates/zed/Cargo.toml`: add `plan_ui` dependency.
- `crates/zed/src/main.rs`: add `plan_ui::init(cx);` alongside the other crate inits.
- `crates/zed/src/zed.rs` `initialize_panels`: add the `let plan_panel =
  PlanPanel::load(handle, cx);` + `add_panel_when_ready(plan_panel, …)` entry, wrapped so
  the panel is only added when the flag is on.
- **Flag mechanism (M0):** gate on an env var (`ZED_PLAN=1`) checked inside
  `plan_ui::init` / at registration. Zero upstream-settings surface, self-controlled,
  trivially removable. The durable `"plan"` setting arrives in M9 (F12.1).
- **Files:** `crates/zed/Cargo.toml`, `crates/zed/src/main.rs`, `crates/zed/src/zed.rs`.
- **Acceptance:** with `ZED_PLAN=1`, the "Plan" panel appears in the bottom dock, can be
  toggled, and can be right-click-moved (best-effort — persistence of position deferred);
  without the flag, no panel and zero behavioural change vs. upstream. `cargo build -p
  zed` green.

### T6 — Create `FORK_DIFF.md` manifest
`[PLAN-M0]: add FORK_DIFF manifest of upstream touches`
- List every upstream file M0 modified and why, in one table:
  root `Cargo.toml` (+2 members), `crates/zed/Cargo.toml` (+1 dep),
  `crates/zed/src/main.rs` (+1 init line), `crates/zed/src/zed.rs` (panel registration).
- State the discipline: conflicts on merge **outside** this list mean a leak; new crates
  are additive and never appear here except as dependency/member lines.
- **Files:** `FORK_DIFF.md` (new, repo root).
- **Acceptance:** manifest matches `git diff --stat main upstream/main` for tracked files;
  `merge-upstream.sh` can read it to classify conflicts.

### T7 — Milestone note
`[PLAN-M0]: add M0 milestone note`
- `docs/milestones/M0.md`: what works, what's deferred, surprises, the recorded build
  time, and the flag decision. Per Part III definition-of-done.
- **Files:** `docs/milestones/M0.md`.
- **Acceptance:** note present; DoD checklist satisfied.

---

## Definition of done (Part III §4)
- [ ] `cargo build -p zed` green; app launches with and without `ZED_PLAN`.
- [ ] Demo reproducible: `ZED_PLAN=1` → a "Plan" hello panel toggles in the bottom dock.
- [ ] `FORK_DIFF.md` lists every upstream file touched, and it's the *only* set touched.
- [ ] `script/merge-upstream.sh` runs and reports cleanly.
- [ ] `docs/milestones/M0.md` written.
- [ ] `plan_core` builds with no GPUI dep; nothing depends on `plan_core`/`plan_ui`.

## Test posture
M0 is infrastructure — no unit tests. Verification is: it builds, it launches, the flag
toggles the panel. Test-first discipline begins in M1 (`plan_core` schema over the LED-212
fixture).

---

## Open questions (need a decision before / during M0)

1. **Branch model (blocking — needs your call).** PRD §5 wants `main` to *mirror
   upstream* and a `plan` *integration branch*. Today we're committing on `main`, which
   already carries `docs/`. Options: **(a)** create a `plan` integration branch now and
   land all M0+ work there, keeping `main` as the clean upstream mirror (recommended,
   matches §5); **(b)** treat `main` as the working branch and skip the mirror/integration
   split. Creating a branch is non-destructive, but I'll not restructure branches without
   your sign-off (Part III hard rule). **Which model?**

2. **Feature-flag mechanism.** Recommendation: **env var `ZED_PLAN=1`** for M0 — zero
   upstream-settings surface, self-owned, removable. Alternatives: a compile-time cargo
   feature on the `zed` crate, or jumping straight to the `"plan"` setting (drags in the
   M9 settings-content work early — not recommended). **OK to use the env var for M0?**

3. **Hello panel vs. hello tab.** M0 says "hello panel." The real feature has *both* a
   dock `Panel` (F1.3) and a center-pane `Item` tab (F1.1). M0 builds only the **panel**
   (simplest thing that proves the dock-registration diff). The `Item`/`SerializableItem`
   tab is M3. **Confirm M0 = panel only.**

4. **`merge-upstream.sh` autonomy.** Should the script actually *perform* the weekly merge,
   or only fetch + report status and leave the merge to a human? Recommendation:
   fetch + report in M0 (safe), add auto-merge-with-conflict-guard later once FORK_DIFF is
   established. **Preference?**

## Risks touched (Part II §7)
- *GPUI learning curve* — mitigated by copying GitPanel verbatim (T4).
- The other §7 spikes (acp_thread events, hooks, file-watch, skill adherence, re-anchoring)
  are **not** exercised in M0; they land in M2–M5. Desk-research findings for the four
  earliest are in `docs/zed-notes.md` and were favorable (notably: acp_thread tool-call
  granularity is available; thread-following needs no upstream patch).
