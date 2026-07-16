# M7b · Git — branch strip + commit rail UI — milestone note

**Status:** code COMPLETE; **§13 visual verification awaiting the developer** (One Dark). `plan_ui`
tests + clippy green; full `zed` build green. This is the review checkpoint — run it and confirm
the strip + rail before M7 (and M7b) is marked done.

**Feature IDs:** F10.2 (branch strip §5/§3.3), F10.3 (commit rail SHAs + foot §8/§3.6), F1.3
(dirty/state chrome). **revert-task-commit omitted by decision** (below).

## What works (built)

- **Live git state (T1)** — `PlanView` reads the active repository from `project::git_store`
  (`Project::active_repository`), subscribing `GitStore`'s `GitStoreEvent` → `cx.notify()` (the
  500ms poll remains a backstop). `git_facts()` reads branch (`Branch::name()`), ahead/behind
  (`Branch::tracking_status()`), and dirty (`status()`, **excluding `.plans/`** to mirror the M7a
  launch guard). No repo → no strip, no panic.
- **Branch strip (T2, §5/§3.3)** — a row under the title, launch-onward: `⎇ branch ← base` chip ·
  `↑n` (created) `↓n` (**amber when behind>0** — drift) · dirty **dot + label** · right-aligned
  `PR —` placeholder. Dirty label via the pure `dirty_label` (task-failed→"task failed"/deleted ·
  executing+dirty→"agent editing"/modified · else "clean"/created) — **unit-tested**.
- **Commit rail closed against §8 (T3)** — node vocabulary completed: hollow pending · **accent
  pulse** in progress · success committed · error failed · amber-hollow gate · **purple amendment
  node** (per-task via `exec::amendment_count`); **rail foot** `▼ base · ↑n ahead · ⑂ PR —`; the
  task-card sha chip shows M7a's `plan_record_commit` SHAs. Spine unchanged (M6c).

## §13 record — decisions & deviations
**Decisions (approved):**
- **Live vs persisted git** — the strip reads **live** from `git_store`; nothing writes
  ahead/behind/dirty back to `plan.json` (no rev churn). `plan.git` stays branch/base/commits only.
- **Dirty excludes `.plans/`** — the plan file is git-versioned and rewritten at launch, so its
  churn must not read as a dirty tree (matches the M7a guard).

**Deviations (flagged, recorded):**
- **Amendment node = purple square, not a 45°-rotated diamond + branch curve.** GPUI rotates only
  svg/img, not divs; the purple square conveys the amendment identity, the exact glyph + connecting
  curve are a fidelity deferral (like M6c's rail-spine note).
- **revert-task-commit (§8 node context menu) omitted.** No `git revert`-a-commit API exists in the
  git surface (`GitRepository` has commit/reset/checkout only; `git_panel`'s revert restores the
  working tree, not a commit). Per the developer: a future revert action **routes through the
  agent** (it runs `git revert`), never `plan_ui` shelling git or upstream git additions. The
  read-only-over-git architecture (M7a decision 2) stands.
- **PR slot shows nothing until a PR exists** (no `PR —` placeholder — dropped per developer
  feedback 2026-07-15; design docs updated). When `plan.git.pr.url` is set, the strip + rail foot
  render a **purple GitHub-icon button** (`PR #n`) that `open_url`s the PR. Populating the PR
  (creation, checks, approvals) stays v1/F10.4.

**§13 protocol (developer, under One Dark):** run `ZED_PLAN=1 cargo run -p zed -- .`; on an
`executing` plan confirm — (1) the **branch strip** shows `⎇ branch ← base`, `↑ ↓` (behind amber),
a dirty dot + label, and **no PR slot** (a GitHub-icon PR button appears only when `git.pr.url` is
set), and is **hidden before launch**; (2) the **commit rail** shows a
node per task (no spine gaps), the in-progress node **pulses**, a committed task shows a filled
node + **sha chip**, an amended task shows the **purple node**, and the **foot** shows base + ahead.

## Fork discipline
Additive — `plan_ui` only, all reads through already-`pub` `project`/`git_store` APIs. **No `pub`
accessor added; no upstream files touched — `FORK_DIFF.md` unchanged.**

## Definition of done
- [x] Live git state via `git_store`, event-driven re-render; degrades without a repo.
- [x] Branch strip per §5/§3.3.
- [x] Commit rail per §8 (node vocab, real SHAs, foot); amendment-node glyph deviation recorded.
- [~] Revert-task-commit — **omitted by decision** (agent-routed if built later).
- [x] `plan_ui` tests + clippy green; full `zed` build green; `docs/milestones/M7b.md`; FORK_DIFF
      unchanged.
- [ ] **§13 visual verification under One Dark — developer gate (pending).**

## Next (after §13 confirms): M7 complete → M8 (tickets) · M9 (settings + hardening).
Deferred into the v1 tail: worktree isolation (F10.2b), PR generation + live PR chip (F10.4),
collision/drift rails (F10.5), git-panel grouping (F10.6), and the agent-routed revert action.
