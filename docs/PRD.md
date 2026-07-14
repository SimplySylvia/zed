# Plan — Product Requirements & Build Plan (handoff v1.0)

**Audience: Claude Code.** This is the single source of truth for building the "Plan"
feature into a personal Zed fork. Part 0 gives context, Part I is the complete product
requirements (spec v1.0), Part II is the build plan, Part III is the working agreement
for how you (Claude Code) should execute. Read Part 0 and Part III fully before
starting; treat Part I as the reference you return to constantly (feature IDs like
F4.5b are the shared vocabulary); execute Part II milestone by milestone.

---

# Part 0 — Context & environment

## What we're building
"Plan" is a first-party planning + supervised-execution surface for agentic coding,
built into a personal fork of the Zed editor. The agent drafts a structured plan; the
developer reviews it with editor-grade feedback tools (comments, suggested edits,
alternatives, staged revisions); the plan executes task-by-task with live tracking,
evidence, gates, and step-level guards; every agent deviation becomes a visible,
approvable diff. The canonical state is a JSON file in the repo
(`.plans/<id>.plan.json`) — the UI, the agent, and recovery all derive from it.

Each **thread has exactly 0 or 1 plan** (F1.0). The Plan tab/panel is a singleton that
follows the active thread. The feature is called **Plan** (singular); "Plans" names
only the cross-thread browser.

## Environment facts
- **Editor**: personal fork of Zed (Rust, ~231 crates, GPUI UI framework). Fork
  discipline: additive crates only; upstream diff target is ~2 registration lines
  (tracked in `FORK_DIFF.md`); weekly upstream merges.
- **Agent**: Claude Code, connected to Zed via the `claude-code-acp` adapter
  (ACP = Agent Client Protocol). MCP servers register through Zed's
  `context_servers` config.
- **New code**:
  - `crates/plan_core` — Rust, no GPUI. Schema, atomic IO, watch, lint, anchoring.
  - `crates/plan_ui` — Rust + GPUI. All surfaces.
  - `plan-server/` — **TypeScript** MCP stdio server (`@modelcontextprotocol/sdk`).
  - `plan-agent/` — planning SKILL.md + Claude Code hooks + settings fragment.
- **Developer profile**: TypeScript-primary (React/React Router/Node), competent in
  Rust but not fluent in GPUI — prefer copying patterns from existing Zed crates
  (`git_ui`, `terminal_view`, `agent_ui`) over inventing.
- **Design truth**: interactive HTML artifacts accompany this doc —
  `planning-mode-demo-e2e-pagination.html` (13-state end-to-end walkthrough; the
  primary UI reference) and mockups v1–v9 (per-surface detail: v5 lifecycle, v6 review
  flow, v7 enhancements, v8 git visuals, v9 settings). They encode exact layouts,
  copy, and Zed One Dark token usage. **Part IV captures this design as text** —
  component anatomy, states, theme-role mapping, and the lifecycle→surface matrix —
  so the HTML is corroboration, not required reading. **In the real build, colors
  come from `cx.theme().colors()`** — hex values are the One Dark reference, not
  constants to hardcode.

## Glossary (quick)
- **Plan / plan.json** — the durable contract; three *lenses* over it: Spec, Design,
  Tasks. **rev** increments on every accepted change.
- **GATE task** vs **step guard** — a GATE pauses between tasks for sign-off with
  evidence (F4.5); a guard holds *inside* a task at one step, either ⛨ approval or
  ✋ input (F4.5b). Both hook-enforced, neither expires.
- **Staged revision** — agent-proposed plan change delivered as a pending diff with
  per-hunk apply/reject (F9.3). **Amendment** — the same primitive during execution
  (failure/drift response, F4.7).
- **Pushback** — agent may disagree with review feedback and hold until you pick
  [Change anyway] / [Keep as planned] (F3.4c).
- **Coverage** — every ticket AC maps to a plan acceptance criterion; hard-checked at
  Done (F2.4f). **Drift** — ticket or git base changed under us (F2.4g, F10.5).
- **Sync receipt** — visible proof of which rev the agent last read (F5.3b).
- **Lease** — one thread executes a plan at a time (F11.3).

---

# Part I — Product requirements (spec v1.0)

**Naming convention (resolved):** the feature is **Plan** — singular and thread-scoped.
Every thread has zero or one plan; the Plan surface always shows *the active thread's*
plan, so it's "the Plan," never "a plan tab among many." **"Plans"** appears exactly
once in the product: the browser/archive (F1.6) where you look across threads and
history. The artifact stays `plan.json`; tools stay `plan_*`. ("Plan Mode" is retired —
it's not a mode, it's a facet of the thread.)

A first-party planning surface for agentic coding in a personal Zed fork. The agent
drafts a structured plan; you review it with editor-grade feedback tools; it executes
task-by-task with live tracking, evidence, and gates; the plan stays the source of truth
through pauses, edits, failures, and multi-day gaps.

Validated across mockups v1–v9 and the end-to-end demo. Feature IDs are stable from the
working drafts (v0.1–v0.9).

---

## 0. Principles

1. **The plan is a document, not chat scrollback.** A real buffer in a real tab —
   selectable, editable, diffable, persistent.
2. **plan.json is canonical.** Structured file at `.plans/<id>.plan.json`, in-repo and
   git-versioned. Everything renders from it; everything recovers from it.
3. **The agent obeys the document.** Enforced, not hoped: skill + hooks + MCP tools +
   re-read-before-every-task. And *observably* enforced (sync receipts).
4. **Every agent deviation is a visible, approvable change.** Revisions, amendments,
   rebases, conflict resolutions — all one "proposed diff" primitive. Nothing silent.
5. **One attention system.** Blockers, lint findings, rehearsal mismatches, gates,
   pushbacks, drift — different sources, one "what's open?" queue.
6. **Theme-native, Zed-native.** Theme tokens, existing panel/tab machinery, familiar
   interaction patterns (Keep/Reject, peek, right-click docking). Tiny fork diff:
   isolated `plan_ui` crate + registration lines; weekly upstream merges.
7. **Tickets are a contract, not decoration** — and always optional.

## 1. Lifecycle

```
intake → drafting → in_review ⇄ revising → approved → executing ⇄ paused
                                                  │→ gate (per task) → executing
                                                  │→ amending (failure/drift) → executing
                                                  └→ done | abandoned
```
Plan status drives surface defaults (lens, tab auto-open, pill state). Terminal plans
archive with full history.

## 2. Surfaces

- **F1.0 [MVP] One thread, one plan.** A plan belongs to exactly one thread (0..1 per
  thread); `plan.json` carries the `thread` binding, the executor lease is the same
  thread, and archives key by it. Multi-ticket work is one thread + one plan (§4).
