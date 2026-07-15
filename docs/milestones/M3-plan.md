# M3 · Read-only Plan tab — implementation plan

> **For Claude:** REQUIRED SUB-SKILLS when executing — superpowers:executing-plans;
> superpowers:test-driven-development for the pure logic (plan-resolution, view-model
> mapping); and **the `plan-ui-design` skill + `docs/design/plan-ui-compliance.md`** for
> every `plan_ui` rendering task (run its §13 verification per component).

**Status:** awaiting sign-off (not started).
**Feature IDs:** F1.1 (Plan tab, thread-following singleton), F1.0 (one thread ↔ one plan),
F2.1 partial (watch the agent draft live).
**Goal:** A read-only Plan **tab** that follows the active agent thread, reloads live as
`plan.json` changes on disk, and renders the Tasks lens (plus Spec/Design as styled text)
entirely from `plan.json` — GPUI chrome only, no mini-buffers yet.
**Demo:** open the Plan tab, ask the agent (via the M2 server) to draft a plan, and watch
it populate the tab live.

**Architecture:** `PlanView` = `Item` + `SerializableItem` (center-pane tab), a singleton
that retargets to the active thread's plan. It resolves the active thread from
`AgentPanel` (all accessors already `pub` — no upstream patch), finds that thread's
`.plans/<id>.plan.json`, watches it, and re-renders on change. Read-only: it never writes
`plan.json` (the agent writes via the M2 server). Rendering uses `cx.theme()` tokens only.

**Tech stack:** Rust + GPUI (`plan_ui`), `plan_core` (schema/store), `workspace`
(Item/SerializableItem), `agent_ui`/`acp_thread` (active thread), `ui`/`theme`. GPUI views
get smoke tests + manual demo notes; pure logic is test-first.

Build env (GPUI in the path): `export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`
and `PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"`. Commit prefix `[PLAN-M3]`.

---

## Research recap (from `docs/zed-notes.md`, already confirmed)
- `Item` requires only `tab_content_text`; override `tab_content`/`tab_icon` for the
  `Plan — <id>` title + status dot. `SerializableItem` = 5 methods; per-crate DB keyed by
  `(workspace_id, item_id)`; `cleanup` required (item ids are unstable).
- Singleton-retarget pattern: `workspace.items_of_type::<Self>().next()` → `activate_item`
  (ProjectDiff, `git_ui/src/project_diff.rs`).
- Active thread: `workspace.panel::<AgentPanel>(cx)` → subscribe `AgentPanelEvent::
  ActiveViewChanged` → `AgentPanel::active_thread_id(cx)` / `active_agent_thread(cx)` (all
  `pub`). No upstream accessor needed.
- Theme roles all map (`cx.theme().colors()` / `.status()`); prefer `version_control_*` for
  diff/created/modified chrome, `StatusColors` for lifecycle dots.
- Read-only mini-buffers are **deferred** (§6): render as GPUI chrome now, upgrade later.

## Tasks

### T1 — Engage the design contract (no code)
`[PLAN-M3]: (research) read Part IV + plan-ui-compliance before UI`
Read PRD **Part IV §1–§2** (token mapping, surface layout) and §3 (component anatomy) +
§9 (lifecycle→surface state matrix), `docs/design/plan-ui-compliance.md`,
`docs/design/plan-ui-design-spec.md`, and load the `plan-ui-design` skill. Capture the
task-card anatomy, tab status-dot color rules, and lens-switcher spec as notes to build
against. **Acceptance:** a short notes section appended to `docs/zed-notes.md` mapping each
M3 component to its compliance rules + theme roles.

