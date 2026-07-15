# M7a · Git — branch/commit core + enforcement — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans;
> superpowers:test-driven-development (the git helpers + policy + hook checks are test-first
> over the LED-212 fixture + a `policy.json` git block). M7a is **no UI**, so the
> `plan-ui-design` skill does not apply until M7b.

**Status:** COMPLETE (2026-07-15) — all tasks done, tests green, clippy clean, full `zed` build
green. See `docs/milestones/M7a.md`. M7a is no-UI; §13 visual checks arrive with M7b.

## M7 is split (like M5/M6) — this plan is slice **a** of two
- **M7a (this) — Branch/commit core + enforcement (no UI).** Pure `plan_core::git` policy +
  helpers (branch name, base, trailer, commit-format match, launch-guard predicate); the
  git-format lint rules wired into `plan_core::lint` (retiring the M5c no-op); `plan_server`
  git tools (`plan_set_branch`, `plan_record_commit`) + the **commit-time PreToolUse check**
  (commit format + trailer, destructive-op block) + the **launch dirty/stale guard**; the
  skill's branch-on-launch + commit-per-task + trailer protocol. *Demo: in plain Claude Code,
  launch refuses on a dirty tree; a launched plan creates the branch, and a task commit with a
  bad message / a `git reset --hard` is denied by the hook while a well-formed
  `[LED-212]: … \n\n Plan: LED-212 rev5 task-3` commit is allowed and recorded to `plan.git`.*
- **M7b — Branch strip + commit rail UI** (F10.2/F10.3 §9.1 visuals): the branch strip under
  the title (`⎇ branch ← base · ↑n ↓n · dirty · PR —`) and the commit rail reading **real repo
  state** via `project::git_store` (SHAs + diffstat on rail nodes and task chips, rail foot
  `▼ base · ↑n`), plus **revert-task-commit** on a rail node. GPUI + §13. Sketched below.

**v1 — explicitly OUT of MVP M7 (flag, do not build):** worktree isolation (F10.2b),
PR generation (F10.4), the **collision & upstream-drift rails** + rebase-as-amendment +
conflict-resolution cards (F10.5 — only **F10.5c destructive_ops** is MVP), git-panel
grouping (F10.6). The NEXT-SESSION note's predicted "collision/drift" third slice is v1; it is
not part of MVP M7.

**Feature IDs (M7a):** F10.1 (git lint + commit-time format/trailer check + one-commit-per-task),
F10.2 (branch on launch + dirty-tree/stale-base guards — **not** worktree isolation), F10.3
(task-scoped commits + `Plan: {ticket} rev{rev} task-{task}` trailer), F10.5c
(`destructive_ops: amendment_only`), F6.1/F6.3 (skill branch/commit protocol + hook enforcement).

**Goal:** the git contract is enforced without any UI. Launch computes and creates the branch
(guarded against a dirty tree / stale base); each task commits once, with the trailer, in a
format the commit-time hook verifies; destructive git is blocked; commits are recorded back into
`plan.git`. This is the enforcement spine M7b's visuals render.

**Architecture:** unchanged decoupling. **`plan_core` stays pure** — no git shelling; its git
helpers take *facts* (dirty, behind, message) and return decisions, so they're testable without a
repo. **`plan_server` owns git command execution** (it's a standalone binary; add a small
`plan_server::git` module that shells `git` in the repo `.plans/` lives under). **`plan_ui` only
reads** git state (M7b, via `git_store`) and writes `plan.json`. Branch/commit *mutations* are the
**agent's** (skill via Bash), consistent with "commit-per-task + trailer via the skill" (F10.3) —
see open question 1.

**Tech stack:** Rust — `plan_core` (git helpers + lint), `plan_server` (git tools, commit hook,
launch guard, git shelling), `plan-agent` (skill + `policy.json` git block). Pure logic test-first.
Commit prefix `[PLAN-M7a]`.

---

## Tasks

### T1 — `plan_core::git` policy + pure helpers (TDD)
`[PLAN-M7a]: add git policy and branch/commit helpers to plan_core`
New `crates/plan_core/src/git.rs`. Parse the `policy.git` block (Appendix B) atop defaults, and
add pure, facts-in helpers reused by the server + skill:
- `GitPolicy` — `branch { pattern, pattern_no_ticket, base { feature, hotfix } }`,
  `commit { format, format_no_ticket, imperative_mood, one_commit_per_task, require_tests_green,
  trailer }`, `destructive_ops` — with Appendix B defaults; `load(plans_dir)` mirroring
  `lint::Policy::load` / `exec::GuardPolicy::load`.
- `branch_name(plan, policy) -> String` — substitute `{ticket}`/`{slug}` (slug from the title,
  lowercased/hyphenated); ticketless → `pattern_no_ticket` (F2.4d).
- `base_for(plan, policy) -> String` — MVP: the `feature` base (hotfix detection deferred, q4).
- `trailer_for(plan, task) -> String` — `Plan: {ticket} rev{rev} task-{task}`; ticketless omits
  the ticket token (q5).