- **F1.1 [MVP] Plan tab — a thread-following singleton.** There is one Plan view, not a
  tab per plan: selecting a thread in the sidebar **retargets the Plan tab and panel to
  that thread's plan** (exactly how the agent panel follows the active thread). Tab
  title reads `Plan — LED-212`; status dot = that plan's lifecycle state. Threads
  without a plan show an empty state ("no plan — ask the agent to draft one").
  Implemented as `Item` + `SerializableItem` keyed by workspace, restoring the
  active-thread binding on relaunch. **[v1] F1.1b Pin to compare:** a pinned copy stops
  following (for side-by-side of two threads' plans in a split); the unpinned singleton
  keeps following.
- **F1.3 [MVP] Plan panel.** Compact dock panel (default bottom; right-click to move),
  following the active thread like the tab:
  pipeline column (task rows + acceptance dots) and live column (activity feed /
  gate evidence / failure info / paused state). Header: status, progress bar, sync
  receipt, primary actions, Open as tab.
- **F1.4 [MVP] Status-bar pill.** `◆ Plan 2/5 ▓▓░ acc 1/6` — the *active thread's*
  plan; toggles the panel.
  **F5.7 [v1] Attention queue:** with items pending it shows "◆ 2 need you" and clicking
  cycles question → blocker → pushback → gate → mismatch across all plans.
- **F1.5 [MVP] Threads-sidebar integration.** Thread rows carry plan state ("plan 2/5",
  "gate — needs you") and worktree badges (`⌂ wt/led-212`) for parallel awareness.
- **F1.6 [v1] "Plans" browser.** The one plural surface: actives + archive across all
  threads, searchable; blame/peek entry points.
- **F12.2c [MVP] Settings page.** "Plan" section in Zed's settings editor (see §13).

## 3. The plan document

### 3.1 Three lenses over one plan.json
- **Spec** — goal · tickets (0..n, live) · scope in/out · acceptance criteria
  (WHEN/SHALL, EARS-style) · open questions.
- **Design** — architecture flow · contracts · UI previews · decisions/rationale · risks.
- **Tasks** — dependency-ordered task cards: steps, files (C/M badges), preview block,
  per-task acceptance, ticket chip, commit chip, system badge (Frontend/Backend/
  Integration/Testing), GATE badge.

**F1.2b [MVP] Preview blocks** (from the ORC-494/495 pattern): typed blocks —
`data-shape`, `contract` (input→output tables), `ui-states` (mini rendered states),
`before-after`, `events`, `evidence`, `result` — attachable to tasks and Design.
**F0.5b [v1] Peek cards:** hover/⌥-hover any cross-reference chip to peek the target
inline (criterion, diff, thread); Esc dismisses, click jumps.

### 3.2 Rendering approach
Blocks render as **read-only Zed mini-buffers** (multibuffer machinery) — selection,
keybindings, and diff rendering come free; interactive chrome (checkboxes, chips,
buttons) is GPUI around them.

## 4. Tickets (optional, 0..n per plan)

- **F2.4 [MVP] Plan-from-ticket(s).** "plan LED-212" → agent pulls via Jira MCP
  (tracker-agnostic `tickets[]` shape), seeds goal + drafts acceptance from ticket AC.
  Multi-ticket single plans replace epic machinery.
- **F2.4b [MVP] Rendering.** Header chips + Spec-lens cards (type, status, priority,
  assignee, link) with **live status** and a **coverage meter**.
- **F2.4c [MVP] Task↔ticket mapping.** `task.ticket` drives commit prefix, trailer, and
  per-ticket PR grouping.
- **F2.4d [MVP] Ticketless + attach-any-time.** `tickets: []` degrades cleanly (slug
  branch, no-ticket commit format). Attaching later backfills — retags prior commits via
  rebase (force-push of pushed branches rides the amendment flow).
- **F2.4f [MVP] Ticket coverage.** Every ticket AC maps to a plan criterion
  (`acceptance[].ticket_ac`); uncovered items are lint findings at draft and a **hard
  check at Done** (explicit recorded waivers only). Descoping is a decision, never an
  omission.
- **F2.4g [MVP] Drift & resync (agent-driven).** Snapshot at `fetched_at`; agent
  refetches at session start / plan open / before gates / before Done. Changes render a
  drift card; AC changes re-run coverage; scope changes arrive as a staged revision.
  Manual ↻ Resync on the card.
- **F2.4e [v1] Write-back.** Launch → In Progress; Done → summary + PR link comment;
  Abandon → reason. Always visible in the feed.

## 5. Drafting & intake

- **F2.0 [MVP] Clarifying questions first — one at a time.** Questions present
  sequentially (progress indicator; upcoming questions queued and dimmed), each as
  option chips plus an escape chip ("Why do you ask?" / "Recommend one"). **Selecting an
  option reveals an optional context field** — free text that rides with the answer into
  the plan record and becomes real material (scope, risks, extra steps), not chat
  ephemera. Confirm advances; per-question skip and a "Draft now — use stated defaults"
  action exit early onto F2.3b assumptions; answering in plain chat always works.
  Answers + context persist in Spec → open questions.
- **F2.1 [MVP] Streaming draft.** Spec → Design → Tasks via MCP writes; skeleton shimmer
  for unwritten blocks; readable mid-draft.
- **F2.3 [MVP] Open questions** as structured items answered inline.
  **F2.3b [MVP] Non-blocking with stated assumptions:** the agent states a default and
  continues; affected blocks carry "assumes Q1" tags; contradiction triggers targeted
  re-draft; unanswered assumptions surface as ⚠ at Approve.
- **F9.1 [MVP] Plan lint** runs on every draft/revision before you read: rules from
  `.plans/policy.json` — every-task-has-tests · criteria-link-tasks · files-must-exist ·
  prod-requires-gate · max-files-per-task · ticket-coverage · git format rules (§9).
  Agent auto-fixes what it can; the rest are auto-flags in the comment system
  (author: plan-lint). Blocker-severity findings gate Approve.
- **F3.5b [v1] Quick-plan** for small tasks per policy eligibility (gate-free pass).

## 6. Review loop

### 6.1 Selection & anchors
- **F3.1 [MVP]** Select anything — text ranges or whole blocks (step, task, criterion,
  contract row, preview). Anchor: `{lens, block, range, quote}`; fuzzy re-anchor across
  revisions; **outdated** badge preserves the original quote when text was rewritten.

### 6.2 Feedback actions (floating toolbar)
- **F3.2a [MVP] Comment ⌘⇧M** — threaded discussion; agent replies/revises.
- **F3.2b [MVP] Suggest edit ⌘⇧E** — you write the replacement; tracked change; agent
  applies verbatim or adapts, and **adaptations must be visible replies**.
- **F3.2c [MVP] Alternatives ⇄** — agent returns 2–3 option cards with trade-offs; your
  pick applies on revision.
- **F3.2d [MVP] Quick flag ⚑**; **F3.2e [v1] task scope actions** (split, descope,
  reorder, add GATE, "I'll do this manually"); **F3.2f [v1] Ask**.
- **F9.2 [v1] Code-anchored feedback:** select code in any buffer → "Attach to plan
  comment"; snippet + file:line ride the anchor. Symmetric standard: agent claims about
  the codebase cite clickable `file:line` chips.

### 6.3 Severity, batch, revision
- **F3.3 [MVP]** ⚑ Blocker (gates Approve) / ⚠ Concern / 💡 Idea. Toolbar blocker chip;
  pill carries counts.
- **F3.4 [MVP] Batch → one revision pass** ("Send for revision · n"; ⌘⏎ sends one).
  Agent: list → reply per item → change → mark addressed; one rev bump per pass.
- **F3.4c [MVP] Pushback.** The agent may disagree with reasoning and
  **[Change anyway] [Keep as planned]**; partial pushback allowed; item stays unresolved
  until you decide. **F3.4d:** resolution belongs to the user (accepted suggestions
  auto-resolve).
- **F9.3 [MVP] Staged revisions.** Default: revisions land as a **pending diff** with
  per-hunk Apply/Reject (mirrors Zed's Keep/Reject). Per-plan/per-agent setting:
  staged | auto_apply. Amendments are always staged. One "proposed diff" primitive
  (`plan_propose_revision`).
- **F3.5 [MVP] Change traceability.** Rev card with old→new per change + caused-by chips
  (c1/s1/a1-B); inline Δ chips on changed blocks link back to threads.
- **F3.6 [MVP] Approve** integrates ExitPlanMode; enabled only at zero open blockers.
- **F3.7 [MVP] Direct edit always works** (edit = know exactly; suggest = sanity-check
  my wording; comment = agent figures it out; alternatives = show options).

## 7. Execution

- **F4.1 [MVP] Launch** = branch/worktree setup (§9) + lease (§12.3) + per-task loop:
  `plan_get` → task → tests → commit → `task_update` → acceptance evidence.
- **F9.4 [v1] Rehearsal** (post-approve, default for large plans): narration-only trace;
  predicted files/commands diffed against declarations; mismatches block Launch until
  added-to-plan or ignored.
- **F4.3 [MVP] Active-task spotlight** + live activity feed (panel + collapsed under the
  task). **F4.3b [v1]** timelines persist per task; **F4.3c [v1]** "While you were away"
  digest on refocus.
- **F4.6 [v1] Acceptance auto-check** with evidence chips.
  **F4.6d [v1] Live evidence:** test chips re-run on click; commit chips diffstat
  popover; screenshots thumbnail; **staleness** (amber when a referenced file changed
  after capture); **Re-verify all** at gates/Done.
- **F4.5 [MVP] GATE tasks** pause execution for sign-off with collected evidence.
  Gates never expire (F11.2c).
- **F4.5b [MVP] Step-level guards.** Individual steps can be safety-guarded, two kinds:
  **⛨ approval** ("about to run the local DB migration — approve to run") and **✋ input**
  ("verify the migration locally and record what you saw"). Guard badges render on the
  step from draft onward — reviewable, addable/removable like any plan content — and the
  task header shows a guard-count chip. At execution the run **holds at the step**: badge
  pulses, panel + pill flip to needs-you, and for input guards an inline field captures
  your response, which is stored on the step and attached as evidence to linked criteria.
  Enforcement is the PreToolUse hook — the agent is blocked, not asked nicely; cleared
  guards leave receipts (who/when/what ran). Policy can force guards onto matching
  commands (e.g. anything matching `migrate|seed|drop`). Like gates, guards never expire,
  and state survives editor restarts (F11.1b).
- **F4.7 [MVP] Amendments.** Failures/blockers/drift → proposed plan change (staged
  diff), never improvisation. **F11.4 [MVP] failure ladder:** 2 amendments on one task →
  escalation card (manual takeover / descope+waiver / guide me). **F11.4b [v1]** stuck
  detection.

## 8. Control & sync

- **F5.1 [MVP] Pause / resume / stop** everywhere (toolbar, panel, pill menu).
- **F5.2 [MVP] Edit while paused (or live).** Edits rev-bump; agent re-reads before the
  next task. **F5.3 [MVP] re-sync guarantee** via skill + per-turn hook injection +
  PreToolUse code-edit gate. **F5.3b [MVP] sync receipts:** "agent synced rev 4 · 12:04";
  amber "1 rev behind — syncs before next task"; **Sync now**.
- **F5.6 [MVP] Kill switch.**

## 9. Git integration

- **F10.1 [MVP] Git lint** from policy: branch pattern (+ticketless fallback), base per
  plan type, commit format (+`format_no_ticket`), one-commit-per-task,
  tests-green-before-commit. Checked at draft / commit-time hook / pre-PR.
- **F10.2 [MVP] Launch creates the branch;** guards for dirty tree/stale base.
  **F10.2b [v1] Worktree isolation** `mode: auto` — first plan runs in the main tree;
  isolation kicks in when a second launches (root per policy).
- **F10.3 [MVP] Task-scoped commits** + trailer `Plan: LED-212 rev5 task-3` →
  task-scoped revert, log→plan traceability, blame integration.
- **F10.4 [v1] PR generation:** title/body from plan (Spec summary + acceptance with
  evidence + file tree + rev history), CODEOWNERS reviewers, body re-sync on amendment.
  **F10.4c [deferred v2+] stacked PRs/epics** — multi-ticket single plans cover the
  need.
- **F10.5 [v1] Collision & drift rails:** cross-plan file collision at launch (+runtime
  v1); upstream drift pauses and proposes rebase as an amendment; conflicts render as
  resolution cards (accept / edit in buffer / abort). **F10.5c [MVP]**
  `destructive_ops: amendment_only` — always.
- **F10.6 [v1] Git panel grouping; [v2] blame→plan peek.**

### 9.1 Git visuals (mockups v8)
**Branch header strip** under the title: `⎇ branch ← base · ⌂ worktree · ↑n ↓n · ● state
· PR chip` (amber ↓ = drift, red ● = conflicts, PR chip live with checks/approvals).
**Commit rail:** git-graph gutter aligned to tasks — filled node per commit (SHA +
diffstat chip on the task), ◆ diamond branching for amendments, hollow ⏸ for gates;
right-click node → revert task commit; rail foot shows base + ahead. **Worktree badges**
in thread rows. Drift/conflict banners + cards per §7 amendments.

## 10. Agent integration

- **F6.1 [MVP] Skill** (`planning-mode`): lifecycle protocol, question policy,
  revision/pushback protocol, per-task loop, evidence rules, references (templates,
  standing rules). **F9.5 [v1.5] Distillation:** recurring feedback (≥3 similar) →
  offered as a standing rule with provenance; mechanically checkable rules promote to
  lint one-click.
- **F6.2 [MVP] MCP server** (`zed-plan-server`, stdio, registered as a context server).
  Tools: `plan_create · plan_get · plan_update_section · plan_add_task · task_update ·
  plan_set_status · plan_list_comments · plan_reply_comment(action) ·
  plan_apply_suggestion · plan_add_alternatives · plan_mark_addressed ·
  plan_answer_question · plan_propose_revision(hunks) · plan_rehearse · plan_lint`.
  Suggestions/alternatives/flags are `kind` variants on the comment object.
- **F6.3 [MVP] Hooks:** SessionStart (plan + ticket resync + resume briefing),
  UserPromptSubmit/PreCompact (re-inject current plan), PreToolUse (block code edits
  without an executing plan; enforce commit format; file-list scope), Stop (loop guard).
- **F6.4 [MVP] ACP bridge:** TodoWrite/plan updates reconciled to plan.json task status
  for native session-update rendering.

## 11. Persistence

`.plans/<id>.plan.json` (+ `.plans/policy.json`, `.plans/.bak/`). Atomic writes,
`schema_version` + forward migration, unknown fields preserved. Archive on terminal
status. Schema: see Appendix A.

## 12. Failure & edge states

Principles: plan.json is the recovery point; every interruption is a visible timeline
event; nothing destructive auto-resumes.

- **F11.1 [MVP]** Agent death mid-task → task `interrupted`; reconnect runs a recovery
  pass (inspect git/tests → propose resume/redo/keep-as-manual). **F11.1b** editor
  restart restores tabs/panel; MCP reloads from file; (v2: headless continuation).
  **F11.1c** MCP death = hard stop for plan-mutating work; atomic writes prevent tears.
- **F11.2 [MVP]** Session resume: hook-driven plan_get + ticket resync + resume briefing
  (waits for "resume"). **F11.2b** context rotation = resume flow + `session_rotated`
  timeline event. **F11.2c** gates never expire.
- **F11.3 [MVP]** Executor lease (one thread executes; stale lease reclaim with
  confirm). **F11.3b** write collisions: user wins, agent re-reads.
- **F11.5 [MVP]** Integrity: validation, three undo layers (atomic temp+rename, .bak,
  git). **F11.5b** external plan.json edits become user revisions; merge conflicts on
  the plan file render as conflict cards.
- **F11.6 [MVP]** Degrade, don't block: Jira down → stale-flagged cards + snapshot
  coverage + queued write-backs; git remote/auth issues → retryable blockers with
  fix-it hints. **F11.6c [v1]** vanished branch/worktree → recreate-from-commits or
  rebind.
- **F11.7 [MVP]** Abandon: revert-all | keep partial | keep + mark manual; worktree
  prune; ticket comment; archive with timeline.

## 13. Settings surface

**Two homes, one rule:** `.plans/policy.json` = what the *work* must satisfy (versioned,
PR-reviewed, enforced); Zed `settings.json → "plan"` = how *you* drive. A knob that
changes what lands in git or what the agent may do is policy; experience-only is a
setting. **F12.2 precedence only tightens:** managed → project → personal → per-agent.

- **F12.1 [MVP]** Personal keys: panel dock, auto-open, tab_open_on, default lens (auto
  follows status), follow mode, peek-on-hover, revisions staged|auto, comment placement,
  attention (pulse set, toast set, quiet_while_typing, digest threshold), stuck/
  escalation thresholds, rehearse default, worktree mode. **F12.1b [v1] per-agent
  overrides** (trust is agent-shaped).
- **F12.2c [MVP] Settings page** in Zed's settings editor: presets
  (Careful/Balanced/Fast) + subsections Surfaces / Review / Attention / Execution /
  Team policy (REPO badge, provenance, propose-as-PR, B/W/off severity, 🔒 managed
  locks, live commit-regex tester on real history) / Per-agent (inherit cells).
  ⌖ point-of-use badges — **F12.3 [MVP]** every recurring choice is also settable in
  context. **F12.3b [v1]** distilled rules surface here for lint promotion.
- **F12.4** Opinionated defaults; presets write the same keys.

## 14. Scope summary

| Tier | Contents |
|---|---|
| **MVP** | Plan tab/panel/pill · three lenses + previews · plan.json + MCP server + skill + hooks · intake questions + assumptions · streaming draft · plan lint (incl. ticket-coverage, git rules) · full review loop (anchors, comment/suggest/alternatives/flag, severity, batch, pushback, staged revisions, change cards) · tickets (from-ticket, cards+coverage, drift/resync, ticketless) · launch + branch + task-commits + trailers · execution spotlight + gates + amendments + failure ladder · pause/edit/re-sync + receipts · destructive-ops gating · failure/edge handling (§12 MVP items) · settings page + personal keys |
| **v1** | Peek cards · plan browser · attention queue · code-anchored feedback · rehearsal · acceptance auto-check + live evidence · timelines + digest · worktree isolation · PR generation + collision/drift rails · git panel grouping · write-back · quick-plan · per-agent settings · policy editor extras · stuck detection |
| **v1.5** | Distillation → standing rules → lint promotion |
| **v2+** | Stacked PRs/epics · blame→plan peek · headless continuation · templates/bootstrapping · metrics/retrospective |

## 15. Remaining open questions

1. ~~Naming~~ — resolved: **Plan**, singular, thread-scoped; "Plans" = browser only.
2. Margin vs inline comments — inline shipped as default; margin behind a setting.
3. ui-states previews: GPUI-native mini-renders (MVP: styled blocks) vs sandboxed
   HTML/SVG (later).
4. Pushback expiry: block forever (current: safe) vs auto-keep after next revision.
5. Assumption auto-send: uniform batch (current) vs instant for 💡.

---

## Appendix A — plan.json schema (v1)

```jsonc
{
  "schema_version": 1,
  "id": "LED-212",
  "title": "Add pagination to the invoices table",
  "status": "executing",            // intake|drafting|in_review|revising|approved|executing|paused|gate|amending|done|abandoned
  "rev": 5,
  "thread": "acp-7f3a",                                    // 1:1 — the owning thread (F1.0)
  "executor": { "thread": "acp-7f3a", "taken_at": "…" },   // null when idle; always the owning thread
  "tickets": [ { "source": "jira", "key": "LED-212", "type": "Story", "status": "In Progress",
      "priority": "High", "assignee": "…", "url": "…",
      "ac": ["paged slices + total", "footer range + prev/next", "deep-linkable page", "export unaffected"],
      "fetched_at": "…", "drift": null } ],
  "spec": {
    "goal": "…", "scope": { "in": ["…"], "out": ["…"] },
    "acceptance": [ { "id": "a1", "when": "…", "shall": "…",
        "ticket_ac": "LED-212#1",   // null when plan-originated
        "tasks": ["t1","t2"], "done": true,
        "evidence": [{ "type": "tests", "ref": "…", "captured_at": "…", "stale": false }],
        "waived": null } ],         // or { by, reason, at }
    "open_questions": [ { "id": "q1", "text": "…", "options": ["…"],
        "assumption": "server-side", "assumed_blocks": ["t1","a1"], "answer": null } ]
  },
  "design": { "contracts": [ { "id": "k1", "rows": [{ "in": "…", "out": "…" }] } ],
      "previews": [ { "id": "p1", "kind": "ui-states", "attached_to": "t3", "body": {} } ],
      "decisions": [ { "text": "offset over cursor", "rationale": "…" } ],
      "risks": [ "CSV export shares the endpoint" ] },
  "tasks": [ { "id": "t2", "title": "…", "system": "backend", "gate": false,
      "ticket": "LED-212", "depends_on": ["t1"],
      "files": [{ "path": "app/routes/invoices/route.tsx", "op": "M" }],
      "steps": [{ "id": "s1", "text": "…", "assumes": null,
                  "guard": null }],                   // or { type:"approve"|"input", prompt,
                                                      //      state:"pending"|"holding"|"cleared",
                                                      //      response, cleared_at, evidence_for:["a1"] }
      "commit_message": "[LED-212]: accept page params in invoices loader",
      "preview": "p2", "acceptance": ["a1","a2"],
      "status": "in_progress",      // pending|in_progress|done|failed|skipped|interrupted
      "manual": false,
      "artifacts": { "sha": null, "diffstat": null, "tests": null },
      "timeline": [ { "at": "…", "kind": "edit", "detail": "route.tsx +12" } ] } ],
  "comments": [ { "id": "c1", "kind": "comment",    // comment|suggestion|alternatives|flag  (lint/rehearsal author via "author")
      "author": "user",                             // user|agent|plan-lint|rehearsal
      "severity": "blocker",
      "anchor": { "lens": "tasks", "block": "t2.s1", "range": [6,52], "quote": "…",
                  "code_refs": [{ "path": "app/routes/api.invoices.export/route.ts", "line": 12, "quote": "…" }] },
      "outdated": false,
      "suggestion": null,           // { original, replacement, applied: verbatim|adapted }
      "alternatives": null,         // { options:[{id,title,summary,tradeoffs}], selected }
      "thread": [ { "author": "user", "text": "…" },
                  { "author": "agent", "text": "…", "action": "revised",  // revised|pushback|answered
                    "rev": 3, "cites": [{ "path": "…", "line": 88 }],
                    "pushback": { "on": "…", "reason": "…" } } ],
      "state": "addressed",         // open|sent|addressed|resolved|reopened
      "caused_changes": ["t2.s1"] } ],
  "pending_revision": { "rev": 6, "hunks": [ { "id": "h1", "target": "t2.s1",
      "old": "…", "new": "…", "from": "c1", "state": "pending" } ] },   // staged revisions & amendments
  "git": { "branch": "led-212-invoices-pagination", "base": "develop",
      "worktree": null, "ahead": 3, "behind": 0, "dirty": false,
      "commits": [ { "task": "t1", "sha": "3f81c2a", "diffstat": "+42 -6" } ],
      "pr": null },
  "history": [ { "rev": 5, "by": "agent", "kind": "revision", "summary": "…" },
               { "at": "…", "kind": "session_rotated" } ]
}
```

## Appendix B — policy.json schema (v1)

```jsonc
{
  "lint": {
    "every-task-has-tests": "blocker", "criteria-link-tasks": "warn",
    "files-must-exist": "warn", "prod-requires-gate": "blocker",   // may be 🔒 managed
    "max-files-per-task": { "severity": "warn", "limit": 8 },
    "ticket-coverage": "blocker"
  },
  "git": {
    "branch": { "pattern": "{ticket}-{slug}", "pattern_no_ticket": "{slug}",
                "base": { "feature": "develop", "hotfix": "main" } },
    "commit": { "format": "^\\[LED-\\d+\\]: .{8,72}$", "format_no_ticket": "^.{8,72}$",
                "imperative_mood": true, "one_commit_per_task": true,
                "require_tests_green": true,
                "trailer": "Plan: {ticket} rev{rev} task-{task}" },
    "worktree": { "mode": "auto", "root": "../acme-worktrees" },
    "pr": { "target": "develop", "template": "plan-summary",
            "reviewers": "codeowners", "sync_body": true },
    "destructive_ops": "amendment_only"
  },
  "tickets": { "coverage": "strict", "write_back": true },
  "launch": { "quick_plan": { "max_tasks": 3, "max_files": 6 } },
  "guards": { "require_on": ["prisma migrate", "db:seed", "DROP ", "docker volume rm"],
              "default_type": "approve" }             // lint auto-adds ⛨ to matching steps
}
```


---

# Part II — Build plan

## 1. Architecture: three components + one file

```
┌─ Zed fork (Rust) ──────────────────────────────┐
│  plan_core   schema · atomic IO · watch · lint  │
│  plan_ui     Plan tab · panel · pill · rail     │
│  (tiny registration diff in zed/src/zed.rs)     │
└───────────────▲────────────────────────────────┘
                │ file watch / write
        .plans/<id>.plan.json   ← the only IPC
                ▲ read / write
┌───────────────┴────────────────────────────────┐
│  zed-plan-server (TypeScript · MCP · stdio)     │
│  plan_* tools · lint · rehearse · revisions     │
└───────────────▲────────────────────────────────┘
                │ MCP (context_servers)
        Claude Code (via claude-code-acp)
        + planning skill + hooks (PreToolUse gate…)
```

**The key decoupling decision: plan.json IS the IPC.** The MCP server never talks to
Zed directly. The agent mutates the plan through `plan_*` tools → server writes the
file (atomic temp+rename) → Zed's worktree watcher fires → `plan_core` reloads →
`plan_ui` re-renders. Your edits go the other way: UI writes, server reads fresh from
disk on every tool call (`plan_get` is never cached). This buys you:

- **Language freedom** — the MCP server is TypeScript, your home turf, with the mature
  `@modelcontextprotocol/sdk`. All the gnarly protocol/tool logic lives where you're
  fastest.
- **Crash isolation** — server dies, file's intact, Zed keeps rendering (F11.1c free).
- **Agent-agnostic** — anything that speaks MCP gets the same contract.
- **Testability** — plan_core and the server test against fixture files, no Zed needed.

Sync receipts (F5.3b) fall out naturally: the server stamps `history[]` + rev on every
write; the UI shows the last stamp it loaded.

## 2. Repo layout

```
zed/ (your fork)
  crates/plan_core/          # ~pure Rust, no GPUI
    src/schema.rs            # serde structs, schema_version, migrate()
    src/store.rs             # load/save atomic, .bak, watch (notify via worktree events)
    src/lint.rs              # rule engine over policy.json
    src/anchor.rs            # fuzzy re-anchoring for comments
    src/rev.rs               # pending_revision apply/reject, history
  crates/plan_ui/
    src/plan_view.rs         # Item + SerializableItem, thread-following singleton
    src/lenses/{spec,design,tasks}.rs
    src/blocks/…             # preview blocks, ticket card, guard badges, rail
    src/plan_panel.rs        # dock Panel
    src/status_pill.rs       # StatusItemView
    src/review/…             # selection→toolbar, comment threads, staged revisions
    src/plan_settings.rs     # settings.json "plan" key + settings page section
  crates/zed/src/zed.rs      # +2 lines: register panel, register item  ← the fork diff

plan-server/ (separate repo, TypeScript)
  src/index.ts               # MCP stdio server
  src/tools/*.ts             # plan_create, plan_get, task_update, …
  src/store.ts               # same atomic write discipline as plan_core
  src/lint.ts                # shared rule definitions (see §6 note)

plan-agent/ (separate repo or folder)
  skills/planning/SKILL.md   # the protocol: lifecycle, question policy, per-task loop
  hooks/pretooluse-gate.ts   # blocks Edit/Bash when plan requires (guards, no-plan)
  hooks/sessionstart.ts      # plan_get + ticket resync + resume briefing
  hooks/stop-guard.ts, precompact.ts
  settings-fragment.json     # context_servers + hooks wiring for Claude Code
```

Fork discipline: **plan_ui may depend on workspace/agent_ui/editor/theme; nothing may
depend on plan_ui.** Upstream files touched: `zed.rs` registration (+ maybe a
`Cargo.toml` line). Everything else is additive crates — weekly merges stay boring.

## 3. Zed internals study list (ordered, with the question each answers)

Do this as a reading pass with Claude Code in the repo before writing UI code:

1. **`crates/workspace/src/dock.rs` + an existing panel (`terminal_view` or
   `git_ui`)** — the `Panel` trait: position, persistence, activation. *Answers: what
   does plan_panel.rs implement?*
2. **`crates/workspace/src/item.rs`** — `Item`, `SerializableItem`, tab rendering,
   `WorkspaceDb` round-trip. *Answers: how the singleton Plan tab restores with its
   thread binding.*
3. **`crates/agent_ui/src/agent_panel.rs` + `crates/acp_thread/`** — how the panel
   tracks the *active thread*, what events a thread emits (tool calls, status), how
   thread switching works. *Answers: F1.1 thread-following + the live activity feed
   (subscribe, don't fork).*
4. **`crates/zed/src/zed.rs` `initialize_panels()` / item registration** — the two
   lines you'll add.
5. **`crates/editor/` + `crates/multi_buffer/`** — creating read-only excerpts,
   rendering diffs (hunk UI is in editor), block decorations. *Answers: how preview
   blocks and staged-revision diffs render as mini-buffers (F1.2b, F9.3).*
6. **`crates/theme/`** — `cx.theme().colors()` etc.; confirm every token the mockups
   used maps to a `ThemeColors` field.
7. **`crates/git/` + `crates/git_ui/` + repository state** — branch info, diffstat,
   commit access for the branch strip + commit rail; how git_ui groups hunks.
8. **`crates/settings/`** — registering the `"plan"` settings key with schema +
   defaults; how the settings UI page sections register.
9. **`crates/project/src/` context server / MCP wiring** — how `context_servers`
   config spawns stdio servers; where the server binary path resolves. *Answers: how
   zed-plan-server ships.*
10. **`crates/workspace` status bar (`StatusItemView`)** — the pill.

## 4. Milestones

Each ends in something demoable/usable. MVP feature IDs in brackets.

**M0 · Fork & rhythm (a weekend)**
Fork, build, run. Set up `upstream` remote + a `merge-upstream.sh` you run weekly.
Create empty `plan_core`/`plan_ui` crates, register a "hello" panel behind a feature
flag. *Proves: build loop + the 2-line fork diff.*

**M1 · plan_core (small)**
Schema structs from Appendix A, `schema_version`, atomic save + `.bak`, load-validate,
unit tests over fixture plans (use the LED-212 demo content as the fixture — it's
already a complete plan).

**M2 · zed-plan-server + skill — use it before any UI exists (the keystone)**
TypeScript MCP server: `plan_create/get/update_section/add_task/task_update/
set_status/answer_question` + the planning SKILL.md + hooks. Wire into Claude Code via
`context_servers`/settings. **Run a real task with it in plain Claude Code, reading
plan.json in a split.** [F6.1–6.4 core, F2.0/F2.3b protocol]
*This is deliberately before UI: it validates the entire agent contract — skill
adherence, hook enforcement, per-task re-reads — with zero GPUI risk. If the loop
doesn't work here, no amount of UI saves it.*

**M3 · Read-only Plan tab**
`PlanView` as Item+SerializableItem, thread-following binding (F1.0/F1.1), file-watch
reload, Tasks lens rendered from plan.json (cards, steps, badges — GPUI chrome only,
no mini-buffers yet), lens switcher, Spec/Design as styled text. *Demo: watch the
agent draft live in the tab.* [F1.1, F2.1 partial]

**M4 · Pill + panel + activity**
Status pill (F1.4), dock panel with pipeline + live column fed by acp_thread events +
plan state (F1.3, F4.3), sync receipts (F5.3b), thread-row state via sidebar
integration if cheap or defer (F1.5).

**M5 · Review loop**
Selection→floating toolbar, comment/suggest/flag with anchors (plan_core::anchor),
threads UI, severity + batch send, server tools `list_comments/reply_comment/
apply_suggestion/mark_addressed/propose_revision`, staged revisions with per-hunk
apply (editor diff machinery from study #5), change cards, Approve gating, lint engine
+ auto-flags. [F3.1–3.7, F9.1, F9.3] *The biggest milestone — split it: 5a comments
round-trip, 5b staged revisions, 5c lint.*

**M6 · Execution**
Launch flow + ExitPlanMode integration, per-task loop enforcement (hooks hard-block:
no-plan edits, guard holds), GATE tasks + step guards with input capture
(F4.5/F4.5b), amendments as staged diffs (F4.7), failure ladder counters (F11.4),
pause/resume/stop + kill (F5.1/F5.6), interrupted-task recovery pass (F11.1).

**M7 · Git**
Branch on launch + guards (F10.2), commit-per-task + trailer via the skill + commit
hook check (F10.1/F10.3), branch strip + commit rail UI reading repo state (study #7),
`destructive_ops: amendment_only` enforcement (F10.5c), revert-task-commit action.

**M8 · Tickets**
`tickets[]` via the agent's Jira MCP (the skill orchestrates; the plan-server just
stores), ticket cards + coverage meter (F2.4b/f), `ticket-coverage` lint, drift
snapshot/resync at the F2.4g checkpoints, task↔ticket commit prefixes (F2.4c),
ticketless fallbacks (F2.4d).

**M9 · Settings + hardening**
`"plan"` settings key + settings page section (F12.1/F12.2c), presets, policy.json
severity wiring, then the §12 failure drills: kill the server mid-write, kill Zed
mid-execution, corrupt the file, edit plan.json externally — each should land in a
designed state, not a surprise.

**v1 tail (post-MVP, in rough order of your likely appetite):** peek cards → rehearsal
→ live evidence + acceptance auto-check → PR generation → worktree isolation →
attention queue → code-anchored feedback → write-back → Plans browser.

## 5. Fork maintenance

- `main` mirrors upstream; `plan` is your integration branch; weekly
  `git fetch upstream && git merge` on main, then rebase-or-merge plan.
- Keep a `FORK_DIFF.md` manifest listing every upstream file touched (target: 1–2).
  When a merge conflicts outside that list, something leaked — fix the leak.
- Pin GPUI API breakage risk by building weekly even when not developing.

## 6. Design decisions to hold onto while building

- **Lint lives twice, defined once.** Rule *definitions* (names, severities, params)
  are data in policy.json; plan_core and plan-server each implement checkers. Keep the
  rule IDs identical so findings are interchangeable. (Alternative — lint only in the
  server — is acceptable for MVP if double-implementation chafes; Zed then shows
  server-produced findings only.)
- **The UI never mutates through the server.** UI writes plan.json directly via
  plan_core; the server writes for the agent. Last-writer wins at the file level;
  rev-stamps + user-wins policy (F11.3b) arbitrate semantic collisions.
- **Subscribe to agent_ui, never patch it.** Activity feed and thread-following come
  from acp_thread events + workspace active-thread state. If an event you need isn't
  public, add a minimal `pub` accessor and record it in FORK_DIFF.md.
- **Mini-buffers can wait.** M3–M5 render plan content as GPUI text/chrome; swap
  Spec/Design prose and diff blocks to read-only multibuffer excerpts when you want
  selection-fidelity (it's an upgrade, not a foundation).

## 7. Risks & early spikes

| Risk | Spike (cheap, early) |
|---|---|
| GPUI learning curve | M0 hello-panel; copy patterns from git_ui, not docs |
| acp_thread events too opaque for activity feed | During study #3: log every event from one Claude Code session; confirm tool-call granularity |
| Claude Code hook coverage (PreToolUse deny, SessionStart inject) | Day-one test in M2 with a trivial deny hook — before building enforcement on it |
| File-watch latency makes "live" feel laggy | Measure in M3; fallback: server also touches a version file / UI polls at 250ms during executing |
| Skill adherence (agent forgets protocol) | M2's whole point; tune SKILL.md + per-turn re-inject until a full task runs clean 3× |
| Fuzzy re-anchoring quality | Property tests in plan_core with mutation fixtures before building comment UI on it |

## 8. Working style

Build it with the philosophy it encodes: every milestone starts as a written plan
(spec section references + acceptance criteria), Claude Code executes task-by-task
with tests, one commit per task. M2 onward, **use the system to build the system** —
plan.json for M3 should be authored through the M2 server. Dogfooding pressure is the
fastest spec validator you have.

First three concrete sessions:
1. M0: fork + build + hello panel + merge script.
2. Study pass: questions #1–#4 with notes committed to `docs/zed-notes.md`.
3. M1: schema.rs + store.rs from Appendix A, tests against the LED-212 fixture.


---

# Part III — Working agreement for Claude Code

## Execution protocol
1. **One milestone at a time**, in Part II order (M0→M9). Before coding a milestone,
   write a short plan for it: tasks, files, tests, the Part I feature IDs it
   implements, and open questions. Get sign-off, then execute task-by-task.
   From M2 onward, author that plan through the plan-server itself (dogfood).
2. **Tests first where feasible**: plan_core and plan-server logic are test-first
   (fixtures = the LED-212 content from the demo). GPUI views get smoke coverage +
   manual demo notes.
3. **One commit per task**, imperative mood, prefixed `[PLAN-M<n>]`, e.g.
   `[PLAN-M1]: add atomic save with .bak rotation`. Never commit with failing tests.
4. **Definition of done per milestone**: tests green · the milestone's "demo" line
   from Part II reproducible · `FORK_DIFF.md` updated if any upstream file changed ·
   a short `docs/milestones/M<n>.md` note (what works, what's deferred, surprises).

## Hard rules
- Do not modify upstream Zed files beyond the FORK_DIFF.md manifest; if an internal
  API isn't public, add a minimal `pub` accessor and record it — never restructure
  upstream code.
- Nothing outside `plan_ui`/`plan_core` may depend on them.
- plan.json writes are always atomic (temp + rename) with `.bak` retention, in both
  Rust and TypeScript implementations. `plan_get` never serves cached state.
- Stay inside MVP scope (Part I §14). If an implementation choice would pull in a v1
  feature, flag it instead of building it.
- Respect the enforcement philosophy: guards/gates **block** (PreToolUse deny), they
  don't advise.
- **All styling resolves through the active Zed theme.** Every color in plan_ui comes
  from `cx.theme().colors()` / status colors by semantic role (Part IV §1 table);
  fonts from `ui_font` / `buffer_font` settings; zero hardcoded hexes, font names, or
  bespoke constants. If a needed role has no obvious ThemeColors field, pick the
  closest semantic role and record the mapping decision in the milestone note —
  never invent a constant. (The mockups' One Dark hexes exist only to verify the
  mapping renders equivalently under the One Dark theme.)

## When to stop and ask
- A Zed upstream API needed for a milestone has changed or doesn't exist as the study
  notes assumed.
- Part I is ambiguous or two features conflict for a concrete case — cite both IDs.
- A spike (Part II §7) fails — e.g. hooks can't deny, or acp_thread events lack tool
  granularity. These invalidate design assumptions; don't route around them silently.
- Anything requiring destructive git operations on the fork.

## Reading order (first session)
1. This preamble + Part III (you're here).
2. Part II §1–§3 (architecture, layout, study list).
3. Part I §0–§2 + Appendix A (principles, lifecycle, surfaces, schema).
4. Part IV §1–§2 (token mapping, surface layout) before any plan_ui work; return to
   its component sections (§3–§7) as you build each surface, with the demo HTML open
   for visual corroboration. The §9 state matrix is the acceptance reference for
   lifecycle-dependent chrome (pill, tab dot, panel, primary action).
5. **UI enforcement**: `docs/design/plan-ui-compliance.md` is the testable contract
   for all plan_ui work, and the `plan-ui-design` skill
   (`.claude/skills/plan-ui-design/SKILL.md`) auto-loads it whenever plan_ui is
   touched. Run its §13 verification protocol per component; design gaps go back to
   the design track, never get improvised.
Then begin M0.

---

# Part IV — UI design specification

Textual capture of the design encoded in mockups v1–v9 and the end-to-end demo, mapped
to feature IDs. This is the buildable description; the HTML artifacts remain the visual
reference. **All colors are Zed theme roles** — One Dark reference hexes are listed only
so you can verify against the mockups.

## 1. Design language & token mapping

Fonts: UI = `.ZedSans` (IBM Plex Sans), code/metadata = `.ZedMono` (Lilex). Base UI
size 13px; metadata runs 9–11px mono; radii 5–8px; borders 1px `border.variant`.

| Role (use this) | One Dark ref | Used for |
|---|---|---|
| `title_bar.background` / `status_bar.background` | #3b414d | window chrome (lightest) |
| `panel.background` / `tab_bar.background` | #2f343e | sidebars, panel, cards |
| `editor.background` / `tab.active_background` | #282c33 | center pane, inputs, nested blocks |
| `element.hover` / `border.variant` | #363c46 | hover fills, hairlines |
| `element.active` / `element.selected` | #454a56 | active nav, segmented-on |
| `border` | #464b57 | interactive borders (chips, buttons) |
| `text` | #dce0e5 | primary text |
| `text.muted` | #a9afbc | secondary text |
| `text.placeholder` | #878a98 | metadata, timestamps, disabled |
| `text.accent` | #74ade8 | links, active state, info, running |
| `created` (+ bg/border variants) | #a1c181 / rgba(...,.1) / #38482f | success, done, additions, evidence |
| `modified` (+ variants) | #dec184 / #5d4c2f | warnings, gates, guards, drift, staged |
| `deleted`/`error` (+ variants) | #d07277 / #4c2b2c | blockers, failures, deletions |
| `info` bg/border | rgba(116,173,232,.1) / #293b5b | info banners, selected alternative |
| syntax: keyword/function/type/string/property/comment/number/constant | #b477cf #73ade9 #6eb4bf #a1c181 #d07277 #5d636f #bf956a #dfc184 | code blocks; **purple (keyword) doubles as the "agent/amendment/PR/distill" identity color** |
| `version_control.added` | #27a657 | diffstat + |
| selection | rgba(116,173,232,.24) | plan-text selection |

Status colors are systematic: **accent = agent working**, **modified = needs you**,
**created = done/verified**, **deleted = failed/blocker**, **placeholder = idle/pending**.
Pulse animation (opacity 50% @ ~1.1–1.4s) means "needs you *now*" or "live".

## 2. Surface layout (agentic layout)

Left→right: **Threads sidebar** (196px, panel bg) → **Agent panel** (320px, panel bg)
→ **Center** (editor bg; tab bar 34px) → optional Project panel. **Plan panel** docks
bottom (200px default). Status bar hosts the pill.

- **Plan tab** [F1.1]: title `Plan — <id>`; leading 8px status dot: placeholder=
  drafting(pulse), modified=review/gate/guard, success=approved/done, accent=executing
  (pulse), deleted=failed. Singleton; follows active thread.
- **Threads sidebar rows** [F1.5]: status dot (same colors) + name; sub-line 10px
  placeholder: state text ("plan 2/5", "gate — needs you") + `⌂ wt/<branch>` worktree
  badge when isolated.

## 3. Plan document (center) — component anatomy

### 3.1 Plan toolbar [F1.1, F3.6]
`[STATUS PILL] [rev n] [Spec|Design|Tasks lens] [⚑ n blocker chip] …spacer…
[contextual actions] [primary]`
- Status pill: 10.5px caps, 99px radius, colored per state family (muted/warn/ok/
  info/err); pulses for GATE and guard-hold.
- Primary action per state: Review→`Approve` (disabled while blockers>0, tooltip
  says why); Approved→`▶ Launch` (disabled while rehearsal mismatches); Executing→
  `⏸ Pause`; Amend→`Accept amendment`/`Reject`; Gate→`✓ Approve gate`; Done→
  `Open PR ↗`. Buttons: 1px `border`, 5px radius; `.primary` = accent fill, dark text.

### 3.2 Ticket card [F2.4b/f/g]
Panel-bg card, one row: `⛓ KEY` (mono, accent) · type/priority/source (muted) ·
status chip (To Do muted / In Progress info / Done success) · **coverage meter**
right-aligned: 22×6px segments, one per ticket AC — success=covered, modified=
needs-update, empty=unmapped — + label ("ticket AC 4/4 covered") · sync stamp 9.5px
mono ("synced 2m · ↻"). **Drift variant**: modified border; meter shows the changed
segment amber; stamp reads "drift · resynced at gate". Followed by a **drift card**:
amber header `⛓ Ticket drift · edited by @who when`, body shows old line
(struck, error-bg) → new line (created-bg), actions `Apply rev n` / `Discuss`.

### 3.3 Branch strip [F10.2, §9.1]
One row under the title: `⎇ branch ← base` chip (mono, editor bg) · `↑n ↓n`
(ahead success / behind placeholder; behind>0 turns amber = drift) · dirty dot +
label (success "clean" / modified "agent editing" / deleted "task failed") ·
right: PR chip (placeholder "PR —" until created; purple live: `⑂ PR #318 ·
checks ✓ · @who approved`).

### 3.4 Cards (shared chassis)
Rounded 8px, panel bg, colored header band 11px caps (warn/ok/info/err/purple) with
optional right-aligned mono source note; body rows separated by variant hairlines.
Instances: **Lint** [F9.1] (rows: severity glyph ⚑ error / ⚠ modified + finding +
mono rule id right + indent detail + fix buttons; auto-fixed rows dimmed with
`✓ auto-fixed` success pill) · **Staged revision** [F9.3] (rows per hunk: target +
"from c1/s1/a1" provenance + old line struck error-bg / new line created-bg + right
`✓ Applied` or Apply/Reject pair; header notes "plan unchanged until applied") ·
**Rehearsal** [F9.4] (per-task rows, right status ✓/⚠; mismatch row amber bg with
predicted-files line and `＋ Add to plan` / `Ignore`) · **Gate evidence** [F4.5]
(mono evidence rows: ✓ + claim + link; actions Approve & finish / Re-verify all /
Request changes) · **Amendment** [F4.7] (error header; +new-task line created-bg,
~changed line amber; Accept/Reject) · **PR** [F10.4] (purple header; checks +
reviewer rows) · **Distillation** [F9.5] (purple; provenance chips `c1 · LED-212`;
rule text in mono block; Save rule / Also add to plan-lint / Not now).

### 3.5 Task card + steps [F1.2, F4.5b]
Border-variant card; state borders: active=info-border, failed=error-border,
gate=modified-border. Header row: **checkbox** (15px, 4px radius: empty pending /
spinner accent / ✓ success fill / ✕ error fill / ⏸ amber outline for gate) · mono
task number · title (struck when done) · chips: ticket `LED-212` (mono 9px) ·
system badge (Backend purple / Frontend teal / Testing green / GATE amber — 9px
caps pill) · guard summary `⛨ n guarded steps` (amber pill, review states) ·
**sha chip** `⌥ 3f81c2a +42 −6` (mono, diffstat colored) · tests/evidence chips ·
amendment tag `◆ amendment · rev n` (purple).
Body (indented 40px): **steps** as numbered rows (16px circle sn); during
execution completed steps swap sn→green ✓. **Guard badges** on steps: `⛨ guarded ·
approve to run` (amber pill) and `✋ needs your input` (teal pill); active guard =
amber + pulse; cleared guard = mono success receipt `⛨ approved 09:41 · migrate
dev ✓`. **Input guard panel** (amber border, panel bg): caps label ("Record
verification — required to continue · stored as evidence on a1"), input field
(editor bg, caret), `Record & continue ⏎` primary / `Pause — I'll verify later`,
hint "hook blocks the agent until recorded".

### 3.6 Commit rail [§9.1]
24px gutter left of task cards; 2px `border` spine. Nodes 13px at task-header height:
hollow border = pending · success fill = committed · accent ring pulse = in progress ·
error fill = failed · amber hollow = gate · **purple 45°-rotated square = amendment**,
connected by a purple curve branching from the spine. Foot: `▼ base · ↑n ahead ·
⑂ PR #…`. Rail hidden (and gutter collapsed) before launch.

### 3.7 Review primitives [F3.x, F9.2]
- **Selection**: accent-tint highlight; **floating toolbar** above (element.active bg,
  shadow): `💬 Comment ⌘⇧M · ✎ Suggest ⌘⇧E · ⇄ Alternatives · ⚑`.
- **Comment thread** (editor-bg card, indented under anchor, max 600px): header =
  state chip (open amber / pushback red / resolved green) + mono anchor
  `c1 ↪ t2.s1 + file:line`; messages with 14px avatar (user amber "Y" / agent purple
  "C"); **code quote** = accent left-border mono block; **code-ref chips**
  `↗ file:line` (info pill, clickable); **pushback block**: bold red lead, cited
  refs, then `[Change anyway] [Keep as planned]`.
- **Suggestion**: header `✎ Suggested edit · you` + anchor; old row struck error-bg,
  new row created-bg.
- **Alternatives**: stacked option cards, mono letter badge; selected = accent border
  ring + `✓ selected`; one-line trade-offs in placeholder.
- Applied changes leave **Δ chips** on the text (`Δ rev 3 · c1`, info pill) linking
  provenance.

### 3.8 Spec & Design lens blocks
- **Acceptance list** [F4.6]: rows with mini-checkbox; `WHEN` teal bold, `SHALL`
  purple bold; ticket ref `LED-212#1` purple mono 9px; evidence chip accent mono
  (`5 tests · 3f81c2a`). Sechead shows running count ("· 3/6").
- **Preview blocks** [F1.2b]: dashed-border block, teal caps label (`◈ Contract —…`).
  Contract rows = 3-col mono grid `input → output` (input constant-gold, output
  success, clamped/warned amber). **ui-states gallery**: mini rendered table
  (header/rows/footer with range text + Prev/Next buttons, disabled at bounds,
  hidden-controls state) with mono caption per state.
- **Assumption block** [F2.3b]: `A1 · assumed —` + text; tagged blocks list;
  "unconfirmed — will ⚠ at Approve" → "✓ confirmed by …" (success).

## 4. Plan panel (bottom dock) [F1.3, F4.3, F5.3b]
Header: status dot + `Plan · <id>` · **sync receipt** mono 9.5px ("agent synced
rev 4 · just now"; amber when behind) · right actions (contextual: Pause / Record
input / Accept amendment / Approve gate / Stop / ✕).
Body 2 columns: **pipeline** (compact task rows: 13px checkbox/spinner + label +
mono right-note — sha, "running", "✋ guard s3", "GATE"; active row info-tint,
failed row error-tint, gate/guard row amber-tint) · **live column** (caps section
header + mono activity rows: teal verb column `plan/edit/run/guard/hook/ev/drift` +
detail; success rows green verb, failures red; live row has pulsing dot).

## 5. Status-bar pill [F1.4]
`◆ Plan <fragment>` — text + color per state: muted intake/drafting · warn
"2 questions — need you" / "review · 3💬 ⚑" / "rehearsed · 1 mismatch" / "GATE +
drift — 2 need you" (pulse) · info "2/5 · acc 1/6" · err "t4 failed — needs you" ·
ok "done ✓ · PR #318". Click toggles panel.

## 6. Agent-panel elements (intake & prompts) [F2.0, F3.6]
- **Question wizard**: progress row ("Question 1 of 2" + dots: accent=current,
  success=done, dim=pending). **Question card** (editor bg): teal caps label
  `Q1 · topic`, body, option chips (99px pills; picked = accent border/text/info-bg);
  escape chip ("Why do you ask?"). Picking reveals **context box**: hairline-top
  section, caps label "Add context — optional", input (panel bg, caret),
  `Confirm — next ⏎` primary / `Skip context`, hint "context lands in the plan
  record". **Queued question**: dashed border, 55% opacity, one-line preview.
  Answered (compact): `· ✓ answer` in label + provenance line 10.5px placeholder
  ("→ scope + criteria a1/a2 · context added risk r2").
- **Assumption card**: same chassis, `A1 · assumption — not blocking`.
- **Permission prompt** (launch): info-bg card, title "Ready to code?", summary,
  stacked option buttons (`▶ Launch — approve edits as I go` / auto / Keep planning).
- **Tool rows**: 2px left border, mono tool name; ✓ teal→green when done; pulsing
  dot while running.

## 7. Settings page [F12.2c] (mockup v9)
Settings tab → left nav (Plan parent + subsections; active = element.active bg +
2px accent left bar; Team policy carries purple `REPO` tag). Content max 760px.
**Presets**: 3 cards (Careful/Balanced/Fast), selected = accent ring; clicking
rewrites the same keys shown below. **Setting row**: title (+ `⌖ point-of-use`
teal pill when also settable in context) + 11.5px muted description | control right
(toggle 34×18 accent-when-on / segmented / dropdown / stepper / multi-select chips).
**Policy section**: provenance header bar (`TEAM · VERSIONED` purple pill + mono
"last changed by @who · PR #n" + `Open file` / `Propose change as PR` primary);
rule rows with **B/W/off severity selector** (B red, W amber); `🔒 managed` rows
disable weaker options; **regex tester**: input (mono, constant-gold) + last-5-
commits list with ✓/✕ verdicts. **Per-agent table**: 4-col grid; unset cells render
italic-less placeholder "inherit (value)". **Distillation note**: purple-tinted bar
+ `Review & promote`.

## 8. Interaction & motion rules
- Keyboard: ⌘⇧M comment · ⌘⇧E suggest · ⌘⏎ send-single (batch button sends all) ·
  ⏎ confirms guard input / question context.
- Hover: cross-ref chips peek (F0.5b, v1); chips/buttons get element.hover fill.
- Right-click: panel icon → dock position; rail node → revert task commit.
- Motion: only pulse (attention/live) and spinner (900ms rotate); skeleton shimmer
  for streaming draft blocks; everything else is instant state swap. Honor
  reduced-motion (disable all).
- Empty states: thread without plan → "no plan — ask the agent to draft one".
- Disabled primaries always carry a tooltip naming the blocker ("1 blocker open").

## 9. State → surface matrix (canonical, from the e2e demo)

| Lifecycle | Pill (color) | Tab dot | Panel | Toolbar primary |
|---|---|---|---|---|
| intake (asking) | "2 questions — need you" (warn) | — (no tab yet) | closed | — |
| intake (answered) | "intake ✓ — drafting" (muted) | — | closed | — |
| drafting | "drafting…" (muted) | placeholder·pulse | closed | — |
| lint findings | "lint 1 open" (warn) | modified | closed | Approve disabled |
| in review | "review · 3💬 ⚑" (warn) | modified | closed | Send for revision · Approve disabled |
| rev staged | "rev 3 staged · pushback" (warn) | modified | closed | Apply all · Approve disabled |
| rehearsed ⚠ | "rehearsed · 1 mismatch" (warn) | success | closed | ▶ Launch disabled |
| approved | "approved — launch?" (ok) | success | closed | ▶ Launch |
| guard hold | "guarded step — input needed" (warn·pulse) | modified·pulse | open | Record input |
| executing | "2/5 · acc 1/6" (info) | accent·pulse | open | ⏸ Pause |
| task failed | "t4 failed — needs you" (err) | deleted | open | Accept amendment / Reject |
| gate (+drift) | "GATE + drift — 2 need you" (warn·pulse) | modified | open | ✓ Approve gate |
| done | "done ✓ · PR #318" (ok) | success | closed | Open PR ↗ |

