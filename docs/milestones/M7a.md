# M7a · Git — branch/commit core + enforcement — milestone note

**Status:** COMPLETE (core + enforcement, **no UI**). `plan_core` + `plan_server` + `plan_ui`
build green; full `zed` build green; **no §13** — M7a ships no plan_ui, so the visual
verification lands with M7b. All logic is test-first.

**Feature IDs:** F10.1 (git lint + commit-time format/trailer check), F10.2 (branch on launch +
dirty/stale guard, no worktree isolation), F10.3 (task-scoped commits + trailer), F10.5c
(`destructive_ops: amendment_only`), F6.1/F6.3 (skill git protocol + hook enforcement).

## What works (built)

- **`plan_core::git`** (new, pure — no git shelling): `GitPolicy` (Appendix B `git` block +
  generic defaults, `load`/`from_json`), `slug`, `branch_name`, `base_for`, `trailer_for` /
  `trailer_with`, `commit_subject_ok`, `trailer_present`, `commit_ok`, `launch_block`,
  `is_destructive`. **12 tests.**
- **`plan_core::lint`** — the `git-commit-format` rule (retires the M5c/M7 no-op): each non-gate,
  non-manual task's `commit_message` must satisfy the git format for the plan's ticketed-ness; a
  missing message flags too. Default severity `warn` (the hard block is the commit-time hook).
  `lint::Policy` now carries the git policy + `git_commit_format` severity (lost its `Copy` derive
  — all call sites pass `&Policy`). **4 tests.**
- **`plan_server::git`** (new, the *only* place the feature shells `git`): `is_dirty` (excludes
  `.plans/` churn), `behind_count` (degrades to 0 when the base can't resolve).
- **`plan_server::tools`** — `set_branch` + `record_commit` (git telemetry — stamp
  `plan.git`/task `artifacts`, **no rev bump**); `launch` now runs the F10.2 dirty/stale guard
  (degrades to a no-op outside a git repo) and stamps the *intended* branch/base idempotently.
- **`plan_server::hooks::pretooluse_gate`** — the commit-time check: `git commit` messages are
  parsed (a small quote-aware tokenizer; multiple `-m` paragraphs are rejoined) and validated for
  subject format + `Plan:` trailer; destructive git (`reset --hard`, `push --force`, `branch -D`,
  `clean -f`, …) is blocked whenever a plan governs the session (F10.5c). **10 tests** incl. two
  real-git-repo launch-guard tests + one end-to-end MCP call (`plan_set_branch`).
- **`plan-agent` SKILL.md** — the launch-branch protocol (`git checkout -b <git.branch>
  <git.base>` from the stamped names → `plan_set_branch`), per-task commit with the trailer +
  `plan_record_commit`, tests-green-before-commit, and the never-rewrite-history rule.
- **`.plans/policy.json`** (local dogfood data, untracked like the test plan) — a full Appendix B
  policy so the hook + lint read real config under `ZED_PLAN`.

## The round-trip
Approve → `plan_launch` refuses on a dirty tree / stale base, else transitions to `executing`,
takes the lease, and stamps `git.branch`/`base`. The skill creates that branch and, per task,
commits once with the `Plan: {ticket} rev{rev} task-{task}` trailer — a malformed subject, a
missing trailer, or a destructive git command is **denied by the PreToolUse hook**, not merely
advised — then records the commit via `plan_record_commit`. Draft-time, `git-commit-format` lint
flags tasks whose declared `commit_message` won't pass.

## Decisions recorded (Part III hard-rule: record mapping/format choices)
- **Generic `GitPolicy` defaults**, not Appendix B's LED-specific example: commit format
  `^\[[A-Z]+-\d+\]: .{8,72}$`, base `main`, so an absent policy behaves sanely. A real
  `.plans/policy.json` git block tightens them (approved q's).
- **Trailer `{task}` = the task id verbatim** (`task-t1`), not the doc's illustrative `task-3`
  (which assumed `t3`→`3` stripping). Verbatim is unambiguous and better for `git log --grep`.
- **Dirty-tree guard excludes `.plans/`** — the UI rewrites `plan.json` at launch, so plan-file
  churn must not read as a dirty tree; only non-`.plans/` changes block launch.
- **Branch name derives from the full title slug** (can be long, e.g.
  `led-212-add-pagination-to-the-invoices-table`); the fixture's hand-shortened branch is not
  reproduced. Acceptable for MVP.
- **Launch stamps the *intended* branch idempotently** (only if `git.branch` is unset); the skill
  creates it and confirms via `set_branch` (open question 1 — agent owns git mutation).
- **`base_for` = feature base only**; hotfix detection deferred (no plan-`type` signal — q4).
- **`git-commit-format` lint default `warn`**; the hard block is the commit-time hook.
- **Destructive-op block is lifecycle-independent** (whenever a plan governs the session), so the
  agent can never rewrite history even pre-execution.

## Deferred (honored, non-blocking)
- **M7b (next):** the branch strip + commit rail UI reading live `git_store` state, revert-task-
  commit, and (if cheap) stale-lease reclaim UI.
- **`require_tests_green` (F10.1)** — enforced by the **skill**, not the hook (the hook sees the
  command, not the test result). True green-gating is out of the hook's reach.
- **Lease enforcement/reclaim (F11.3b)** — still deferred (the hook can't identify the acting ACP
  session id), unchanged since M6c.
- **v1 (out of MVP M7):** worktree isolation (F10.2b), PR generation (F10.4), collision/upstream-
  drift rails + rebase-as-amendment + conflict cards (F10.5 except c), git-panel grouping (F10.6).

## Fork discipline
Additive only. New files `crates/plan_core/src/git.rs`, `crates/plan_server/src/git.rs`;
extensions to `plan_core::{lint,schema}` + `plan_server::{tools,hooks,plan_server}`; skill prose.
`crates/plan_core/Cargo.toml` gained `regex.workspace = true` (a crate manifest, `regex` was
already a workspace dep). **No upstream files touched — `FORK_DIFF.md` unchanged.**

## Definition of done
- [x] `plan_core::git` policy + helpers correct + test-first.
- [x] `git-commit-format` lint wired in (M5c no-op retired); test-first.
- [x] `plan_server`: `plan_set_branch`/`plan_record_commit` + commit-time hook (format/trailer +
      destructive deny) + `plan_launch` dirty/stale guard; one end-to-end MCP call.
- [x] Skill teaches branch-on-launch + commit-per-task + trailer + no-destructive; `policy.json`
      git block present (local dogfood).
- [x] `cargo test -p plan_core -p plan_server` green; clippy clean; full `zed` build green;
      `docs/milestones/M7a.md`; FORK_DIFF unchanged.

## Next: M7b — branch strip + commit rail UI (reads live `git_store`; §13 lands there).
