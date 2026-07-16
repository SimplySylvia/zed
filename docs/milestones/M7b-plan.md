# M7b · Git — branch strip + commit rail UI — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans; the
> **`plan-ui-design` skill** + `docs/design/plan-ui-compliance.md` (**§5 branch strip**, **§8
> commit rail**, §3.3/§3.6 toolbar/strip anatomy, §7 sha chip) — run its **§13 verification
> protocol per component** under One Dark. GPUI views get smoke coverage; the visual check is
> the developer's gate. TDD applies to any pure helper (e.g. dirty-label mapping).

**Status:** COMPLETE — **§13 confirmed by the developer (2026-07-15)**. T1–T3 built + verified
(tests/clippy/build green); **T4 revert-task-commit omitted by decision** (see below); T5 note
written. Closes M7. Second and final slice of M7.

> **T4 decision (developer, 2026-07-15):** revert is naturally the **agent's** job — a future
> "revert task commit" UI action would *kick off the agent* to run `git revert`, never shell git
> from `plan_ui` (option B) or add upstream git surface (option C). Omitted for now; the read-only-
> over-git architecture (M7a decision 2) stands. The §8 "node context menu offers revert" line is a
> recorded deferral.

## Goal
Surface the M7a git state visually: a **branch strip** under the plan title and a **commit rail**
that reads **real repo state** via `project::git_store`, plus **revert-task-commit** on a rail
node. This is the slice with the §13 visual verification you'll review.

**Feature IDs:** F10.2 (branch strip §9.1/§3.3), F10.3 (commit rail SHAs + revert §9.1/§3.6),
F1.3 (dirty/state chrome). MVP visuals only — PR live chip (F10.4), worktree badges (F10.2b),
and collision/drift banners (F10.5) are **v1**, out of scope (PR slot renders the placeholder).

**Architecture:** `plan_ui` **reads** git state, never mutates plan.json for it. `PlanView`
already holds a `WeakEntity<Workspace>`; reach the repo via `workspace → Project → git_store`
(zed-notes study #7: `RepositorySnapshot` + `RepositoryEvent` are observable, no upstream
patch). The rail's per-task nodes/SHAs come from `plan.git.commits` / task `artifacts` (M7a
populates these via `plan_record_commit`); branch/ahead/behind/dirty are **live-read** from the
active repository, not the persisted cache (M7a decision 3). Revert is a *new* commit
(non-destructive), so it's a direct action (M7a q7).

