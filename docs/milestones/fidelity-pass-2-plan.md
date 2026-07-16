# Fidelity pass 2 — plan

> **For Claude:** REQUIRED SUB-SKILL: use `superpowers:executing-plans` (or
> `subagent-driven-development`) to implement task-by-task. All `plan_ui` work is governed by
> the **plan-ui-design skill** + `docs/design/plan-ui-compliance.md` — run its §13 protocol per
> component. Colors ONLY via `cx.theme()` by the §1 semantic-role table; fonts ONLY via
> `ui_font`/`buffer_font`; no hex, no font names.

**Goal:** close the visual-finish gap between the current `plan_ui` and the approved mockup
(`docs/design/planning-mode-demo.html`) so the built surface reads as *finished*, not wireframe —
without inventing design and without building net-new v1 features.

**Architecture:** additive, all in `crates/plan_ui/` (fork discipline — no upstream files, no
`FORK_DIFF.md` change). Three phases: **P1** shared chassis/token fixes that propagate across the
whole surface; **P2** render data the schema already carries; **P3** rendering-only structural
adds that need no live-agent or new interaction backend. Where the mockup shows something the
contract doesn't yet assert, the **contract is updated first** (Task 0, your sign-off) so code and
docs stay in lockstep (design-change protocol).

**Tech stack:** Rust + GPUI; `plan_core` schema/status; existing `plan_ui` helpers
(`chip`, `mono_chip`, `card_shell`, `card_row`, `primary_button`, `pulse`, `status_dot`).

**Commit prefix:** `[PLAN-FP2]`, one commit per task, imperative. **No `Co-Authored-By` trailer.**
Do not stage `.rules` / `.claude/` / `.plans/` / `crates/plan_server/LICENSE-GPL`.

---

## Scope

**In scope (this pass):**
- **P1 — chassis & tokens:** filled/tinted pill chips (99px), solid-accent primary button, segmented
  lens switcher, branch-strip container, ticket-card 8px radius, coverage-meter segment borders/fills,
  rail-spine inset.
- **P2 — render existing data:** sha diffstat + accent color, tests count + failed variant, guard
  receipts (timestamp + what-ran), evidence chip, amendment tag, GATE badge, anchor `file:line`,
  sync-receipt amber "rev behind" variant, `shows_git` window (branch strip + rail at Approved/Done),
  rail-foot ahead green, base-segment dim, drift attribution, blocker-chip copy/style.
- **P3 — rendering-only structural adds:** toolbar status pill, top-of-plan banner strip,
  gate-evidence card body (evidence rows + Re-verify all + Request changes), live-column caps header +
  pulsing live row + curated verb vocabulary, comment thread as avatar card + multi-message,
  input-guard panel body, lint card as a §6 chassis member.

