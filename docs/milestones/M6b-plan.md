# M6b · Execution — gates + step guards — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS — superpowers:executing-plans; superpowers:test-driven-development
> (guard/gate helpers + hook enforcement are test-first); the `plan-ui-design` skill +
> `plan-ui-compliance.md` (§7 guard badges + input-guard panel, §6 gate evidence card, §9 needs-you
> chrome) for the UI. This is the enforcement-heavy slice — the §7 spike (hooks *can* deny) is
> already retired (see zed-notes), so build on it.

**Status:** awaiting sign-off (not started). Slice **b** of M6 (M6a launch spine done).

**Feature IDs:** F4.5 (GATE tasks pause for sign-off + evidence; never expire), F4.5b (step
guards — ⛨ approval / ✋ input, hold at the step, PreToolUse-enforced, cleared-guard receipts,
policy force-guards), F6.3 (PreToolUse enforcement). Guard *badges* already render from draft
(M3/§7); this slice makes them **hold, block, and clear** at execution.

**Goal:** at execution the run **holds** at a guarded step or GATE — the agent is *blocked by the
PreToolUse hook* (not asked nicely), the badge pulses, the pill/panel flip to needs-you — and the
user **clears** it: ⛨ approve, or ✋ record input (stored on the step + attached as evidence),
leaving a receipt (who/when/what). Policy can force ⛨ guards onto commands matching
`policy.guards.require_on` (e.g. `migrate|seed|drop`).

**Architecture:** unchanged. Guard/gate mutations are pure `plan_core::exec` helpers (shared by
UI + server). The **PreToolUse hook** (already in `plan_server::hooks`) gains the guard/gate
denial. The **UI** clears guards/gates by writing `plan.json` directly; the **agent** clears via
tools. Enforcement **blocks** (deny), per the enforcement philosophy.

**Tech stack:** Rust — `plan_core::exec` (guard/gate helpers + policy-guard match), `plan_server`
(hook enforcement + `plan_hold_guard`/`plan_clear_guard`/`plan_approve_gate`), `plan_ui` (guard
panel + gate card + needs-you chrome). Pure logic test-first; GPUI = smoke + §13. Commit prefix
`[PLAN-M6b]`.

---

## Tasks

### T1 — `plan_core::exec` guard/gate helpers + policy-guard match (TDD)
`[PLAN-M6b]: add guard and gate lifecycle helpers to plan_core`
- `hold_guard(plan, task, step)` — set the step guard `state = holding` (the agent marks this when
  it reaches the step; q1). Errors if no guard there.
- `clear_guard(plan, task, step, response: Option<Value>)` — set `state = cleared`, store
  `response` (for ✋ input), `cleared_at = None` (time-free; server may stamp), and **attach
  evidence**: for each id in the guard's `evidence_for`, push an evidence entry onto that
  acceptance criterion (F4.5b). Rev-bump + history "guard_cleared" (the receipt).
- `approve_gate(plan, task)` — mark a GATE task cleared so execution proceeds (q2). Rev-bump.
- `current_hold(plan) -> Option<Hold>` — the active hold for the UI/hook: the first `holding`
  guard, else a pending GATE task blocking progress. (`Hold { task, step?, kind }`.)
- **Policy guards** (Appendix B `guards`): parse `{ require_on: [String], default_type }` (extend
  `lint::Policy` or a sibling), and `command_requires_guard(policy, command) -> bool` (substring
  match on `require_on`) + `command_cleared(plan, command) -> bool` (a `cleared` guard on the
  in-progress task whose prompt/step covers it; q3).
- **Tests:** hold→clear round-trip (approve + input); input response stored + evidence attached to
  `evidence_for` criteria; approve_gate clears the gate; `current_hold` finds the holding guard /
  pending gate; `command_requires_guard` matches `require_on`; unknown-target guards error.

### T2 — PreToolUse guard/gate enforcement + tools (TDD)
`[PLAN-M6b]: enforce guard and gate holds in the PreToolUse hook`
- Extend `hooks::pretooluse_allows_edit` (rename → `pretooluse_gate`): in addition to the
  no-plan/not-executing check, **deny** when (a) `current_hold(plan)` is `Some` (a guard is holding
  or a gate blocks) — the agent is held until cleared; or (b) the tool is `Bash` and its command
  matches `policy.guards.require_on` without a covering `cleared` guard (q3). Deny messages quote
  the guard prompt (F4.5b: "about to run the local DB migration — approve to run").
