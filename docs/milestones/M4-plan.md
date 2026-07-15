# M4 · Pill + panel + activity — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans;
> superpowers:test-driven-development for the pure logic (pill/fragment, activity-row, and
> sync-receipt mapping); and the **`plan-ui-design` skill + `plan-ui-compliance.md`** for
> every rendering task (run §13 per component — pill §1, panel §10).

**Status:** awaiting sign-off (not started).
**Feature IDs:** F1.4 (status-bar pill), F1.3 (plan panel), F4.3 (active-task spotlight +
live activity feed), F5.3b (sync receipts). F1.5 (threads-sidebar state) — assess/defer.
**Goal:** A status-bar pill and a real dock panel that both follow the active thread's plan,
with a **live activity feed fed by `acp_thread` events** and sync receipts. This is where the
M0 "hello" panel becomes the real Plan panel.
**Demo:** run a task through the agent (M2 server) and watch the pill update, the panel's
pipeline column track task status, and the live column stream tool activity.

**Architecture:** the pill (`StatusItemView`) and panel (`Panel`) both follow the active
thread exactly like the M3 tab — subscribe to `AgentPanel::ActiveViewChanged`, resolve the
plan via the shared helper. The live column subscribes to the active `AcpThread`'s
`AcpThreadEvent`s (tool-call-granular: `NewEntry`/`EntryUpdated(ix)` + the `ToolCall` entry
with `kind`/`status`/`locations`, confirmed in the M0 study). All colors via `cx.theme()`.

**Tech stack:** Rust + GPUI (`plan_ui`), `plan_core`, `agent_ui`/`acp_thread`, `workspace`
(`StatusItemView`, `Panel`), `ui`/`theme`. GPUI views: smoke + §13 visual; pure logic: TDD.

Build env: `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`,
`PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"`. Commit prefix `[PLAN-M4]`.

---

## Tasks

### T1 — Extract a shared "active-thread plan follower" (refactor)
`[PLAN-M4]: extract shared plan-following helper`
Pull the M3 tab's plans-dir resolution + `resolve_plan` + `AgentPanel` subscription + 500ms
poll into a reusable helper (e.g. `plan_ui::following` with a `PlanFollower` or free fns) so
the tab, pill, and panel share one implementation. Refactor `PlanView` to use it (behavior
unchanged; existing checks still pass). **Acceptance:** `plan_ui` builds; tab still works.

### T2 — Status-bar pill (F1.4, compliance §1) (TDD for the mapping)
`[PLAN-M4]: add the status-bar Plan pill`
- A `StatusItemView` that follows the active thread and renders `◆ Plan <fragment>` per the
  **state matrix (§9)** — fragment text + color family for all rows (e.g. `2/5 · acc 1/6`
  when executing, `drafting…`, `approved — launch?`, `t4 failed — needs you`). Click toggles
  the plan panel. Reflects the *active thread's* plan only.
- **Tests first:** a pure `pill_fragment(&Plan) -> (String, Color)` mapping, asserted across
  the matrix rows (executing shows `n/m · acc x/y`, failed shows needs-you, etc.).
- Register in `zed.rs` status bar (`add_left_item`/`add_right_item`) — **new upstream lines →
  FORK_DIFF update**. Pulse (gate/guard-hold) deferred with motion (open Q).

### T3 — Panel follows the thread + header + pipeline column (F1.3, compliance §10)
`[PLAN-M4]: make the plan panel real (header + pipeline)`
Replace the M0 hello body: the panel follows the active thread (shared helper); header shows
status dot + `Plan · <id>` + progress + **sync receipt** (T5) + primary action per matrix;
**pipeline column** = task rows with acceptance dots and per-state tints (active info / failed
error / gate+guard amber) + right-side mono note (sha / "running" / "✋ guard" / "GATE").
**Acceptance:** panel renders the LED-212 fixture's pipeline; §13 §10 checks.

### T4 — Live activity column (F4.3) — subscribe to acp_thread events
`[PLAN-M4]: add the live activity feed from acp_thread events`
Subscribe to the active `AcpThread` (`AgentPanel::active_agent_thread`) for `AcpThreadEvent`s;
render the live column: a teal mono verb column (plan/edit/run/guard/hook/ev/drift) + detail,
derived from `AgentThreadEntry::ToolCall` (`kind`/`status`/`locations`); completions green,
failures red, the live row pulses (motion deferred). Also render paused/gate/failure states.
- **Tests first:** a pure `activity_row(&AgentThreadEntry) -> (verb, detail, Color)` mapping
  over fixture entries (edit → "edit route.tsx", run → command, failed → red).
**Acceptance:** with the agent running, the column streams tool activity live.

### T5 — Sync receipts (F5.3b)
`[PLAN-M4]: show the sync receipt in the panel header`
Header shows `agent synced rev N · <time>` from the plan's `rev` + latest `history[]` stamp
(the server stamps these on every write, M2). **Tests first:** a pure
`sync_receipt(&Plan) -> String`. The amber "1 rev behind — syncs before next task" state
needs the agent's last-read rev (server-tracked) — **deferred** (open Q), noted in the receipt
area. **Acceptance:** receipt reflects the fixture's rev + last history entry.

### T6 — Threads-sidebar state (F1.5) — assess or defer
`[PLAN-M4]: (assess) threads-sidebar plan state`
Investigate whether thread rows in the agent panel can carry plan state ("plan 2/5", "gate —
needs you") + worktree badges cheaply/additively. **If it needs patching `agent_ui`'s thread
list rendering, DEFER** and record why (per PRD "if cheap or defer"). **Acceptance:** a short
note in `zed-notes.md` with the decision.

### T7 — Milestone note + FORK_DIFF + §13
`[PLAN-M4]: add M4 milestone note`
`docs/milestones/M4.md` (what works, §13 record, deviations); update `FORK_DIFF.md` for the
status-bar registration lines; full `zed` build; hand off the One-Dark §13 visual check.

---

## Definition of done
- [ ] Pill shows `◆ Plan <fragment>` per the state matrix; follows the active thread; toggles the panel.
- [ ] Panel follows the thread; pipeline column tracks task status; live column streams `acp_thread` activity.
- [ ] Sync receipt renders from rev + history.
- [ ] Shared follower used by tab + pill + panel (no duplicated logic).
- [ ] Pure mappings (pill/activity/receipt) test-first; `cargo build -p zed` green; smoke tests pass.
- [ ] `docs/milestones/M4.md` written; `FORK_DIFF.md` updated for the pill registration; §13 visual check done.

## Open questions (recommendations in parens)
1. **Pill placement** — left or right status-bar item? *(left / leading, near other primary status)*.
2. **Motion (pulse on gate/guard-hold; live-row pulse)** — implement now or defer to the fidelity pass with M3's deferred motion? *(defer — batch all motion together)*.
3. **Sync-receipt "rev behind" amber** — needs the server to record the agent's last-read rev on `plan_get`. Implement that now or defer? *(defer; show rev + timestamp for M4)*.
4. **Threads-sidebar (F1.5)** — attempt or defer? *(defer unless it's clearly additive — it likely needs `agent_ui` thread-row changes)*.
5. **M0 hello panel** — replace its body with the real panel (keeping the same `PlanPanel` registration)? *(yes)*.
6. **Activity feed retention** — cap the live column at, say, the last ~50 entries for M4? *(yes, with a note)*.
