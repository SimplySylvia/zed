# M6a · Execution — launch + the per-task loop spine — milestone note

**Status:** COMPLETE. `plan_core` + `plan_server` + `plan_ui` + full `zed` build green;
**§13 visual verification under One Dark confirmed by the developer (2026-07-15)** — approve →
`▶ Launch` → `executing` + lease + accent-pulse tab dot, and the in-progress task spotlight.
First slice of M6 (execution); M6b (gates/guards) and M6c (amendments/control) follow.

### UI follow-ups fixed during §13 (committed separately, `[PLAN]`)
Discovered while validating the launch flow; all `plan_ui`-only, additive:
- **Panel toggle** — the status pill now opens **and closes** the panel (it used
  `toggle_panel_focus`, which only closes a focused panel; a status-bar click doesn't retain
  focus). Now checks whether Plan is the visible dock panel and toggles explicitly.
- **Pill hover + selected** — standard `element.hover` on hover (G8) and a solid
  `element.selected` fill while the panel is open, matching the other status-bar toggles;
  replaced the always-on faint tint/border.
- **Pill active emphasis** — `shadow_sm` on the open pill so the active state reads clearly raised.
- **"Open as tab"** — a Maximize icon button in the panel header (F1.3) opens/activates the full
  Plan document tab, via a shared `PlanView::open_tab`.

**Feature IDs:** F4.1 (launch + lease + per-task loop), F3.6 (Approve→Launch), F4.3 (active-task
spotlight), F11.3 (executor lease), F6.3 (PreToolUse enforcement — from M2), F6.1 (skill loop).

## What works (built)

- **`plan_core::exec`** (shared by UI + server) — `launch(plan, thread)` transitions
  `approved → executing` and takes the single executor lease (F4.1/F11.3): idempotent re-launch
  under the same thread, errors when another thread holds the lease or the plan isn't `approved`;
  `release_lease` clears it. `taken_at` stays `None` (time-free). **5 tests.**
- **`plan_launch`** (`plan_server`) — `plan_launch(id, thread)` reusing `exec::launch`; the
  per-task loop reuses the existing `task_update`. **3 tests**, incl. a regression that the
  **existing PreToolUse gate denies edits before launch and allows them after** (F6.3), and one
  end-to-end MCP call.
- **Tab** — the toolbar primary shows **`▶ Launch`** for `approved` plans; clicking launches
  (writes `plan.json` → `executing` + lease). The **active (in-progress) task is spotlighted**
  with an info-tinted background + info border (F4.3), alongside the executing tab-dot pulse from
  the fidelity pass.

## The loop
Approve (M5b) → **`▶ Launch`** → `executing` + lease → the PreToolUse gate flips blocked→allowed,
so the agent may edit code and drive `plan_get → task → tests → commit → task_update`; the tab
spotlights the active task and the pill shows `n/m · acc x/y`. Enforcement is real (the gate
blocks edits until launch), reusing the M2 hook + skill scaffolding.

## §13 record — decisions & seams (developer gate + follow-ups)

**Decisions (approved at sign-off):**
- **Git stays in M7** (q1) — Launch does not create a branch/worktree; it launches in the current
  tree. F4.1 lists branch setup under Launch, but that's M7 (F10.2). The branch strip + commit
  rail are M7.
- **ExitPlanMode is a seam** (q2) — M6a ships the UI Launch button + `plan_launch` + the
  status/lease transition. Wiring the agent's ACP **ExitPlanMode** to call `plan_launch` (so
  "approve to exit plan mode" == launch) is handled by the skill for now; deeper ACP
  reconciliation is later. Like the thread-id seam, not a spike failure.
- **Lease enforcement deferred** (q3) — M6a *sets* the lease; the PreToolUse gate keeps its
  `status == executing` check. Leaseholder-only editing + stale-lease reclaim (F11.3b) → M6c.
- **`taken_at` = `None`** (q4, time-free); **F6.4 TodoWrite↔plan.json bridge** out of scope (q5),
  the loop uses `task_update` explicitly.

**§13 protocol (developer, under One Dark):**
1. With an **approved** plan, the toolbar shows **`▶ Launch`**; the pill is `approved — launch?`
   (created/green).
2. Click **Launch** → status flips to `executing`; the tab dot turns accent + **pulses**; the
   pill shows `n/m · acc x/y`; no Approve/Launch primary while executing.
3. Set a task's `status` to `in_progress` (edit the plan or via the agent) → that task card is
   **spotlighted** (info-tinted bg + info border).
4. (Enforcement, optional) confirm the agent is blocked from editing before launch and allowed
   after — this is the `plan_launch` regression test's runtime counterpart.

## Fork discipline
Additive only. `plan_core` gained the `exec` module; `plan_server` gained `plan_launch`; `plan_ui`
gained the Launch primary + spotlight. **No upstream files touched** — `FORK_DIFF.md` unchanged.

## Definition of done
- [x] `plan_core::exec` launch/lease correct + test-first (from-approved-only, lease guard,
      idempotent, release).
- [x] `plan_launch` present + correct; PreToolUse gate denies-before/allows-after; one MCP call.
- [x] Tab: `▶ Launch` from approved → executing + lease; active-task spotlight; executing chrome.
- [x] Builds green (`plan_core`/`plan_server`/`plan_ui`/`zed`); clippy clean; pure logic test-first.
- [x] `docs/milestones/M6a.md` written; FORK_DIFF unchanged.
- [x] **§13 visual verification under One Dark** — confirmed by the developer (2026-07-15).

## Next: M6b — Gates + step guards
GATE-task pause + evidence card (F4.5), step guards holding at the step via the PreToolUse hook
(⛨ approve / ✋ input capture + evidence, cleared-guard receipts, F4.5b), policy force-guards
(`guards.require_on`), and the needs-you chrome (guard badge pulse, pill/panel flip). Then M6c
(amendments reusing `plan_core::rev` + failure ladder + pause/resume/stop/kill + recovery).