- `commit_ok(policy, message, ticketed) -> Result<(), String>` — regex match (`format` vs
  `format_no_ticket`) **and** trailer presence.
- `launch_block(policy, dirty: bool, behind: u32) -> Option<String>` — the F10.2 guard as a pure
  predicate over supplied git facts (dirty tree / stale base → reason).
- `is_destructive(command) -> bool` — `reset --hard`, `push --force`/`-f`, `branch -D`,
  `checkout --`, `clean -fd`, etc. (F10.5c).
- **Tests:** branch name (ticketed + ticketless slug), trailer (ticketed + ticketless), commit_ok
  accept/reject (format + missing-trailer), launch_block (clean/dirty/behind), is_destructive
  matrix. Fixtures = LED-212 + a `policy.json` git block.

### T2 — git-format lint rules wired into `plan_core::lint` (TDD)
`[PLAN-M7a]: wire git-commit-format lint into the plan_core lint engine`
Retire the recorded M5c/M7 no-op (`lint.rs:225`). Add a draft-time rule:
- `git-commit-format`: each non-gate task's `commit_message` (when present) satisfies
  `git::commit_ok` for the ticketed-ness of the plan; a task missing a `commit_message` flags too.
  Severity from the git policy commit block (default `warn` at draft — the **hard** block is the
  commit-time hook in T3). Findings route through the existing `reconcile` comment path.
- **Tests:** a task with a malformed/absent `commit_message` produces the finding; a clean fixture
  produces none; ticketless plans check against `format_no_ticket`.

### T3 — `plan_server` git tools + commit-time hook + launch guard (TDD)
`[PLAN-M7a]: add git tools, the commit-time hook check, and the launch guard`
- New `crates/plan_server/src/git.rs` — a thin shell wrapper: `is_dirty(repo)`,
  `behind_count(repo, base)`, `head_sha`/`diffstat` helpers (each shelling `git`, returning
  `Result`). Used only by the server (never linked into Zed).
