# M6c · Execution — amendments + failure ladder + control — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS — superpowers:executing-plans; superpowers:test-driven-development
> (control + amendment helpers are test-first, reusing `plan_core::rev`); the `plan-ui-design` skill
> + `plan-ui-compliance.md` (§6 amendment card, §3 toolbar controls, §9 paused/failed matrix rows).

**Status:** awaiting sign-off (not started). Final slice of M6 (M6a launch + M6b guards done).

**Feature IDs:** F4.7 (amendments — failures/blockers/drift → a staged plan change, never
improvisation), F11.4 (failure ladder — 2 amendments on one task → escalation card), F5.1
(pause/resume/stop), F5.6 (kill switch), F11.1 (interrupted-task recovery). Closes M6.

**Goal:** when a task fails, the agent proposes an **amendment** — the same staged-diff primitive
as M5b (`plan_core::rev`), rendered during execution as an **amendment card** (Accept/Reject);
two amendments on one task raises an **escalation card** (manual takeover / descope+waiver / guide
me). The developer can **pause / resume / stop** execution (and kill) from the toolbar/panel/pill;
an interrupted task offers a **recovery** choice (resume / redo / keep-as-manual).

**Architecture:** unchanged. Control + amendment logic are pure `plan_core::exec` helpers reusing
`plan_core::rev` for the staged diff. Amendments are `pending_revision` proposed while
`executing` — the UI renders them as an amendment card (err/amendment header) vs M5b's info
staged-rev card. The UI writes `plan.json` directly; the agent proposes via the server.

**Tech stack:** Rust — `plan_core::exec` (pause/resume/stop + amendment tag/count + recovery),
`plan_server` (`plan_propose_amendment` + control tools + Stop loop guard), `plan_ui` (amendment
+ escalation + recovery cards, control buttons). Pure logic test-first; GPUI = smoke + §13.
Commit prefix `[PLAN-M6c]`.

---

## Tasks

### T1 — `plan_core::exec` control + amendment helpers (TDD)
`[PLAN-M6c]: add pause/resume/stop and amendment helpers to plan_core`
- **Control:** `pause(plan)` (`executing → paused`, keep lease), `resume(plan)` (`paused →
  executing`), `stop(plan)` (`→ paused` + `release_lease`, a halt resumable via launch/resume).
  Kill (F5.6) = `stop` for MVP. Errors on illegal transitions.
- **Amendments (F4.7):** `propose_amendment(plan, task, hunks)` — stage a `pending_revision` via
  `rev::stage_revision` and **tag it an amendment** (marker so the UI/apply path distinguishes it
  from an in-review staged revision; q1) + record the amendment against `task` for the ladder.
  Apply/reject reuse `rev::apply_hunk`/`reject_hunk`/`resolve_revision`.
- **Failure ladder (F11.4):** `amendment_count(plan, task) -> usize` (count from history/timeline;
  q2) and `needs_escalation(plan, task) -> bool` (`>= 2`).
- **Recovery (F11.1, lean):** `mark_interrupted(plan, task)` (`→ interrupted`) and a recovery
  transition helper `recover_task(plan, task, choice)` where choice ∈ resume|redo|manual (sets
  `in_progress` / `pending` / `manual=true`+skipped). Death *detection* is deferred (q3).
- **Tests:** pause/resume/stop transitions (+ illegal-transition errors, lease released on stop);
  propose_amendment stages a tagged pending_revision + bumps the task's amendment count;
  needs_escalation flips at 2; recover_task choices set the right task state.

### T2 — `plan_server` amendment + control tools + Stop loop guard (TDD)
`[PLAN-M6c]: add amendment, control, and Stop-guard tools`
- Tools reusing T1: `plan_propose_amendment(id, task, hunks)`, `plan_pause`, `plan_resume`,
  `plan_stop`, `plan_recover_task(id, task, choice)`.
- **Stop loop guard (F6.3, lean):** the `Stop` hook (currently a placeholder) nudges the agent to
  continue while the plan is `executing` with unfinished, unheld tasks, with a runaway cap (don't
  nudge more than N times) (q4).
- **Tests:** pause→resume→stop over the tools (status + lease); propose_amendment persists a
  tagged pending_revision; Stop guard nudges while work remains and stops at the cap / when paused;
  one end-to-end MCP call.

