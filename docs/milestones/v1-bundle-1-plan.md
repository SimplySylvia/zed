# v1 bundle 1 — plan (F4.6d diff-view · F0.5b peek cards · F5.7 attention queue)

> **For Claude:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development` to execute
> task-by-task. All `plan_ui` work is governed by the **plan-ui-design skill** +
> `docs/design/plan-ui-compliance.md` — run its §13 protocol per component. Colors ONLY via
> `cx.theme()`; fonts via `ui_font`/`buffer_font`; no hex, no font names.

**Goal:** ship three approved v1-tail features as one bundle — click-a-sha → read-only commit diff
(F4.6d), hover-peek cross-ref chips (F0.5b), and a rendered "needs you" attention queue (F5.7).

**Architecture:** additive `plan_ui` + fork-local support. One new Cargo dep (`git_ui`, for the
ready `CommitView::open`); two `plan_core` symbols made `pub` (`anchor::block_text`; helpers as
needed). **No upstream source files** — `FORK_DIFF.md` unchanged. Reuses this repo's existing
derivations (`DisplayState`, `current_hold`, `open_blocker_count`/`open_lint_count`, `sync_receipt`,
`tickets::coverage`) and card/chip chassis.

**Scope decisions (developer-approved 2026-07-16):**
- **F4.6d = diff-as-workspace-tab** via `CommitView::open` (NOT an inline in-card mini-buffer).
- **F0.5b = hover-peek only** via `hoverable_tooltip` (NOT ⌥-hover / Esc / click-to-jump / overlay).
- **F5.7 = rendered active-plan "NEEDS YOU" queue** (a documented divergence from the PRD's
  pill-cycle-across-all-plans).

**Commit prefix:** `[PLAN-V1A]`, one commit per task, imperative. **No `Co-Authored-By` trailer.**
Never stage `.rules` / `.claude/` / `.plans/` / `crates/plan_server/LICENSE-GPL`.

---

## Task 0 — prerequisites (shared)
**Files:** `crates/plan_ui/Cargo.toml`, `crates/plan_core/src/anchor.rs`.
1. Add `git_ui.workspace = true` to `plan_ui`'s `[dependencies]` (fork-local; no cycle — `agent_ui`,
   already a dep, pulls `git_ui`; `git_ui` does not depend on `plan_*`).
2. Make `anchor::block_text` (`anchor.rs:50`) `pub` (F0.5b resolver). Confirm `anchor::reanchor` is
   already `pub` (it is — used at `plan_view.rs:859`).
**Verify:** `cargo build -p plan_ui` still green (dep resolves). Commit
`[PLAN-V1A] add git_ui dep and expose anchor::block_text`.

---

## Feature A — F4.6d: diff-view + evidence fidelity

### A1 — click a task's sha → read-only commit diff (tab)
**Files:** `crates/plan_ui/src/plan_view.rs` (`sha_chip` ~2284, `render_task_card` ~737).
`CommitView::open(commit_sha: String, repo: WeakEntity<Repository>, workspace: WeakEntity<Workspace>,
stash: Option<usize>, file_filter: Option<RepoPath>, window, cx)` is `pub` in `git_ui::commit_view`.
- Make the sha chip clickable: give it an `.id()` + `.on_click(...)`. `sha_chip` is a free fn with no
  handles — either convert its call in `render_task_card` to build the click there (the method has
  `self.workspace` + can read `active_repository(cx)` like `git_facts` at `:523-524`), or pass the
  `WeakEntity<Workspace>` + `WeakEntity<Repository>` into `sha_chip`. Prefer building the on_click in
  `render_task_card` (it owns the handles); keep `sha_chip` presentational.
- On click: resolve `repo = workspace.project().active_repository(cx)?.downgrade()`,
  `workspace = self.workspace.clone()`, then `CommitView::open(sha, repo, workspace, None, None, window,
  cx)`. `file_filter = None` (one-commit-per-task ⇒ whole commit == the task's changes).
- The on_click closure receives `(event, window, cx)` — `CommitView::open` needs `window`, so this
  works from a `div`/`Button` `on_click`.
**Test:** `plan_ui` smoke build; §13 = clicking a done task's sha opens the read-only diff.
**Commit** `[PLAN-V1A] open a task's commit diff read-only on sha-chip click`.

### A2 — acceptance evidence chips + running count (Spec lens)
**Files:** `plan_view.rs` `render_spec` (~2570) + its acceptance rows + sechead.
- Render each acceptance criterion's evidence as chips (mono, accent) like `5 tests · 3f81c2a`; when
  `evidence.stale` is true, tint that chip amber (`status.modified`) — reuse the gate card's
  stale-glyph pattern (`:1400-1402`). (Staleness is the agent-supplied `Evidence.stale` bool — do NOT
  invent UI-side freshness; `plan_core` is time-free.)
- Add the running count to the ACCEPTANCE sechead: `ACCEPTANCE · {done}/{total}` (design-spec §3.8),
  `done = acceptance.iter().filter(|a| a.done).count()`.
**Defer (record):** UI-computed freshness (file-changed-since-`captured_at`), test-chip re-run-on-click
(agent-routed), and making the checkbox *derived* from evidence (keep `done` authoritative).
**Test:** §13 (evidence chips + amber stale + count). **Commit**
`[PLAN-V1A] show acceptance evidence chips, stale tint, and a running count`.

---

## Feature B — F0.5b: hover-peek cross-ref chips