### T2 — plan_core: resolve plans by thread + list (TDD)
`[PLAN-M3]: add plan resolution helpers to plan_core`
- `plan_core`: `list_plan_ids(plans_dir) -> Vec<String>` and `find_by_thread(plans_dir,
  thread) -> Option<Plan>` (the tab's core lookup; also reusable by the panel/hooks).
- **Tests first:** fixtures with 0/1/many plans; correct thread match; missing → None.
- *(Refactor note: `plan_server::hooks::active_plan` can later share this.)*

### T3 — File-watch reload
`[PLAN-M3]: watch plan.json and reload on change`
- Decide the mechanism (open question 1): Zed's `project.fs().watch(path)` vs a
  `plan_core` `notify`-based watcher. Implement so `PlanView` reloads when its bound
  `plan.json` changes and calls `cx.notify()`.
- **Acceptance:** editing the file on disk (or the agent writing it) updates the tab within
  ~250ms; measure latency (§7 risk); fallback to a light poll while `executing` if needed.

### T4 — `PlanView` as `Item` (empty state + frame)
`[PLAN-M3]: add PlanView item with empty state`
- `PlanView` struct: `FocusHandle`, bound `thread_id: Option<ThreadId>`, loaded
  `plan: Option<Plan>`, current `lens`, watcher subscription, plans-dir.
- `impl Item`: `tab_content_text` → `Plan — <id>`; `tab_content` → title + lifecycle status
  dot (Part IV colors); `tab_icon`. `impl Focusable`, `EventEmitter`, `Render` (empty state:
  "no plan — ask the agent to draft one" when unbound/none).
- An open action (`plan_ui::OpenPlan`) + `register` wiring `workspace.register_action`.
- **Acceptance:** the tab opens (singleton — reusing an existing instance), shows the empty
  state; smoke test constructs it in a test window.

### T5 — Thread-following binding
`[PLAN-M3]: follow the active thread`
- On `added_to_workspace`, grab `AgentPanel`, subscribe `ActiveViewChanged`, retarget:
  resolve active `thread_id` → `find_by_thread` → load + watch → `cx.notify()`.
- **Acceptance:** switching threads in the agent panel retargets the tab to that thread's
  plan (or empty state); manual demo note.

### T6 — Tasks lens rendering (compliance-driven)
`[PLAN-M3]: render the Tasks lens from plan.json`
- Task cards from `plan.json`: title, system badge, status dot, steps list, files with C/M
  badges, guard chips, per-task acceptance dots, GATE badge — GPUI chrome + `cx.theme()`
  tokens only (no mini-buffers). Follow `plan-ui-compliance.md` anatomy; run its §13
  verification.
- **Acceptance:** rendering the LED-212 fixture matches the compliance rules (states,
  tokens); pure plan→view-model mapping is unit-tested; manual demo screenshot notes.

### T7 — Lens switcher + Spec/Design styled text
`[PLAN-M3]: add lens switcher and Spec/Design lenses`
- Segmented Spec | Design | Tasks switcher; Spec (goal, scope in/out, acceptance) and
  Design (contracts, decisions, risks) as styled GPUI text. Default lens follows status
  (Part IV).
- **Acceptance:** switching lenses renders each from the fixture; compliance §13 per lens.

### T8 — `SerializableItem` (persist/restore the binding)
`[PLAN-M3]: persist and restore the plan tab`
- Implement `SerializableItem`: a small `PlanDb` table keyed by `(workspace_id, item_id)`
  storing the bound `thread_id`; `deserialize` re-resolves the thread (tolerating a dangling
  binding → fall back to the current active thread); `cleanup` prunes unloaded rows;
  `register_serializable_item::<PlanView>` in `plan_ui::init`.
- **Acceptance:** relaunch restores the tab bound to its thread (or gracefully to the active
  thread); smoke test of the serialize→deserialize round-trip.

### T9 — Milestone note + FORK_DIFF check
`[PLAN-M3]: add M3 milestone note`
- `docs/milestones/M3.md`; update `FORK_DIFF.md` only if an upstream file was touched
  (expected: none — item + action registration live in `plan_ui::init`, already wired in M0).

---

## Definition of done
- [ ] Plan tab opens as a thread-following singleton; retargets on thread switch (F1.1).
- [ ] Reloads live as `plan.json` changes (demo: watch the agent draft into the tab).
- [ ] Tasks lens + Spec/Design render from `plan.json` using only `cx.theme()` tokens
      (zero hardcoded colors); passes `plan-ui-compliance.md` §13 per component.
- [ ] Persists/restores its thread binding across relaunch; `cleanup` implemented.
- [ ] `cargo build -p plan_ui` + smoke tests green; pure logic test-first.
- [ ] `docs/milestones/M3.md` written; `FORK_DIFF.md` accurate (no unplanned upstream touch).

## Open questions
1. **File-watch mechanism** — `project.fs().watch()` (integrates with Zed's watcher, no new
   dep) vs a `plan_core` `notify` watcher (self-contained, reusable by `plan_server`)?
   *Recommend `project.fs().watch()` in `plan_ui`* for M3 (least new surface); revisit if
   latency is poor (§7 fallback: 250ms poll while executing).
2. **`.plans/` location** — resolve under the first visible worktree root? *Recommend yes*
   (matches where the agent/server operate).
3. **M0 hello panel** — keep it until M4 makes the dock panel real, or remove now?
   *Recommend keep* (M4 reconciles; the tab and panel are separate surfaces).
4. **Open-tab affordance** — a command-palette action `plan: open` (+ optional keybinding)?
   *Recommend the action for M3*; status-bar/pill entry is M4.
5. **Thread with no plan** — show the empty state ("no plan — ask the agent to draft one")
   and offer nothing else for M3? *Recommend yes* (drafting is agent-driven via M2).
6. **`find_by_thread` home** — land it in `plan_core` (shared by tab, panel, hooks) rather
   than `plan_ui`? *Recommend `plan_core`.*