### T3 — Amendment + escalation + recovery cards + control buttons (compliance §6/§3/§9) [M6c]
`[PLAN-M6c]: add amendment cards, escalation, recovery, and controls to the Plan tab`
- **Amendment card (§6):** render `pending_revision` while `executing` as an amendment card (err
  header `◆ amendment · rev n`, +new/~changed lines, **Accept / Reject** reusing the M5b apply
  path). (M5b's info staged-rev card still renders in review.)
- **Escalation card (§6/F11.4):** when `needs_escalation(task)`, an err card with the three
  options (manual takeover / descope+waiver / guide me) → the matching `recover_task` / waiver.
- **Recovery card (F11.1):** an `interrupted` task renders resume / redo / keep-as-manual.
- **Controls (§3/§9):** toolbar primary + actions by state — `executing → ⏸ Pause`,
  `paused → ▶ Resume` (+ `⏹ Stop`), with a kill affordance; the pill/panel reflect
  paused/failed per the §9 matrix.
- **Acceptance:** a failed task with a proposed amendment shows the amendment card → Accept
  applies it (rev bump); a second amendment raises the escalation card; Pause/Resume/Stop flip the
  status + chrome; an interrupted task offers recovery. §13 under One Dark.

### T4 — Milestone note + FORK_DIFF + §13
`[PLAN-M6c]: add M6c milestone note`
`docs/milestones/M6c.md`; FORK_DIFF unchanged (additive); full `plan_ui` + `zed` build; §13
handoff. **Closes M6 (execution)** — the note should say so and point at M7 (git).

---

## Definition of done
- [ ] `plan_core::exec` control + amendment + recovery helpers correct, test-first (transitions,
      amendment count/escalation, recovery choices).
- [ ] Amendment/control/recover tools + Stop loop guard present; one end-to-end MCP call.
- [ ] Tab: amendment card (Accept/Reject) → applied; escalation at 2; recovery on interrupted;
      Pause/Resume/Stop controls + paused/failed chrome.
- [ ] Builds green; clippy clean; pure logic test-first; `docs/milestones/M6c.md`; FORK_DIFF unchanged.
- [ ] §13 visual verification under One Dark — developer gate.

## Open questions (recommendations in parens)
1. **Amendment vs staged-revision distinction.** *(Tag it: set a marker on `pending_revision`
   (e.g. `pending_revision.kind = "amendment"`, an `extra` field) so the UI picks the amendment
   card vs the review staged-rev card. Alternative — infer "amendment" purely from
   `status == executing` — is simpler but conflates the two if a revision lands mid-execution.
   Rec: explicit marker.)*
2. **Failure-ladder counter home.** *(Count amendment history/timeline entries referencing the
   task — no schema change. Rec: history `kind:"amendment"` with the task in the summary/entry;
   `amendment_count` scans it. Alternative: a dedicated `task.amendments` field — cleaner but a
   schema add. Rec: history-scan for MVP, flag the field option.)*
3. **Interrupted-task recovery depth (F11.1).** *(Lean: build the recovery **card + transitions**
   (resume/redo/keep-manual); **defer agent-death detection** (needs ACP disconnect wiring). The
   agent/skill marks a task `interrupted` on failure for now. Rec: lean + flag.)*
4. **Stop loop guard (F6.3).** *(Lean: nudge-continue while `executing` with unfinished unheld
   tasks, capped at N nudges; a paused/holding plan stops. Rec: lean; full stuck-detection
   (F11.4b) is v1.)*
5. **Lease enforcement + reclaim (F11.3b).** *(**Defer** — the PreToolUse hook can't reliably
   identify the acting ACP session id yet, so leaseholder-only editing + stale-lease
   reclaim-with-confirm wait for ACP session context in the hook. The lease is set (M6a) and
   released on stop; that's the M6 scope. Rec: defer, flag.)*
6. **Kill vs stop vs abandon.** *(MVP: kill == stop (hard halt → paused + lease released,
   resumable). Abandon (F11.7 — revert/keep/keep-manual + archive) is its own later concern. Rec.)*

## Heads-up
Final M6 slice and broad — the fully-solid core is **amendments (reusing `rev`) + the failure
ladder + pause/resume/stop**. Recovery (q3), the Stop guard (q4), and especially lease
enforcement/reclaim (q5) are scoped lean or deferred with flags to keep the slice tractable.
Expect a checkpoint after T2 before the UI. Git stays in M7.
