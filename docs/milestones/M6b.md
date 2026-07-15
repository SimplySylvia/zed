# M6b · Execution — gates + step guards — milestone note

**Status:** COMPLETE. `plan_core` + `plan_server` + `plan_ui` + full `zed` build green;
**§13 visual verification under One Dark confirmed by the developer (2026-07-15)** — a holding
guard pulses with needs-you chrome, the clear affordance clears it to a receipt (verified
against the saved plan.json), and the gate card approves. Slice b of M6; M6c follows.

**Feature IDs:** F4.5 (GATE tasks pause + evidence), F4.5b (step guards — ⛨ approve / ✋ input,
hold, PreToolUse-enforced, cleared-guard receipts, policy force-guards), F6.3 (PreToolUse
enforcement).

## What works (built)

- **`plan_core::exec`** guard/gate lifecycle (shared): `hold_guard` (→ holding), `clear_guard`
  (records the ✋ input response, marks cleared, attaches a `guard` evidence entry to each
  `evidence_for` criterion, leaves a history receipt), `approve_gate` (marks a GATE task done),
  `current_hold` (the active holding guard or in-progress GATE). Plus `GuardPolicy` (Appendix B
  `guards`), `matched_require_on`, and `command_guard_cleared` (the q3 coverage rule). **6 tests.**
- **PreToolUse enforcement** (`plan_server::hooks::pretooluse_gate`) — while executing, **denies**
  edits/Bash when a guard or GATE is holding, and denies a Bash command matching
  `policy.guards.require_on` unless a covering guard is cleared; deny messages quote the guard
  prompt. Tools `plan_hold_guard` / `plan_clear_guard` / `plan_approve_gate`. **4 tests** incl.
  an end-to-end MCP call.
- **Plan tab** — a holding guard badge **pulses**; a cleared guard shows a mono receipt; the task
  card renders a clear affordance per holding guard (⛨ Approve to run / ✋ Record & continue,
  writing `plan.json`). A GATE hold renders a **§6 gate card** with ✓ Approve gate. The tab dot +
  status pill flip to **needs-you** (modified, pulsing) whenever `current_hold` is set.

## The round-trip
The agent marks a guard `holding` (`plan_hold_guard`) when it reaches the step → the PreToolUse
hook **blocks** its edits/commands (not asked nicely) → the badge pulses, pill/dot flip to
needs-you → the user clears it in the tab (⛨ approve / ✋ record) → evidence attaches to the
linked criterion, a receipt is stamped, and the hook lets the agent proceed. Policy force-guards
(`require_on`) block matching Bash commands until a covering guard is cleared. GATE tasks hold and
clear via ✓ Approve gate.

## §13 record — decisions & deviations
**Decisions (approved at sign-off):**
- **Q1 hold mechanism:** the agent marks authored guards `holding` (`plan_hold_guard`); the hook
  blocks while holding. Policy command-guards (`require_on`) are hook-inferred from the command.
- **Q2 GATE depth:** the agent holds at a GATE (skill), the UI shows the card + Approve gate, and
  the hook blocks edits while a GATE is in-progress (`current_hold`). Full task-graph gate
  sequencing is later.
- **Q3 command coverage:** a `require_on` Bash command is allowed iff the in-progress task has a
  `cleared` guard whose prompt contains the matched pattern. Simple + testable; refine later.
- **Q4 `cleared_at`** stays `None` (time-free); the server may stamp.

**Deviations (flagged, deferred):**
- **✋ input free-text field deferred** (fidelity) — the panel's Record & continue button clears
  the guard + attaches evidence + leaves a receipt, but does not yet capture a typed response in
  the UI (like the M5a comment composer). The agent's `plan_clear_guard` **can** pass a response;
  only the UI field is deferred.
- **Policy auto-add of ⛨ to matching steps** (Appendix B comment) is a later lint rule (M5c
  engine can host it), not built here.
- **Guard-badge pulse uses one element id** (`guard-hold-pulse`) — fine because `current_hold`
  surfaces one hold at a time; revisit if simultaneous holds render.

**§13 protocol (developer, under One Dark):**
1. On an executing plan with a step guard, set the guard `state` to `holding` (or have the agent
   call `plan_hold_guard`) → the badge **pulses**, the pill/dot flip to needs-you, and a clear
   affordance (⛨ Approve to run / ✋ Record & continue) appears on the task.
2. Click it → the guard shows a **cleared receipt**, evidence lands on the linked criterion, the
   needs-you chrome clears.
3. Set a GATE task `in_progress` → a **gate card** + ✓ Approve gate appears; approving marks it
   done and clears the hold.

## Fork discipline
Additive only. `plan_core::exec` gained guard/gate helpers + `GuardPolicy`; `plan_server` gained
hook enforcement + three tools; `plan_ui` gained the guard controls, gate card, and needs-you
chrome. **No upstream files touched** — `FORK_DIFF.md` unchanged.

## Definition of done
- [x] `plan_core::exec` guard/gate helpers + policy match correct, test-first.
- [x] PreToolUse denies at a hold / on a `require_on` command, allows after clearing; tools; one MCP call.
- [x] Tab: holding guard pulses + clear affordance → receipt + evidence; GATE card + Approve gate;
      needs-you pill/dot.
- [x] Builds green; clippy clean; pure logic test-first; `docs/milestones/M6b.md`; FORK_DIFF unchanged.
- [x] **§13 visual verification under One Dark** — confirmed by the developer (2026-07-15).

## Next: M6c — Amendments + failure ladder + control
Amendments as staged diffs **reusing `plan_core::rev`** (F4.7), the failure ladder + escalation
card (F11.4), pause/resume/stop + kill (F5.1/F5.6), interrupted-task recovery (F11.1), the Stop
loop guard, and lease enforcement + reclaim (F11.3b). Then M7 (git).
