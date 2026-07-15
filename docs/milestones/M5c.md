# M5c · Review loop — the policy.json lint engine — milestone note

**Status:** COMPLETE. `plan_core` + `plan_server` + `plan_ui` + full `zed` build green;
**§13 visual verification under One Dark confirmed by the developer (2026-07-15)** — clicking
Lint on a plan with a violation surfaces a `plan-lint` blocker comment with its rule id, the ⚑
chip counts it, and Approve is gated; the values (rule ids / severities / blocks / messages) and
the gate state were verified against the saved plan.json. **M5c closes the M5 review-loop
milestone.** Deviations (below) remain.

**Feature IDs:** F9.1 (plan lint — policy rules, auto-flags authored `plan-lint`, blocker
severity gates Approve), F6.2 (`plan_lint`). Deferred by design: F2.4f `ticket-coverage` (M8),
F10.1 git-format rules (M7 commit-time hook), `prod-requires-gate` (no prod signal yet).

## What works (built)

- **`plan_core::lint`** (single source, reused by server + UI) — `Policy` reads the `lint` block
  of `.plans/policy.json` (Appendix B) over built-in defaults (absent file → defaults; the file
  only tightens); `lint(plan, policy, repo_root)` runs the enabled checkers producing typed
  `Finding`s. **Implemented rules:** `max-files-per-task`, `criteria-link-tasks`,
  `every-task-has-tests` (non-gate/non-`testing` task with no linked acceptance — the approved
  Q1 predicate), `files-must-exist` (against `repo_root`; skipped when `None`). **10 tests**
  (clean fixture → none; each rule fires on a mutation; `off` suppresses; defaults vs overrides).
- **`lint::reconcile`** — idempotently syncs `plan-lint` comments to findings: adds one per new
  finding (id `lint-<rule>-<block>`, anchored, severity-mapped), refreshes a drifted one, and
  clears comments whose finding no longer fires; never touches user/agent comments; rev-bumps +
  stamps history only on change. **4 tests**.
- **`plan_lint`** (`plan_server`) — runs lint (worktree root = parent of `.plans/`), reconciles,
  and returns the findings (rule/severity/block/message) for the agent to auto-fix and re-run.
  **2 tests** (in-process + one end-to-end MCP call).
- **Tab** — a **Lint** toolbar action runs lint + reconcile from the UI (writes `plan.json`
  directly, on-demand); `plan-lint` comments render with their rule id (mono); blocker findings
  feed the ⚑ chip + Approve gate built in M5b. **Smoke test** for the rule-id parser.

## The round-trip
Lint (from the tab or the agent's `plan_lint`) → findings reconcile into `plan-lint` comments in
`plan.json` → blocker findings raise the ⚑ chip and disable Approve → the agent (or the
developer) fixes the underlying issue → re-lint clears the resolved findings → Approve enables.
Single source of rule truth: `plan_core::lint`, no server/UI duplication (per the handoff's
"decisions to honor").

## §13 record — decisions & deviations (for the developer gate + a fidelity pass)

**Decisions (approved at sign-off / mapping):**
- **Q1 `every-task-has-tests` = link-to-acceptance** — flags a non-gate/non-`testing` task with
  empty `task.acceptance`. Testing tasks and GATE tasks are exempt.
- **Q3 `prod-requires-gate` deferred** — no "prod" signal exists on a task; revisit with
  guards/git in M6/M7 rather than invent one.
- **Scope:** `ticket-coverage` → M8, git-format rules → M7 — both recorded no-ops the engine
  hosts cheaply later. **Auto-fix is agent-side** (the tool only flags). **Lint runs on-demand**
  in the UI (not per-poll) to avoid rev-churn; the agent runs it at draft via the tool.
- **`plan-lint` comment id** = `lint-<rule>-<block>` (deterministic, so re-runs update in place).

**Deviations (flagged, deferred):**
- **No dedicated §6 Lint card** — findings render as `plan-lint` comment rows with the rule id;
  the full card chassis (auto-fixed dimming, inline fix buttons) is a fidelity-pass upgrade.
- **Criteria-anchored findings** (e.g. `criteria-link-tasks` → an acceptance id) count in the
  gate and render in the comment stream, but not yet under the Spec lens (task-anchored findings
  render under their task). Spec-lens finding rendering → fidelity.
- **Lint blockers aren't manually resolvable** in the UI (✓ resolve is user/agent-only) — they
  clear by fixing + re-linting (F9.1). An explicit lint *waiver* path (F2.4f) is future work.
- **"Send for revision" sweeps plan-lint comments too** (observed during §13) — `mark_sent_batch`
  moves *all* open comments `open → sent`, including machine-authored `plan-lint` flags. Harmless
  (they still gate — `sent != resolved` — and re-lint still clears them when fixed), but ideally
  batch-send would skip `author == "plan-lint"`. Deferred polish, alongside the M5b auto-resolve
  question.

**§13 protocol (developer, under One Dark):**
1. Open a plan with a violation (e.g. a backend task with no linked acceptance, or >8 files) →
   click **Lint** → a `plan-lint` **blocker** comment appears with its rule id (mono); the ⚑
   chip counts it and **Approve is disabled**.
2. Fix the issue (link an acceptance / trim files) → click **Lint** again → the finding's
   comment clears; when no blockers remain, Approve enables.
3. Record the result here and flag any deviation before merge.

## Fork discipline
Additive only. `plan_core` gained the `lint` module; `plan_server` gained `plan_lint`; `plan_ui`
gained the Lint action + rule-id rendering. **No upstream files touched** — `FORK_DIFF.md`
unchanged.

## Definition of done
- [x] `plan_core::lint` parses policy (defaults when absent) + implemented checkers correct,
      test-first; deferred rules are explicit no-ops with recorded reasons.
- [x] `reconcile` idempotently syncs plan-lint comments; blocker findings feed `open_blocker_count`.
- [x] `plan_lint` tool present + correct; one end-to-end MCP call.
- [x] Tab: Lint action flags a violation as a plan-lint blocker (with rule id) → gate reacts.
- [x] Builds green (`plan_core`/`plan_server`/`plan_ui`/`zed`); clippy clean; pure logic test-first.
- [x] `docs/milestones/M5c.md` written; FORK_DIFF unchanged.
- [x] **§13 visual verification under One Dark** — confirmed by the developer (2026-07-15).

## Next: M6 — Execution (M5 review loop is complete)
Launch flow + ExitPlanMode integration, per-task loop enforcement (hooks hard-block no-plan
edits + guard holds), GATE tasks + step guards with input capture (F4.5/F4.5b), amendments as
staged diffs reusing `plan_core::rev` (F4.7), failure-ladder counters (F11.4), pause/resume/stop
+ kill (F5.1/F5.6), interrupted-task recovery (F11.1). Note: the M5b/M5c review surface (staged
revisions, lint, Approve gate) is the on-ramp — Launch is enabled from `approved`.