**Explicitly OUT of scope (flagged as v1 features, not polish — do not build here):**
- Selection highlight + floating toolbar + the comment/suggest/**alternatives** authoring loop (F9.2).
- **ui-preview gallery** / mini-tables (F4.6).
- **Intake / Q&A wizard** (§11) — net-new intake surface.
- **Rehearsal card + Launch gating** (F9.4).
- **Pushback authoring** flow (F3.4c) — but P2 *does* add the `pushback`→red state-chip mapping so an
  existing pushback comment isn't mis-colored gray.

## Decisions (confirmed with developer)
1. **Render-time display-state.** Pill fragments, tab-dot pulse, and banners derive a richer
   `DisplayState` computed from plan contents (open blockers, staged rev, failed task, gate) at render
   time. **No `Status` enum / schema change.**
2. **Contract updated first** for mockup elements not yet asserted (banner strip, live-column verbs,
   gate-evidence rows) — Task 0.

---

## Task 0 — contract + spec edits (design-change protocol, needs sign-off)

**Files:** Modify `docs/design/plan-ui-compliance.md`, `docs/design/plan-ui-design-spec.md`.

Only two things were genuinely under-documented (live-column verbs are already in design-spec §4;
gate-evidence rows + Re-verify all are already in design-spec §3.4 + contract §6 — those are pure
implementation gaps, no doc change):
- **New contract §14 + design-spec §3.9 banners** — top-of-plan banner strip (warn/err/ok kinds;
  `<b>` lead; 8px radius) driven by `DisplayState`; assert the four mockup callouts (guard-hold /
  task-failed / gate / complete). This component lived only in the mockup HTML.
- **Contract G9 (chip chassis)** — the shared chip is a filled, `rounded_full` tinted pill (bg = 10%
  of role color, border = role color), not a bare outline; ticket `⛓ KEY` chip keeps 4px radius.
- **Contract §10 (tighten)** — add the "caps section header precedes activity rows" clause (the verb
  vocabulary + live-row pulse were already asserted).

**Step 1 (done):** edits made to both docs. **Step 2:** present the diff for sign-off. **Step 3:**
commit `[PLAN-FP2] assert banners, chip-chassis fill, and the live-column header`.
**Gate: do not start P1 until Task 0 is signed off.**

---

## Phase 1 — chassis & tokens (do first; highest leverage)

### Task 1 — filled tinted chip chassis
**Files:** Modify `crates/plan_ui/src/plan_view.rs:1752` (`chip`) and
`crates/plan_ui/src/plan_ui.rs:632` (`mono_chip`). **Test:** `plan_ui` smoke (compiles + renders).

Change both from `rounded_sm` + border-only to a filled pill:
```rust
// chip(): filled, full-radius pill (mockup .chip/.sysbadge/.guardb — 99px, tinted bg)
div()
    .px_1p5()
    .rounded_full()
    .border_1()
    .border_color(color)
    .bg(color.opacity(0.1))
    .text_color(color)
    .text_size(px(10.))
    .child(text.into())
```
Apply the same `rounded_full` + `bg(color.opacity(0.1))` to `mono_chip` (keep the `buffer_font`
label). Regression (contract §13.4): re-verify §0 + every chip consumer (task card, ticket, guard,
tests, lint rows, toolbar blocker, staged-rev provenance).

**Steps:** edit → `cargo build -p plan_ui` → visual spot-check deferred to §13 → commit
`[PLAN-FP2] fill and round the shared chip chassis`.

### Task 2 — solid-accent primary button
**Files:** `plan_view.rs:1747` (`primary_button`). Design-spec §3.1 (`.primary` = accent fill, dark
text). Change `ButtonStyle::Tinted(TintColor::Accent)` → `ButtonStyle::Filled` with an accent
background (mirror how `git_ui`/`agent_ui` render their solid primary — copy that idiom rather than
inventing). Commit `[PLAN-FP2] make the primary action a solid accent fill`.

### Task 3 — segmented lens switcher
**Files:** `plan_view.rs:380-385` (lens row in `render_header`). Wrap the three `lens_button`s in a
container: `.bg(panel)`, `.border_1().border_color(border_variant)`, `.rounded_md()` (7px), inner
`p_0p5()`. Copy the `ToggleButtonGroup`/segmented idiom from an existing crate if one exists. Commit
`[PLAN-FP2] give the lens switcher a segmented-control chassis`.

### Task 4 — branch-strip container + base dim + foot color
**Files:** `plan_view.rs:465` (`render_branch_strip`), `:651` (`render_rail_foot`). Add the container
chassis to the strip (`.bg(panel).border_1().border_color(border_variant).rounded_lg().px_3().py_1()`);
render base segment `Color::Placeholder` distinct from the branch name; color the `↑n ahead` token
`Color::Created` (green) in the foot. Commit `[PLAN-FP2] frame the branch strip and color its base/ahead`.

### Task 5 — ticket-card radius + coverage-meter segments
**Files:** `plan_view.rs:1885` (`.rounded_md()`→`.rounded_lg()`), `coverage_meter` (~`:1835`). Empty
segment: add editor `.bg()` + `border_variant`; covered/needs-update: add matching role border to the
fill; radius to 3px. Commit `[PLAN-FP2] tighten ticket-card radius and coverage segments`.

### Task 6 — inset the commit-rail spine
**Files:** `plan_view.rs:628-638` (`render_task_list` spine). Replace `top_0().h_full()` with an inset
so the 2px line begins at the first node and ends at the last (the code comment already promises this;
implement it — measure node offset ~`top:10px`, `bottom` past the last node). If exact geometry needs
per-row measurement GPUI can't cheaply give, STOP and flag. Commit `[PLAN-FP2] inset the commit-rail spine to its terminal nodes`.

---

## Phase 2 — render data we already carry

### Task 7 — sha chip: accent color + diffstat
**Files:** `plan_view.rs:769-775`. Recolor the `⌥ sha` chip to `status.accent`; append the diffstat from
`Artifacts.diffstat` (`+a` in `created`/vc-added green, `−d` in `deleted` red). Contract §7. Commit
`[PLAN-FP2] color the sha chip accent and render its diffstat`.

### Task 8 — tests chip count + failed variant + evidence chip
**Files:** `plan_view.rs:776-778`. Read `Artifacts.tests`: render `✓ n tests` (green) / `✕ n tests`
(red failed variant); add the teal `⛨ guards n/n` evidence chip. Contract §7. Commit
`[PLAN-FP2] render tests count, failed variant, and evidence chip`.

### Task 9 — guard receipt: timestamp + what-ran
**Files:** `plan_view.rs:1386-1391` (`render_step` cleared branch). Use `Guard.cleared_at` + the
recorded command/result to render `⛨ approved <ts> · <what-ran> ✓` (mono, success). Contract §7 + G6.
Commit `[PLAN-FP2] show guard-receipt timestamp and what ran`.

### Task 10 — amendment tag + GATE badge on task card
**Files:** `render_task_card` chip row (`plan_view.rs:760-778`). Append a purple `◆ amendment · rev n`
tag when `exec::amendment_count` > 0; render the amber `GATE` `sysbadge` alongside the system badge on
gate tasks. Contract §7. Commit `[PLAN-FP2] add amendment tag and GATE badge to task cards`.

### Task 11 — anchor label `file:line`
**Files:** `plan_view.rs:1344` (`render_comment` anchor). Append `+ file:line` from the comment's code
anchor when present. Contract §9. Commit `[PLAN-FP2] append file:line to code-anchored comment labels`.

### Task 12 — `shows_git` window (branch strip + rail at Approved/Done)
**Files:** `plan_view.rs:419-423` (`shows_git`). Include `Approved` and `Done` so the strip flips to the
live PR chip and the rail shows the final commit node + PR foot. Contract §5/§8. **Logic → test-first**
in `plan_ui` (a unit test asserting `shows_git` for each status). Commit
`[PLAN-FP2] show the branch strip and rail at Approved and Done`.

### Task 13 — sync-receipt amber "rev behind" variant
**Files:** `plan_ui.rs:597-602` (`sync_receipt`) + its render site (`:396-401`). When the agent is a rev
behind, render amber with "syncs before next task" copy; prefix "agent". Contract §10. **Test-first**
(receipt text/variant given rev delta). Commit `[PLAN-FP2] add the rev-behind sync-receipt variant`.

### Task 14 — pushback state-chip color + blocker-chip copy/style
**Files:** `comment_state_color` (`plan_view.rs:~1572`) add a `pushback`→`deleted` (red) arm;
blocker-chip copy → singular `⚑ n blocker` (`:388-389`); the chip already inherits Task 1's filled
style. Contract §3/§9. Commit `[PLAN-FP2] map pushback to red and fix blocker-chip copy`.

### Task 15 — drift-card attribution + Apply-rev action
**Files:** `render_drift_card` (`plan_view.rs:1953`). Add `edited by @who <when>` to the header (G6) and
an `Apply rev n` action that routes through the existing global staged-revision apply path (do NOT add a
new apply mechanism; reuse it). Contract §4. If the drift model lacks author/when, STOP and flag. Commit
`[PLAN-FP2] add drift-card attribution and Apply-rev action`.

---

## Phase 3 — rendering-only structural adds

### Task 16 — `DisplayState` derivation (foundation for 17 + 18)
**Files:** new derivation in `plan_ui.rs` or `plan_view.rs` (a pure fn `display_state(plan) ->
DisplayState`). Computes matrix sub-states (intake-count, lint-open, rev-staged, review-counts,
rehearsed, task-failed, gate, done+PR) from plan contents. **Test-first** — a table test mapping seed
plans → expected `DisplayState`. Commit `[PLAN-FP2] derive a render-time DisplayState from plan contents`.

### Task 17 — toolbar status pill + matrix-accurate fragments
**Files:** `render_header` (`plan_view.rs:352-368`), `plan_pill.rs` (`pill_fragment`). Add the colored
caps status pill to the toolbar (reuse the tinted-pill chassis); drive fragment text + color from
`DisplayState` to match the §9 matrix (incl. the failed/deleted row). Keep the body `h1.ptitle` on the
plan (remove the redundant toolbar title). Contract §1/§3 + matrix. **Test-first** on fragment mapping.
Commit `[PLAN-FP2] add the toolbar status pill and matrix-accurate fragments`.

### Task 18 — top-of-plan banner strip
**Files:** new `render_banners(plan) -> Option<AnyElement>` in `plan_view.rs`, called at the top of the
plan body (`~:2142`). Render warn/err/ok banners from `DisplayState` (guard-hold / task-failed / gate /
complete) per the new contract §14: flex, 8px radius, `<b>` lead, role bg+border+text. Commit
`[PLAN-FP2] add the top-of-plan banner strip`.

### Task 19 — gate-evidence card body
**Files:** `render_gate_hold` (`plan_view.rs:1155`). Add mono evidence rows (`✓` claim + link) from the
gate's staged evidence, and the actions `Re-verify all` + `Request changes` beside `✓ Approve & finish`.
Contract §6 (updated). If the evidence source isn't in the model, STOP and flag. Commit
`[PLAN-FP2] flesh out the gate-evidence card body and actions`.

### Task 20 — live column: caps header + pulsing live row + verb vocabulary
**Files:** `plan_ui.rs` live-column build (`~:474-593`). Add a caps section header per state; pulse the
running/live row + add the accent `fdot`; map ACP tool-kinds → the curated verb vocabulary
(`plan/edit/run/guard/hook/ev/drift`). Contract §10 (updated). Commit
`[PLAN-FP2] add the live-column header, pulse, and verb vocabulary`.

### Task 21 — comment thread as avatar card + multi-message
**Files:** `render_comment` (`plan_view.rs:1308-1366`). Reshape from a one-line row to an editor-bg card
(8px radius, indented, max-width): render the full `thread` (not just `first()`), each message with a
14px avatar (`u` amber / `a` purple) + muted body. Contract §9 + design-spec §3.7. Commit
`[PLAN-FP2] render comments as avatar-bearing multi-message cards`.

### Task 22 — input-guard panel body
**Files:** `guard_controls` (`plan_view.rs:839-874`). Add the caps evidence-target label (from
`Guard.evidence_for`), an input field, the secondary `Pause — I'll verify later`, and the hook hint;
keep the primary `Record & continue ⏎`. Contract §7. If a real text input needs an editor entity that's
heavier than a polish task, render the field chrome and STOP/flag the wiring. Commit
`[PLAN-FP2] build out the input-guard panel body`.

### Task 23 — lint card as a §6 chassis member
**Files:** new `render_lint_card` in `plan_view.rs`, fed by the same `lint::reconcile` findings currently
folded into comments. Header band warn (findings) / ok (pass); rows = severity glyph + finding + mono
rule id; auto-fixed rows dimmed + `✓ auto-fixed` pill; a pass card ("⚙ Plan lint · passed · n/n rules").
Contract §6. Decide with developer whether findings *also* stay as comments or move entirely to the card
(flag). Commit `[PLAN-FP2] render lint findings as a dedicated card`.

---

## Testing
- **Logic (test-first, `cargo test -p plan_ui`):** `shows_git` window (T12), sync-receipt variant (T13),
  `DisplayState` derivation (T16), pill-fragment mapping (T17). These are pure fns — table tests.
- **Views:** smoke coverage (compiles + renders under a seed plan); the real acceptance is the §13
  visual check.
- **Regression:** after Task 1 (shared chips) and Task 2 (primary), re-verify §0 + every consumer.
- Run `./script/clippy` before each commit; `cargo build -p plan_ui` for fast loops; full `cargo build -p
  zed` before the §13 visual check.

## §13 verification (developer, under One Dark)
Drive the seed plan through its states using the mockup as choreography and check, per phase:
- **P1:** chips are filled tinted pills; primary is a solid accent fill; lens switcher is a segmented
  group; branch strip is a framed pill; rail spine starts/ends at its nodes.
- **P2:** sha chip is accent + shows `+a −d`; tests show count/failed; guard receipt shows ts + what-ran;
  amendment tag + GATE badge present; branch strip + rail persist at Done with the PR chip; sync receipt
  goes amber when the agent is behind.
- **P3:** toolbar status pill matches the matrix row-for-row (incl. failed); banners render per state;
  gate card lists evidence + Re-verify all; live column has a header, a pulsing live row, and curated
  verbs; comments are avatar cards; input-guard panel is the full evidence surface; lint card renders.
Record checked sections + any theme-role mapping decisions in `docs/milestones/fidelity-pass-2.md`.

## Open questions (resolve before/at the relevant task)
1. **T6 rail geometry** — can we inset the spine to exact node centers without per-row measurement? If
   not, propose the closest cheap approximation and flag.
2. **T15 drift model** — does `tickets::TicketDrift` carry author + timestamp? If not, attribution is
   agent-supplied (crate is time-free) → flag, render what exists.
3. **T22 input field** — is a lightweight editor entity acceptable in a polish pass, or render field
   chrome only and defer wiring?
4. **T23 lint** — findings in the card only, or card + comments?
5. **Commit prefix** — `[PLAN-FP2]` ok, or keep fidelity-pass-1's plain `[PLAN]`?
