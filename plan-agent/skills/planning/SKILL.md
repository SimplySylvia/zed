---
name: planning-mode
description: Use when the user asks to plan, draft a plan, or execute work against a plan (e.g. "plan LED-212", "draft a plan", "resume the plan"). Drives structured planning + supervised execution through the plan_* MCP tools, with plan.json as the single source of truth.
---

# Planning mode

You are working against a **plan** — a structured document at `.plans/<id>.plan.json`,
**not** chat scrollback. The plan is the contract; every change goes through the `plan_*`
tools so the file (and the UI reading it) stays authoritative. Never track plan state in
your head or in prose replies — read and write the file.

## Tools (from the plan MCP server)

- `plan_get(id)` — read the plan fresh. **Do this before every task and whenever unsure.**
- `plan_create(id, title, goal, thread)` — start a new draft plan for a thread.
- `plan_update_section(id, section, value)` — edit `spec.goal` / `spec.scope.in` /
  `spec.scope.out` / `design.risks`.
- `plan_add_task(id, task_id, title, system?)` — append a task (starts `pending`).
- `task_update(id, task_id, status?, detail?)` — set a task's status and/or append a
  timeline note (record evidence here).
- `plan_answer_question(id, question_id, answer)` — record an answer to an open question.
- `plan_set_status(id, status)` — advance lifecycle
  (`intake|drafting|in_review|revising|approved|executing|paused|gate|amending|done|abandoned`).
- `plan_launch(id, thread)` — launch an approved plan (→ `executing`) and take the executor
  lease. Returns the plan with `git.branch` / `git.base` **stamped** — the branch you create.
- `plan_set_branch(id, branch, base)` — record the branch/base after you create it.
- `plan_record_commit(id, task, sha, diffstat?)` — record a task's commit for the commit rail.

## Lifecycle

```
intake → drafting → in_review → approved → executing → done
```

Each write bumps `rev` and stamps `history[]` automatically — that's the visible
sync-receipt trail. You do not manage `rev` yourself.

## Intake — clarifying questions first, one at a time (F2.0)

Before drafting, resolve ambiguity. Ask **one question at a time**; wait for the answer;
record it. Prefer a small set of option chips plus "why do you ask?" / "recommend one".

**Non-blocking with stated assumptions (F2.3b):** if the user says "draft now" or skips,
**state a default assumption and continue** rather than stalling. Capture it as an open
question (`plan_answer_question` later) and tag affected blocks so a contradiction triggers
a targeted re-draft. Unanswered assumptions surface as warnings at approve time.

## Drafting

Build the plan in order — Spec → Design → Tasks — using the tools:
1. `plan_create` with the goal.
2. `plan_update_section` for scope in/out and design risks.
3. `plan_add_task` per task; keep tasks dependency-ordered and small (one commit each).
Then `plan_set_status(id, "in_review")` and hand it back for review.

## Launch — create the branch (F10.2)

When the plan is launched (you call `plan_launch`, or the user clicks Launch and the plan
becomes `executing`), **create the git branch before the first task**:
1. `plan_get(id)` and read `git.branch` / `git.base` (launch stamps the intended names).
2. `git checkout -b <git.branch> <git.base>` — cut the branch from its base.
3. `plan_set_branch(id, <branch>, <base>)` to confirm it.

Launch is **guarded**: if the working tree is dirty or the base is stale, `plan_launch` fails
with the reason — commit/stash or update first, don't force past it.

## Execution — the per-task loop

Only after the plan is **`executing`** (launched). For each task, in order:
1. **`plan_get(id)`** — re-read. The plan may have changed under you (user edits, revisions).
   This re-read is mandatory before every task; a hook also injects the current plan.
2. Do exactly what the task's steps say — its declared files, nothing extra.
3. Run the task's tests; **they must pass before you commit** (a red suite means fix or amend,
   never commit).
4. `task_update(id, task_id, "done", "<evidence: test result, commit sha, …>")`.
5. **One commit per task.** Use the task's `commit_message` as the subject and append the
   trailer `Plan: {ticket} rev{rev} task-{task}` (ticketless drops the ticket token). A
   commit-time hook **blocks** a commit whose subject doesn't match the policy format or that
   is missing the trailer. Example:
   `git commit -m "[LED-212]: accept page params in loader" -m "Plan: LED-212 rev5 task-t1"`.
6. `plan_record_commit(id, task_id, <sha>, "<diffstat>")` so the commit shows on the rail.

**Never rewrite history.** `git reset --hard`, `git push --force`, `git branch -D`, and
`git clean -f` are amendment-only (F10.5c) and the hook blocks them — propose an amendment
instead. `git revert` (a new commit) is fine.

If a step is guarded, the run **holds** — you will be blocked by a hook until the guard is
cleared. Do not try to route around it.

## Enforcement (not optional)

Hooks enforce this protocol; they **block**, they don't advise:
- Code edits (`Edit`/`Write`/`MultiEdit`/`NotebookEdit`) are **denied unless a plan is
  `executing`**. Draft and launch a plan before editing code.
- The current plan is re-injected at session start and on each prompt, so you always have it.

If you find yourself about to edit code without an executing plan, stop and draft/launch the
plan first — the edit will be blocked otherwise.

## Failure & drift

Never improvise around a failure. If a task fails, or the ticket/base drifts, propose a
plan change (an amendment) rather than silently doing something different. Record what
happened in the task timeline via `task_update`.