### B1 — hoverable peek on block-resolvable chips
**Files:** `plan_view.rs` (chip call sites), `plan_ui.rs` (`mono_chip`/`mono_chip_ticket` ~710-745).
- `mono_chip`/`mono_chip_ticket` return bare non-stateful `div()`. Add an `ElementId` param (or an
  id'd variant) so the peek chips can carry `.id(...)` + `.hoverable_tooltip(...)`. IDs must be unique
  per instance — derive from the block id / comment id / ticket key.
- Wire `hoverable_tooltip(move |_window, cx| Tooltip::element(...))` (idiom: `mention_crease.rs:141`)
  on the **block-resolvable** chips:
  - ticket `⛓ KEY` (`:458`, `:2486`) → peek the `Ticket` (key · status · AC list) from `plan.tickets`.
  - comment anchor `c1 ↪ t2.s1` (`:1709`) → `anchor::block_text(plan, block)` (+ reflect
    `reanchor` result: show "outdated"/moved when not `Anchored`).
  - `Δ from-block` staged-hunk chip (`render_hunk` ~1627) → `block_text` of `hunk.from`.
- Peek card content: a small `v_flex` (editor-bg, rounded, ~360px max) with the resolved title/text.
  Instant (no entrance animation); honor reduced-motion.
**Defer (record):** ⌥-hover, Esc-dismiss, click-to-jump (custom `deferred()`/`anchored()` overlay);
external `ticket_ac #n` peek (no cached ticket-criterion data); the `peek-on-hover` settings toggle
(would touch upstream `default.json` — keep peek always-on).
**Test:** §13 (hover each wired chip → peek shows correct target; outdated anchors reflect state).
**Commit** `[PLAN-V1A] peek cross-ref chips on hover via hoverable_tooltip`.

---

## Feature C — F5.7: rendered attention queue (active plan)

### C1 — enumerate "needs you" items (pure, test-first)
**Files:** new `attention.rs` logic in `plan_ui` (or a `pub(crate)` fn in `plan_ui.rs`); reuse
`plan_core`/`plan_view` predicates. **Test-first.**
- Define `AttentionItem { kind, label, lens, target }` and `attention_items(plan) -> Vec<AttentionItem>`
  enumerating the built sources (in the PRD cycle order where they exist):
  open questions (`open_questions` w/ `answer.is_none()`) · blockers (same filter as
  `open_blocker_count`) · lint findings (`open_lint_count` filter) · guard/gate holds (walk
  tasks/steps for `guard.state=="holding"`, + gate task `gate && InProgress`) · failed task
  (`status==Failed`) · rev-behind (`sync_receipt(plan).1`) · ticket drift (`any_ticket_drift`).
  Each item carries the `Lens` to jump to (Spec for questions/blockers/lint/drift; Tasks for
  holds/gate/failed) and a short label.
- These predicates currently return counts/first-only — add small enumerating variants beside them
  (pure). **Table test** mapping seed plans → expected item kinds/order.
**Commit** `[PLAN-V1A] enumerate attention-queue items from plan state`.

### C2 — render the "NEEDS YOU" panel section
**Files:** `plan_ui.rs` panel `render_plan` (~347-393).
- When `attention_items(plan)` is non-empty, insert a caps **"NEEDS YOU"** section (reuse the
  `caps_label`/role styling already in the panel header) between the header and the pipeline/live
  columns. One compact row per item: kind glyph (role-colored) + label + a jump affordance.
- **Coarse jump-to** (no scroll-to infra exists): row click → open the Plan tab
  (`open_plan_tab`/`PlanView::open_tab`) and set the item's `lens`. (Precise scroll-to-item deferred.)
**Defer (record):** cross-plan aggregation (single thread-bound `PlanFollower`), the pill-cycle click
behavior, precise scroll-to-item, and the unbuilt sources (rehearsal F9.4, pushback F9.2).
**Test:** §13 (section lists the right items for each seed state via `.plans/seed-state.py`; clicking
a row opens the tab on the right lens). **Commit** `[PLAN-V1A] render the NEEDS YOU attention queue section`.

### C3 — design-doc divergence note (design-change protocol)
**Files:** `docs/design/plan-ui-compliance.md` + `docs/design/plan-ui-design-spec.md`.
- Record that F5.7 ships as a **rendered active-plan queue section** (panel), diverging from the PRD's
  pill-cycle-across-all-plans; note cross-plan + pill-cycle + precise-jump as deferred. Present the
  diff for sign-off before/with C2. **Commit** `[PLAN-V1A] document the F5.7 rendered-queue divergence`.

---

## Testing
- **Pure logic (test-first, `cargo test -p plan_ui`):** C1 `attention_items` (table test); any A2 count
  helper. Run `./script/clippy` before each commit; `cargo build -p zed` before §13.
- **Views:** smoke build; real acceptance is the §13 visual check per component (governed by
  `plan-ui-compliance.md`), driven via the `.plans/seed-state.py <guard|failed|gate|done|executing>`
  switcher.

## §13 verification (developer, under One Dark)
- **F4.6d:** click a done task's sha → read-only commit diff opens; Spec-lens acceptance rows show
  evidence chips (amber when stale) + a `n/m` sechead count.
- **F0.5b:** hover a ticket chip / comment anchor / `Δ from` chip → a peek card shows the resolved
  target; an outdated anchor reflects its state.
- **F5.7:** the panel shows a "NEEDS YOU" section listing the current items (walk seed states); a row
  click opens the tab on the right lens.

## Open questions (resolve at the task)
1. **A1 repo handle** — confirm `active_repository(cx)` is the right repo for a task's commit (single
   repo assumed); if multi-repo, which? Flag if ambiguous.
2. **C1 home** — `attention_items` in `plan_ui` (reads `plan_view`/`plan_core` predicates) vs a new
   `plan_core` module. Prefer `plan_ui` (it's a UI concern reusing pure predicates); revisit if the
   predicates aren't all reachable.
3. **B1 id scheme** — ensure per-instance chip ids don't collide (many chips share text).
4. **Commit prefix** — `[PLAN-V1A]` ok?
