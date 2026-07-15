# M6a · Execution — launch + the per-task loop spine — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans;
> superpowers:test-driven-development (the launch/lease helpers + tool are test-first over the
> LED-212 fixture); and the `plan-ui-design` skill + `plan-ui-compliance.md` (§3 toolbar primary,
> §7 task-card active state, §9 state matrix, §10 panel) for the launch UI + spotlight.

**Status:** awaiting sign-off (not started).

## M6 is split (like M5) — this plan is slice **a** of three
- **M6a (this) — Launch + per-task loop spine.** Approve→Launch (`approved → executing`),
  executor lease (F11.3), active-task spotlight (F4.3), and confirming the existing PreToolUse
  gate end-to-end (edits blocked until launched, allowed after). *Demo: approve → launch → the
  agent can now edit + drive the per-task loop; the tab spotlights the active task.*
- **M6b — Gates + step guards** (F4.5/F4.5b): hold at a GATE task / guarded step via the
  PreToolUse hook, ⛨ approve / ✋ input capture + evidence, cleared-guard receipts, gate evidence
  card, policy force-guards. The enforcement-heavy, hook-centric slice.
- **M6c — Amendments + failure ladder + control** (F4.7/F11.4/F5.1/F5.6/F11.1): amendments as
  staged diffs (reuse `plan_core::rev`), the failure ladder + escalation card,
  pause/resume/stop + kill, interrupted-task recovery, the Stop loop guard.

**Feature IDs (M6a):** F4.1 (launch + lease + per-task loop), F3.6 (Approve→ExitPlanMode/Launch),
F4.3 (active-task spotlight), F11.3 (executor lease), F6.3 (PreToolUse enforcement — already
built in M2), F6.1 (skill per-task loop — already present).
**Goal:** a plan can be **launched** from `approved` — it transitions to `executing`, takes the
executor lease, and the existing PreToolUse gate flips from "blocked" to "allowed" so the agent
can edit code and drive the `plan_get → task → tests → commit → task_update` loop; the tab
spotlights the active task.

**Architecture:** unchanged. The **UI writes `plan.json` directly** via `plan_core` (Launch
button); the **server** exposes `plan_launch` for the agent (and ExitPlanMode reconciliation, see
q2). Launch logic is a pure `plan_core` helper reused by both. Git branch creation is **M7** — M6a
launches in the current tree (q1).

**Tech stack:** Rust — `plan_core` (launch/lease), `plan_server` (`plan_launch`), `plan_ui`
(Launch primary + spotlight). Pure logic test-first; GPUI = smoke + §13. Commit prefix `[PLAN-M6a]`.

---

## Tasks

### T1 — `plan_core` launch + lease helpers (TDD)
`[PLAN-M6a]: add launch and executor-lease helpers to plan_core`
Pure functions on `&mut Plan` (reused by UI + server), rev-bumping + stamping history:
- `launch(plan, thread) -> Result<()>` — only from `approved` (else error, so the gate holds);
  sets `status = executing` and `executor = Some(Executor { thread, taken_at: None })`. `taken_at`
  stays `None` in `plan_core` (time-free, consistent with existing `history.at`); the server may
  stamp it (q4).
- `release_lease(plan)` — clears `executor` (used on pause/stop in M6c; added here as the pair).
- Guard: `launch` refuses if a *different* thread already holds the lease (F11.3 — one executor);
  same-thread re-launch is idempotent.
- **Tests:** launch from `approved` → executing + lease set to the thread; launch from
  `in_review`/`drafting` → error, unchanged; launch when another thread holds the lease → error;
  same-thread idempotent; `release_lease` clears it; one rev bump + `history` "launch" entry.

### T2 — `plan_server` `plan_launch` tool (TDD)
`[PLAN-M6a]: add the plan_launch MCP tool`
- `tools::launch(plans_dir, id, thread)` reusing T1 (load → `launch` → save → return plan).
- The per-task loop uses the **existing** `task_update` (pending→in_progress→done) — no new tool.
- Confirms the **existing PreToolUse gate** now composes: before launch it denies edits
  (`plan is Approved, not executing`); after launch it allows them. (No hook code change expected;
  add a regression test if cheap.)
