# M5c · Review loop — the policy.json lint engine — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans;
> superpowers:test-driven-development (the lint engine, checkers, and reconcile are test-first
> over the LED-212 fixture + mutations); and the `plan-ui-design` skill + `plan-ui-compliance.md`
> (§6 Lint card, §3 blocker chip) for any lint UI.

**Status:** awaiting sign-off (not started).
**Feature IDs:** F9.1 (plan lint — rules from `policy.json`, auto-flags in the comment system,
author `plan-lint`, blocker-severity gates Approve), F6.2 (`plan_lint` tool). Cross-refs whose
*enforcement points* belong to later milestones: F2.4f `ticket-coverage` (needs tickets → M8),
F10.1 git-format rules (commit-time hook → M7).
**Goal:** a single-source lint engine in `plan_core` (reused by the server) that reads
`.plans/policy.json` (Appendix B) over built-in defaults, runs rule checkers producing typed
findings, and reconciles them into `plan-lint`-authored comments on `plan.json`. Blocker-severity
findings surface in the tab and **gate Approve via the mechanism M5b already built**
(`open_blocker_count`/`approve_enabled` count any blocker comment regardless of author).

**Architecture:** lint lives **once** (`plan_core::lint`), per the handoff's "decisions to honor"
(server reuses `plan_core` — no double-implementation). The **UI writes `plan.json` directly**
(runs lint on load/edit and reconciles findings); the **server** exposes `plan_lint` for the
agent (run + write + return findings so the agent can auto-fix and re-run). Findings render via
the existing M5a comment machinery (author `plan-lint`, anchored to the offending block).

**Tech stack:** Rust — `plan_core::lint` (policy parse, checkers, reconcile), `plan_server`
(`plan_lint`), `plan_ui` (surface findings). Pure logic test-first; GPUI = smoke + §13.

Build env as before. Commit prefix `[PLAN-M5c]`.

---

## Tasks

### T1 — `plan_core::lint` — policy + engine + feasible checkers (TDD)
`[PLAN-M5c]: add the policy.json lint engine to plan_core`
New module `crates/plan_core/src/lint.rs` (PRD Part II §2 layout):
- `Policy` (Appendix B `lint` block) deserialized from `.plans/policy.json`, with **built-in
  defaults** used when the file is absent (mirrors how settings default; the file only tightens).
  Rule severities are data (`"blocker" | "warn" | "off"`); `max-files-per-task` carries a limit.
- `Finding { rule_id, severity, message, block: Option<String> }` and
  `lint(plan, policy, repo_root: Option<&Path>) -> Vec<Finding>` running each enabled rule.
- **Checkers whose inputs exist today** (proposed predicates — see open questions):
  - `max-files-per-task` — `task.files.len() > limit`. (trivial, exact)
  - `criteria-link-tasks` — an acceptance criterion with empty `tasks[]`.
  - `every-task-has-tests` — a non-gate, non-`testing` task with **no linked acceptance**
    (`task.acceptance` empty). *(proposed predicate; q1)*
  - `files-must-exist` — a `task.files[].path` that doesn't exist under `repo_root` (skipped when
    `repo_root` is `None`, e.g. pure fixture tests). *(q2)*
- **Deferred rules, wired as no-ops with a recorded reason** (the engine makes them cheap to add
  when their inputs land): `ticket-coverage` → **M8** (needs `tickets[]`/`ticket_ac`);
  `prod-requires-gate` → **needs a "prod" signal the schema doesn't yet carry** (q3); git-format
  rules (F10.1) → **M7** (commit-time hook).
- **Tests first**: the clean LED-212 fixture yields no blocker findings; targeted mutations trip
  each implemented rule (over-limit files, an unlinked criterion, a testless task, a missing
  file); `off` severity suppresses a rule; absent `policy.json` falls back to defaults.
- **Stop-and-ask** if a proposed predicate (q1/q3) is wrong — don't guess rule semantics.

### T2 — `plan_core::lint` reconcile findings → comments (TDD)
`[PLAN-M5c]: reconcile lint findings into plan-lint comments`
- `reconcile(plan, findings)` — idempotently sync `plan-lint`-authored comments to the current
  findings: add a comment per new finding (author `plan-lint`, severity from the finding,
  anchored to `block`, rule id carried in the comment), and **clear plan-lint comments whose
  finding no longer fires** (so fixing an issue removes its flag). Never touches `user`/`agent`
  comments. Rev-bumps + stamps history only when something changed.
- **Tests**: findings → matching plan-lint comments appear; re-running with the issue fixed
  removes the stale comment; user comments are untouched; a blocker finding produces a comment
  that `open_blocker_count` counts (gate integration).