- Tools reusing T1: `plan_hold_guard(id, task, step)`, `plan_clear_guard(id, task, step, response?)`,
  `plan_approve_gate(id, task)`.
- **Tests:** holding guard → edit/Bash denied; after `clear_guard` → allowed; a `require_on`
  command (e.g. `prisma migrate`) denied, then allowed once a covering guard is cleared; gate hold
  → denied until `approve_gate`; one end-to-end MCP call.

### T3 — Guard panel + gate card + needs-you chrome (compliance §7/§6/§9) [M6b]
`[PLAN-M6b]: add the guard panel, gate card, and needs-you chrome to the Plan tab`
- **Step guard at hold** (§7): the `holding` guard badge **pulses** (amber ⛨ / teal ✋); an
  **input-guard panel** (amber border, caps label naming the evidence target — "stored as evidence
  on a1", input field, primary **Record & continue ⏎**, secondary pause) captures the response;
  ⛨ approval shows **Approve to run**. Clearing writes `plan.json` (T1) → cleared receipt renders
  (mono ✓ + timestamp + what ran).
- **GATE evidence card** (§6): mono evidence rows + **✓ Approve gate**.
- **Needs-you chrome** (§9): at a hold the pill flips to `guarded step — input needed`
  (warn·pulse) / `GATE — needs you`, and the panel header action becomes **Record input** /
  **Approve gate**. (Reuses the M6a spotlight + fidelity pulse helpers.)
- **Acceptance:** a plan with a `holding` guard shows the pulsing badge + input panel; recording
  clears it (receipt + evidence on the linked criterion); a GATE task shows the card + Approve
  gate; §13 §7/§6/§9 under One Dark.

### T4 — Milestone note + FORK_DIFF + §13
`[PLAN-M6b]: add M6b milestone note`
`docs/milestones/M6b.md`; FORK_DIFF expected unchanged (additive); full `plan_ui` + `zed` build;
§13 handoff. Next: M6c (amendments + failure ladder + control).

---

## Definition of done
- [ ] `plan_core::exec` guard/gate helpers + policy-guard match correct, test-first (hold/clear,
      input response + evidence, approve_gate, current_hold, require_on match).
- [ ] PreToolUse denies at a hold / on a `require_on` command, allows after clearing; tools present;
      one end-to-end MCP call.
- [ ] Tab: holding guard pulses + input panel captures → cleared receipt + evidence; GATE card +
      Approve gate; needs-you pill/panel.
- [ ] Builds green; clippy clean; pure logic test-first; `docs/milestones/M6b.md`; FORK_DIFF unchanged.
- [ ] §13 visual verification under One Dark — developer gate.

## Open questions (recommendations in parens)
1. **How a step guard reaches `holding`.** *(The agent marks it via `plan_hold_guard` when it
   reaches the step, per the skill — explicit + observable; the hook then blocks until cleared.
   Command-guards (`require_on`) are hook-inferred from the command text, needing no `holding`
   state. Rec: both — agent-marked holds for authored guards, pattern-match for policy guards.)*
2. **GATE enforcement depth.** *(MVP: the agent holds at a GATE per the skill, the UI shows the
   evidence card + Approve gate, and `approve_gate` clears it; the hook blocks edits while
   `current_hold` reports a pending gate. Full task-graph gate sequencing is later. Flag.)*
3. **What "covers" a `require_on` command when cleared (the trickiest).** *(Rec MVP: a `require_on`
   Bash command is allowed iff the in-progress task has a `cleared` guard whose prompt contains the
   matched pattern — i.e. the user approved that class of command on this task. Simple + testable;
   refine (one-shot allowances, exact command capture) later. Flag explicitly.)*
4. **`cleared_at` timestamp** — `None` in `plan_core` (time-free); the server may stamp. Not blocking.
5. **Policy-guard auto-add to steps** (Appendix B comment "lint auto-adds ⛨ to matching steps").
   *(Out of M6b — that's a lint rule (M5c engine can host it later); M6b enforces at the hook +
   clears in the UI. Flag.)*

## Heads-up
The enforcement core (T1/T2) is the load-bearing, design-heavy part — expect a checkpoint after
T2 before the UI. The rock-solid, fully-testable pieces are **policy command-guards at the hook**
and **block-while-holding**; the GATE sequencing (q2) and command-coverage (q3) carry the real
ambiguity and are scoped to defensible MVP behavior with flags. Git stays in M7; amendments +
failure ladder + control are M6c.