- Tools: `plan_set_branch(id, branch, base, worktree?)` (stamp `plan.git`; called by the skill
  after it creates the branch) and `plan_record_commit(id, task, sha, diffstat)` (append
  `plan.git.commits` + set the task's `artifacts.sha`/`diffstat`) — the server writes for the
  agent (M6-style, one write each, no rev bump for a recorded commit unless we decide otherwise).
- `plan_launch` gains: run `git::launch_block(policy, is_dirty, behind_count)` **before**
  `exec::launch` and refuse with the reason if it fires (F10.2 guard, hard block — q8); on success
  return the computed `branch`/`base` (from `git::branch_name`/`base_for`) so the skill knows what
  to create.
- Extend `hooks::pretooluse_gate` (the existing gate at `hooks.rs:79`): when `tool == "Bash"` and
  the command is a `git commit`, parse the `-m`/`-F` message and deny unless `git::commit_ok`
  passes (F10.1/F10.3); when the command `git::is_destructive`, deny always
  (`destructive_ops: amendment_only`, F10.5c) — routed through the amendment flow instead.
- **Tests:** `plan_set_branch`/`plan_record_commit` stamp correctly; the hook denies a
  bad-format commit / a missing trailer / a `git reset --hard`, and allows a well-formed task
  commit; `plan_launch` refuses on dirty/behind facts and returns the branch spec when clean; one
  end-to-end MCP call.

### T4 — skill + policy git protocol (`plan-agent`)
`[PLAN-M7a]: teach the planning skill the branch/commit git protocol`
- `plan-agent` SKILL.md: on launch, create the branch (`git checkout -b <branch> <base>` from the
  `plan_launch` return), then `plan_set_branch`; in the per-task loop commit **once** with the
  trailer and call `plan_record_commit`; **never** run destructive git (amendment-only, F10.5c);
  assert tests green before committing (q-heads-up on `require_tests_green`).
- Add a `.plans/policy.json` `git` block (Appendix B) to the throwaway test plan / fixtures so the
  hook + lint have real policy to read.
- Config/prose only — no Rust; but it's half the enforcement contract, so it ships in M7a.

### T5 — milestone note + FORK_DIFF + tests green
`[PLAN-M7a]: add M7a milestone note`
`docs/milestones/M7a.md` (what works / what's deferred to M7b + v1 / the branch-ownership +
git-state seams); FORK_DIFF expected **unchanged** (all additive: `plan_core::git`,
`plan_server::git` + tools + hook, skill); `cargo test -p plan_core -p plan_server` green; full
`cargo build -p zed` sanity. No §13 (no UI in M7a).

---

## M7b sketch (separate sign-off after M7a)
- Read live git state in `plan_view` via `project::git_store` — subscribe `RepositoryEvent`
  (`StatusesChanged`/`HeadChanged`/`BranchListChanged`), read `RepositorySnapshot.branch`
  (+ `tracking_status()` for `↑n ↓n`), dirty, `head_commit` (zed-notes study #7). Confirm exact
  accessors when building (as M6a did).
- **Branch strip** under the title (§3.3): `⎇ branch ← base` chip · `↑n ↓n` (behind>0 amber) ·
  dirty dot+label · PR chip **placeholder** (`PR —`; the live PR chip is v1/F10.4).
- **Commit rail** SHAs/diffstat (§3.6/§9.1): rail nodes + the existing sha chip
  (`plan_view.rs:570`) now carry real recorded commits; rail foot `▼ base · ↑n`; amendment
  diamond node.
- **revert-task-commit** — right-click a rail node → `git revert <sha>` via `git_store` (revert
  is a *new* commit → non-destructive → allowed directly; only reset/force-push/branch-D are
  amendment-only — q7). Copy the right-click-menu pattern from `git_ui`.
- **Lease reclaim (F11.3b)** — build the stale-lease reclaim-with-confirm affordance here **only
  if cheap**; lease *enforcement* in the hook stays deferred (still blocked on ACP session id, q6).
- Note + §13 under One Dark.

## Scope guard (don't pull v1 into M7)
Worktree isolation (F10.2b), PR generation (F10.4), collision/upstream-drift rails + rebase-as-
amendment + conflict cards (F10.5 except c), git-panel grouping (F10.6) are **v1**. If a choice
would pull one in, flag it — don't build it.

## Definition of done (M7a)
- [ ] `plan_core::git` policy + helpers correct + test-first (branch/base/trailer/commit_ok/
      launch_block/is_destructive).
- [ ] `git-commit-format` lint wired in (M5c no-op retired); test-first.
- [ ] `plan_server`: `plan_set_branch`/`plan_record_commit` + commit-time hook (format/trailer
      deny, destructive deny) + `plan_launch` dirty/stale guard; one end-to-end MCP call.
- [ ] Skill teaches branch-on-launch + commit-per-task + trailer + no-destructive; `policy.json`
      git block present.
- [ ] `cargo test -p plan_core -p plan_server` green; clippy clean; `docs/milestones/M7a.md`;
      FORK_DIFF unchanged.

## Open questions (recommendations in parens)
1. **Who creates the branch — UI or agent?** *(The **agent/skill** via Bash, as the single owner
   of git mutation — matches F10.3 "commit-per-task + trailer via the skill" and keeps `plan_ui`
   read-only over git. `plan_launch` returns the computed branch/base; the skill runs
   `git checkout -b` and calls `plan_set_branch`. The UI Launch button (M6a) only transitions to
   `executing`; the strip shows "branch pending" until the skill stamps it. Alternative — the UI
   creates the branch via `git_store` on click — duplicates git-mutation ownership; rejected.)*
2. **Where does git command execution live?** *(`plan_server` (standalone binary, may shell
   `git`); `plan_core` stays **pure** (facts-in predicates); `plan_ui` reads via `git_store`. No
   git shelling in core or ui.)*
3. **Persisted vs live git state.** *(Persist branch/base/worktree/commits in `plan.json`
   (durable identity); read `ahead`/`behind`/`dirty` **live** from `git_store` in the M7b strip —
   the persisted copies in Appendix A's `git` are a best-effort cache, refreshed opportunistically,
   never the source of truth for the strip.)*
4. **`base_for` feature-vs-hotfix.** *(`plan.json` has no plan-`type` field, but Appendix B's
   `base` keys on `feature`/`hotfix`. MVP: always the `feature` base; hotfix detection (a plan
   type signal) is deferred. Flag.)*
5. **Ticketless trailer/format.** *(Ticketless commits check `format_no_ticket` and the trailer
   drops the ticket token → `Plan: rev{rev} task-{task}` (F2.4d "no-ticket commit format").)*
6. **Lease enforcement/reclaim (F11.3b).** *(Enforcement stays **deferred** — unchanged since M6c;
   the PreToolUse hook still can't identify the acting ACP session id. The stale-lease **reclaim**
   UI can ride M7b if cheap; otherwise defer the whole item. Confirm.)*
7. **Is `revert-task-commit` destructive?** *(No — `git revert` creates a *new* commit, so it's a
   direct action (M7b). Only `reset --hard` / `push --force` / `branch -D` / `clean -fd` are
   `amendment_only` (F10.5c).)*
8. **Launch guard: hard block or override?** *(Hard block with a clear reason for MVP — "guards
   block, don't advise" (Part III). Stash/rebase is the developer's move; an override affordance
   is later. Flag.)*

## Heads-up
- **`require_tests_green` (F10.1) can't be verified by the commit hook** — the hook sees the
  command, not the test result. MVP: the **skill** asserts tests-green before committing; the hook
  enforces only commit **format/trailer** + one-commit-per-task structure. True green-gating is
  out of the hook's reach — flag, don't fake it.
- M7a leans hard on the M2 scaffolding (the PreToolUse gate, the tool/hook harness, `policy.json`
  loading). Expect a checkpoint after **T3** (core + enforcement, all test-first) before the skill
  wiring. Git *visuals* stay entirely in **M7b**; don't pull them into M7a — flag if a choice would.