### T3 — `plan_server` `plan_lint` tool (TDD)
`[PLAN-M5c]: add the plan_lint MCP tool`
- `tools::lint(plans_dir, id)` — load, run `plan_core::lint` (with the plans_dir's repo root),
  `reconcile`, save, and return the findings as JSON (rule id + severity + block + message) so
  the agent can auto-fix what it can and re-run (F9.1). Agent auto-fix logic is skill/agent-side,
  not the tool.
- **Tests**: in-process over a seeded temp `.plans` — lint a plan with a violation → findings
  returned + a plan-lint comment persisted; one end-to-end MCP `tools/call`.

### T4 — Surface lint findings in the Plan tab (compliance §6/§3) [5c]
`[PLAN-M5c]: surface lint findings in the Plan tab`
- Lint findings already render as `plan-lint` comments via M5a; add the **rule id** (mono) to a
  plan-lint comment row and count them in the toolbar (they already flow into the blocker chip +
  Approve gate from M5b). A **run affordance** (e.g. a "Lint" action) runs
  `plan_core::lint` + `reconcile` from the UI (the UI writes `plan.json` directly) so findings
  refresh without the agent.
- **Deferred (flag for fidelity):** the full §6 **Lint card** (dedicated card chassis, `✓
  auto-fixed` dimmed rows, inline fix buttons) — 5c renders findings as comment rows with rule
  ids; the dedicated card is a fidelity-pass upgrade.
- **Acceptance:** a plan with a lint violation shows a `plan-lint` blocker comment (with rule id)
  → the ⚑ blocker chip counts it → Approve is disabled until the issue is fixed and lint re-runs;
  §13 §6/§3 checks under One Dark.

### T5 — Milestone note + FORK_DIFF + §13
`[PLAN-M5c]: add M5c milestone note`
`docs/milestones/M5c.md` (rules implemented vs deferred + predicate decisions); FORK_DIFF
expected unchanged (all additive); full `plan_ui` + `zed` build; §13 handoff. **M5c closes the
M5 review-loop milestone** — the note should say so and point at M6 (execution).

---

## Definition of done
- [ ] `plan_core::lint` parses policy (defaults when absent) + implemented checkers correct,
      test-first; deferred rules are explicit no-ops with recorded reasons.
- [ ] `reconcile` idempotently syncs plan-lint comments (adds new, clears fixed, leaves
      user/agent comments); blocker findings feed `open_blocker_count`.
- [ ] `plan_lint` tool present + correct; one end-to-end MCP call.
- [ ] Tab: a violation shows a plan-lint blocker comment (rule id) → blocker chip counts it →
      Approve gated until fixed + re-linted.
- [ ] Builds green; clippy clean; pure logic test-first; `docs/milestones/M5c.md` written;
      FORK_DIFF unchanged.
- [ ] §13 visual verification under One Dark — developer gate.

## Open questions (recommendations in parens)
1. **`every-task-has-tests` predicate** — flag a non-gate/non-`testing` task with no linked
   acceptance (`task.acceptance` empty)? *(recommend yes for MVP — acceptance is where test
   evidence attaches at execution; alternative "require a testing task or a test-mentioning step"
   is fuzzier. Flagging, not guessing.)*
2. **`files-must-exist` root** — check `task.files[].path` against the first visible worktree
   root (the same dir `.plans/` lives under), skipped when no root is available? *(recommend yes;
   pure fixture tests pass `repo_root: None` and skip it.)*
3. **`prod-requires-gate` signal** — the schema carries no "prod" marker on a task, and policy has
   no prod-path pattern. Options: (a) **defer** the rule to when a signal exists (recommend);
   (b) add a `policy.lint.prod_paths` glob and flag tasks whose `files` match without a gate;
   (c) treat `system: "backend"` tasks touching migration-ish files as prod (too heuristic).
   *(recommend (a) defer — don't invent a prod signal now; revisit with guards/git in M6/M7.)*
4. **Auto-fix ownership** — the tool returns findings and the **agent** auto-fixes (skill), vs the
   engine mutating the plan? *(recommend agent-side per F9.1 "agent auto-fixes what it can"; the
   engine only flags. Keeps the engine pure.)*
5. **Lint UI depth for 5c** — comment rows + rule id + a run affordance now, full §6 Lint card
   later? *(recommend yes — keeps 5c mostly test-first plan_core/server; the card is fidelity.)*
6. **When lint runs in the UI** — on demand (a Lint action) vs automatically on every load/edit?
   *(recommend on-demand for 5c to avoid rev-churn on every poll; auto-on-draft is the agent's
   job via the tool. Flag; auto-run is a small follow-up.)*

## Heads-up
Final slice of the biggest milestone. T1 is load-bearing and the rule predicates (q1/q3) are the
real design decisions — expect a checkpoint after T1. Scope is deliberately the **engine + the
rules whose inputs exist now**; ticket-coverage (M8) and git-format (M7) are wired as recorded
no-ops so they drop in cheaply later. Closing M5c ends the review loop; M6 (execution) is next.