**Tech stack:** Rust + GPUI (`plan_ui` only). Commit prefix `[PLAN-M7b]`. Additive — expect
`FORK_DIFF.md` unchanged (all reads go through already-public `project`/`git_store` APIs; if an
accessor turns out non-`pub`, add the minimal one and record it — don't restructure upstream).

---

## Tasks

### T1 — Live git state in `PlanView` (via `project::git_store`)
`[PLAN-M7b]: read live git repository state into the Plan tab`
- From `added_to_workspace`/init, grab `workspace.project().read(cx)` → the active `Repository`
  entity from `git_store`; `cx.subscribe` its `RepositoryEvent::{StatusesChanged, HeadChanged,
  BranchListChanged}` (and `GitStoreEvent::ActiveRepositoryChanged`) → `cx.notify()`. Store the
  subscription alongside the existing `_agent_subscription`/`_watch_task`.
- Read into a small `GitStrip` view-struct on render: `branch` (`snapshot.branch`), `base` (from
  `plan.git.base`), `ahead`/`behind` (`Branch::tracking_status()`), `dirty`
  (`statuses_by_path`, excluding `.plans/` to match the M7a launch guard). **Confirm the exact
  accessors when building** (as M6a did) — the study notes name them but line numbers drift.
- **Smoke:** the tab renders with and without a repository present (degrade cleanly — no repo →
  no strip, no panic).

### T2 — Branch strip (compliance §5 / spec §3.3) [F10.2]
`[PLAN-M7b]: add the branch strip under the plan title`
- A row under the title, **rendered only from Launch onward** (`executing|paused|gate|amending`,
  same predicate as the rail): `⎇ branch ← base` chip (mono, editor bg) · `↑n ↓n`
  (ahead `created` / behind `text_placeholder`; **behind>0 → amber `modified`** drift signal) ·
  **dirty dot + label** · right-aligned **PR slot** placeholder `PR —` (`text_placeholder`).
- Dirty-label mapping (a pure helper, TDD): any task `failed` → `deleted` "task failed"; else
  executing + dirty → `modified` "agent editing"; else `created` "clean" (q2).
- All colors via `cx.theme()`; purple identity via `syntax()` keyword (per M3 mapping).
- **Acceptance:** on an executing plan the strip shows branch←base + ↑↓ + dirty + `PR —`; behind
  turns amber; hidden before launch. §13 §5/§3.3 under One Dark.

### T3 — Commit rail completion (compliance §8 / spec §3.6)
`[PLAN-M7b]: complete the commit rail — real SHAs, foot, and node vocabulary`
- Audit `rail_node` (`plan_view.rs`) against the §8 vocabulary and fill gaps: hollow=pending ·
  success=committed · accent-pulse=in progress · error=failed · **amber hollow=gate** · **purple
  45°-rotated square + branch curve = amendment**. (The spine + base nodes exist from the M6c
  fidelity pass; this closes the vocab.)
- The task-card **sha chip** (`⌥ sha +a −d`, already reading `artifacts.sha`) now shows the real
  M7a-recorded commit; confirm diffstat coloring.
- **Rail foot** (§8): `▼ base · ↑n ahead` (+ `⑂ PR #…` slot placeholder). Base + ahead from the
  T1 live state.
- **Acceptance:** every visible task row has a node (no spine gaps); a committed task shows a
  filled node + sha chip from the recorded commit; foot shows base + ahead. §13 §8.

### T4 — Revert-task-commit (node context menu)
`[PLAN-M7b]: add revert-task-commit to the rail node context menu`
- Right-click a rail node → context menu `Revert task commit` (copy the `ContextMenu` +
  right-click pattern from `git_ui`). Action runs `git revert <sha>` for the task's recorded
  commit via the `git_store`/`Repository` API (revert = new commit → non-destructive → allowed;
  reset/force-push stay amendment-only, enforced server-side in M7a).
- **Open investigation (q3):** confirm `git_store`/`Repository` exposes a revert (or a commit-
  creating path) usable from `plan_ui`. If it does not, deliver the menu item wired to the
  closest available path and **flag** any gap rather than shelling git from `plan_ui` (which
  would break the read-only-over-git architecture) — stop and ask before improvising.
- **Acceptance:** right-clicking a committed node offers Revert; invoking it reverts that commit.
  §13 (interaction).

### T5 — Milestone note + §13 + FORK_DIFF
`[PLAN-M7b]: add M7b milestone note`
`docs/milestones/M7b.md` (what works / deferred / any accessor recorded); FORK_DIFF updated only
if a `pub` accessor was added; full `plan_ui` + `zed` build; **§13 visual verification handoff to
the developer** (the milestone gate). Optionally, if cheap, the **stale-lease reclaim** button
(F11.3b UI half) rides here — else explicitly defer.

---

## Definition of done
- [ ] Live git state read via `git_store` with event-driven re-render; degrades without a repo.
- [ ] Branch strip per §5/§3.3 (launch-onward, behind-amber, dirty label, PR placeholder).
- [ ] Commit rail per §8 (node vocabulary complete, real SHAs, foot base+ahead, no spine gaps).
- [ ] Revert-task-commit on the node context menu (or a flagged gap with the reason).
- [ ] Smoke tests + pure-helper TDD green; clippy clean; full `zed` build; `docs/milestones/M7b.md`.
- [ ] **§13 visual verification under One Dark — developer gate** (this is the review checkpoint).

## Open questions (recommendations in parens)
1. **Which repository does the strip read?** *(The **active** repository from `git_store`. During
   execution HEAD should be the plan's branch — the skill creates it at launch. If the checked-out
   branch ≠ `plan.git.branch`, MVP still shows live state; a branch-mismatch indicator is later.
   Flag.)*
2. **Dirty-dot label source.** *(task-failed → "task failed" (deleted); else executing+dirty →
   "agent editing" (modified); else "clean" (created). A pure mapping helper, unit-tested.)*
3. **Does `git_store` expose a revert usable from `plan_ui`?** *(Investigate at T4. If yes, wire
   it. If no clean path exists, deliver the menu item against the closest API and flag the gap —
   do **not** shell git from `plan_ui` (breaks read-only-over-git). Stop and ask.)*
4. **Live vs persisted git state.** *(Strip reads **live** from `git_store`; do not write
   ahead/behind/dirty back to plan.json (avoids rev churn / write loops). The persisted `git`
   block stays branch/base/commits only, stamped server-side in M7a.)*

## Heads-up
Most of the rail chassis already exists (M6c fidelity pass) — T3 is an audit-and-close, not a
rebuild. The genuinely new surface is the **branch strip** (T1+T2) and **revert** (T4). Keep the
PR chip a placeholder (live PR is v1/F10.4); don't pull collision/drift banners (v1/F10.5) — flag
if a choice would. Expect a checkpoint after **T2** (live read + strip visible) before the rail
audit + revert.