- **Tests:** launch tool → `executing` + `executor` persisted; `pretooluse_allows_edit` denies
  before / allows after launch; one end-to-end MCP call.

### T3 — Launch primary + active-task spotlight (compliance §3/§7/§9/§10) [M6a]
`[PLAN-M6a]: add the Launch action and active-task spotlight to the Plan tab`
- **Toolbar primary (§9 matrix):** `approved` → **`▶ Launch`** (enabled; rehearsal-mismatch
  gating is v1/F9.4 — out of scope). Clicking launches via `plan_core::launch` (UI writes
  `plan.json`), flipping status to `executing`. Extends the existing `render_primary`
  (which already handles review→Approve and staged→Apply-all).
- **Active-task spotlight (F4.3):** the `in_progress` task card already gets the info border;
  add the spotlight emphasis the matrix implies (the executing tab dot already pulses via the
  fidelity pass). Panel pipeline already tints the active row — confirm it reads during execution.
- **Executing-state chrome:** pill shows `2/5 · acc 1/6` (already), tab dot accent+pulse
  (already), no Approve while executing.
- **Acceptance:** with an `approved` plan, `▶ Launch` → status `executing`, dot→accent+pulse,
  the active task is spotlighted; §13 §3/§7/§9/§10 rows under One Dark.

### T4 — Milestone note + FORK_DIFF + §13
`[PLAN-M6a]: add M6a milestone note`
`docs/milestones/M6a.md` (what works / deferred / the ExitPlanMode + branch seams); FORK_DIFF
expected unchanged (additive); full `plan_ui` + `zed` build; §13 handoff.

---

## Definition of done
- [ ] `plan_core::launch`/`release_lease` correct + test-first (from-approved-only, lease guard,
      idempotent, rev/history).
- [ ] `plan_launch` tool present + correct; the PreToolUse gate denies-before / allows-after
      launch; one end-to-end MCP call.
- [ ] Tab: `▶ Launch` from `approved` → `executing` + lease; active-task spotlight; executing
      chrome per §9.
- [ ] Builds green; clippy clean; pure logic test-first; `docs/milestones/M6a.md` written;
      FORK_DIFF unchanged.
- [ ] §13 visual verification under One Dark — developer gate.

## Open questions (recommendations in parens)
1. **Does Launch create the git branch now?** *(No — defer branch/worktree creation to **M7**
   (F10.2); M6a launches in the current tree. F4.1 lists branch setup under Launch, but git is
   M7's milestone. Flag the seam in M6a.md.)*
2. **ExitPlanMode integration depth (F3.6).** *(M6a ships the **UI Launch button** +
   `plan_launch` tool + the status/lease transition. Wiring the agent's ACP **ExitPlanMode** to
   call `plan_launch` (so "approve to exit plan mode" == launch) is a flagged **seam** — like the
   thread-id seam — handled via the skill/hook for now; deeper ACP reconciliation is later. Not a
   spike failure, just a documented boundary.)*
3. **Lease enforcement in the hook.** *(M6a **sets** the lease; the PreToolUse gate keeps its
   current check (`status == executing`). Enforcing that only the leaseholder thread may edit, and
   stale-lease reclaim-with-confirm (F11.3b), are **M6c**/later. Flag.)*
4. **`taken_at` timestamp.** *(Leave `None` in `plan_core` (time-free, matches existing
   `history.at`); the server may stamp it later. Not blocking.)*
5. **ACP bridge F6.4** (TodoWrite ↔ plan.json task status). *(Out of M6a — the per-task loop uses
   `task_update` explicitly. The TodoWrite reconciliation bridge is its own slice; flag for M6b/c
   or a dedicated pass.)*

## Heads-up
M6 is the largest milestone; this slice is deliberately the **spine** (launch + lease + confirming
the enforcement gate), leaning on the M2 scaffolding (PreToolUse gate, skill loop, schema). The
hook-heavy work (gates/guards) is **M6b** and the risky/varied work (amendments, failure ladder,
recovery, control) is **M6c**. Expect a checkpoint after T2 (pure logic + tool) before the UI.
Git (branches/commits/rail) stays in **M7**; don't pull it in — flag if a choice would.
