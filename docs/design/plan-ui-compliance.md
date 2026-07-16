# Plan — UI Compliance Contract v1.0

**Purpose:** the enforceable companion to `plan-ui-design-spec.md`. The design spec
*describes*; this document *asserts*. Every item is a checkable MUST. Claude Code
verifies the relevant sections after building or modifying any plan_ui component and
records the check in the milestone note. Deviations are flagged, never improvised
around.

**Sources of truth, in order:** (1) this contract → (2) `plan-ui-design-spec.md`
(anatomy detail) → (3) the HTML artifacts in `docs/design/` (visual corroboration
under One Dark). If they disagree, stop and flag.

---

## 0. Global assertions (checked on every component)

- [ ] **G1** Every color resolves through `cx.theme()` by the semantic role in design
  spec §1. Zero hex literals, zero named-color constants in plan_ui.
- [ ] **G2** Fonts come from `ui_font` (UI text) and `buffer_font` (code, SHAs,
  metadata, anchors). Zero hardcoded font names.
- [ ] **G3** Color semantics hold: accent = agent working · modified = needs you ·
  created = done/verified · deleted = failed/blocker · placeholder = idle/pending.
  A new element MUST NOT repurpose a semantic color for a different meaning.
- [ ] **G4** Pulse animation appears only on "needs you now" or "live" elements;
  spinners only on in-progress work; skeleton shimmer only on streaming draft blocks.
  No other motion. `reduced-motion` disables all three.
- [ ] **G5** Every disabled primary action has a tooltip naming the specific blocker
  ("1 blocker open", "1 rehearsal mismatch").
- [ ] **G6** Every agent-caused change renders provenance (Δ chip, "from c1",
  caused-by, receipt) — no unexplained mutations on screen.
- [ ] **G7** Metadata (timestamps, SHAs, rule ids, anchors, sync receipts) is mono,
  9–11px, placeholder/muted. Body text is 12–13px sans.
- [ ] **G8** Interactive elements have hover (element.hover) and focus-visible states.
- [ ] **G9** The shared chip chassis (`chip` / `mono_chip`) renders as a **filled, full-radius
  tinted pill** — `rounded_full`, background = the semantic role color at ~10% opacity, border =
  the role color — not a bare outline. Exception: the ticket `⛓ KEY` chip uses a 4px radius
  (mockup `.tkchip2`). *(Added 2026-07-15 — the mockup's chips are tinted pills; §0 previously left
  chip fill unspecified and the code shipped bare outlines.)*

## 1. Status pill [F1.4]

- [ ] Text format is `◆ Plan <fragment>`; fragment + color follow the **state matrix
  (design spec §9) exactly** for all 13 lifecycle rows.
- [ ] Pulses only in `gate` and `guard-hold` rows of the matrix.
- [ ] Click toggles the plan panel; pill reflects the *active thread's* plan only.

## 2. Plan tab [F1.0/F1.1]

- [ ] Exactly one Plan tab exists per workspace (singleton); selecting a thread
  retargets it; title is `Plan — <id>`.
- [ ] Status dot color/pulse follows the state matrix tab-dot column.
- [ ] Thread without a plan → empty state with the exact copy "no plan — ask the
  agent to draft one".
- [ ] Serialization restores the active-thread binding after relaunch.

## 3. Plan toolbar [F3.6]

- [ ] Contains, in order: status pill · rev indicator (mono) · lens switcher
  (Spec/Design/Tasks) · blocker chip when count > 0 · spacer · contextual actions ·
  primary.
- [ ] Primary action per state matches the matrix; Approve disabled while open
  blockers > 0; Launch disabled while unresolved rehearsal mismatches > 0 (G5
  tooltips apply).
- [ ] Blocker chip: error colors, `⚑ n blocker` copy.

## 4. Ticket card [F2.4b/f/g]

- [ ] One chip per ticket (`⛓ KEY`, mono, accent); status chip colors: To Do muted /
  In Progress info / Done success.
- [ ] Coverage meter: one segment per ticket AC; success = covered, modified =
  needs-update, empty = unmapped; label text matches "ticket AC n/m covered" form.
- [ ] Sync stamp present (mono, 9.5px) with a resync affordance (↻).
- [ ] Drift: card border → modified; changed AC segment → modified; a drift card
  follows showing old (struck, error-bg) → new (created-bg) with Apply-rev action.

## 5. Branch strip [F10.2]

- [ ] Renders only from Launch onward; contains branch←base chip, ↑/↓ counts,
  dirty indicator, PR slot.
- [ ] behind > 0 → amber (drift signal); dirty dot text tracks execution state
  (clean / agent editing / task failed).
- [ ] PR slot: **empty until a PR exists (no placeholder)**; once one does, a purple
  **GitHub-icon button** (`⧉ github` + `PR #n`) that opens the PR on the host.
  Checks/approvals in the chip are v1/F10.4. *(Revised 2026-07-15 — the "PR —"
  placeholder was dropped per developer feedback: show nothing until there's a real,
  clickable PR.)*

## 6. Card chassis (lint, staged rev, rehearsal, gate, amendment, PR, distillation)

- [ ] All use the shared chassis: 8px radius, panel bg, colored caps header band,
  hairline-separated rows, optional mono source note right-aligned in header.
- [ ] Header band color by kind: lint-with-findings warn / lint-pass ok / staged rev
  info / rehearsal info / gate warn / amendment err / PR + distillation purple.
- [ ] Lint rows: severity glyph (⚑ error, ⚠ modified) + mono rule id right;
  auto-fixed rows dimmed with success `✓ auto-fixed` pill; blocker findings gate
  Approve (cross-check §3).
- [ ] Staged revision: header states "plan unchanged until applied"; each hunk shows
  provenance ("from c1") and either `✓ Applied` or Apply/Reject pair; old struck
  error-bg, new created-bg.
- [ ] Rehearsal mismatch rows: amber bg, predicted-file list with the undeclared file
  highlighted, `＋ Add to plan` / `Ignore` actions.
- [ ] Gate evidence rows: mono, ✓ + claim + link; actions include Re-verify all.
- [ ] Amendment: added lines created-bg with `+`, changed lines amber with `~`;
  Accept/Reject only (no auto-apply path).

## 7. Task card + steps + guards [F1.2, F4.5b]

- [ ] Checkbox vocabulary exact: empty = pending · spinner accent = running ·
  ✓ success fill = done · ✕ error fill = failed · ⏸ amber outline = gate.
- [ ] Done task titles strike through; active task border info; failed border error;
  gate border modified.
- [ ] Chips render in order: ticket · system badge · guard summary (review states) ·
  sha chip (`⌥ sha +a −d`, diffstat colored) · tests/evidence · amendment tag.
- [ ] System badge colors: Backend purple / Frontend teal / Testing green / GATE amber.
- [ ] Guard badges on steps: ⛨ approval (amber) and ✋ input (teal); active guard
  pulses amber; cleared guard renders a mono success receipt including timestamp and
  what ran.
- [ ] Input-guard panel: amber border, caps label naming the evidence target
  ("stored as evidence on a1"), input field, primary `Record & continue ⏎`, secondary
  pause option, hint stating the hook blocks until recorded.
- [ ] In multi-ticket plans every task shows its ticket chip; in ticketless plans no
  ticket chips render anywhere.

## 8. Commit rail [§9.1 of design spec]

- [ ] Hidden (gutter collapsed) before launch; visible from guard-hold/executing on.
- [ ] Node vocabulary exact: hollow = pending · success fill = committed · accent
  pulsing = in progress · error fill = failed · amber hollow = gate · purple rotated
  square + branch curve = amendment.
- [ ] Every visible task row has a node once the rail is shown (no gaps in the spine).
- [ ] Foot shows base + ahead count (+ the GitHub-icon PR button once a PR exists — no
  placeholder); node context menu offers revert-task-commit *(the revert action is
  agent-routed and deferred — see M7b)*.

## 9. Review primitives [F3.x]

- [ ] Selection highlight uses theme selection color; floating toolbar shows
  Comment/Suggest/Alternatives/Flag with shortcuts ⌘⇧M / ⌘⇧E.
- [ ] Thread state chips: open amber / pushback red / resolved green; anchor label is
  mono `id ↪ target` (+ file:line when code-anchored).
- [ ] Pushback renders the agent's reasoning with clickable `↗ file:line` cite chips
  and exactly two resolution buttons: [Change anyway] [Keep as planned]; the item
  stays unresolved until one is clicked.
- [ ] Suggestions: old struck error-bg / new created-bg; agent adaptations (vs
  verbatim) MUST be visible as replies.
- [ ] Alternatives: selected card gets accent ring + `✓ selected`; each option carries
  one-line trade-offs.
- [ ] Applied changes leave Δ provenance chips linking back to their thread (G6).

## 10. Plan panel [F1.3, F5.3b]

- [ ] Header always shows the sync receipt (mono; amber variant when agent is a rev
  behind, with "syncs before next task" copy).
- [ ] Pipeline row tints: active info / failed error / gate+guard amber; right-side
  mono note per state (sha, "running", "✋ guard s3", "GATE").
- [ ] Live column: a **caps section header** (kind-colored, mono, per state) precedes the
  activity rows; rows carry a teal mono verb column (plan/edit/run/guard/hook/ev/drift) +
  detail; failures red, completions green, the live row pulses (accent dot, G4).
- [ ] Contextual header action matches the matrix primary for panel-open states.
- [ ] Body layout is responsive to the dock: **pipeline + live side-by-side when docked
  bottom** (wide), **stacked vertically when docked left/right** (narrow) — a left/right dock
  doesn't have room for two columns. *(Added 2026-07-15 per developer feedback; the design's
  "2 columns" assumed the bottom dock.)*

## 11. Intake wizard [F2.0/F2.3b]

- [ ] Questions present one at a time; progress dots (accent current / success done /
  dim pending); upcoming questions render queued (dashed, dimmed, one-line preview).
- [ ] Selecting an option reveals the optional context field; confirm advances;
  per-question skip and "Draft now — use stated defaults" exist; chat answering
  always works.
- [ ] Answered questions collapse to compact cards showing the answer AND a
  provenance line stating where it landed in the plan.
- [ ] Assumption cards are visually distinct from questions, state the default, and
  later flip to "✓ confirmed by …" with source.

## 12. Settings page [F12.2c]

- [ ] Presets rewrite the same keys the rows expose (verifiable: switching presets
  visibly changes row controls).
- [ ] Personal rows may carry ⌖ point-of-use badges; policy section is visually
  distinct (REPO tag, provenance line, Open file / Propose change as PR).
- [ ] Severity selectors are B/W/off with correct colors; 🔒 managed rules disable
  weaker options; per-agent table renders "inherit (value)" for unset cells.
- [ ] The commit-format tester evaluates against real recent commits and shows ✓/✕
  verdicts.

## 13. Verification protocol (run per component, per milestone)

1. **Before building**: read the matching design-spec section + this contract's
   section; list the assertions in the task plan.
2. **After building**: run the fork under **One Dark**, drive the component through
   its states (use the e2e demo HTML step-by-step as the choreography), and check
   every assertion. For lifecycle chrome, walk all applicable rows of the state
   matrix.
3. **Record**: in `docs/milestones/M<n>.md`, list checked sections, any assertion
   that required a mapping decision (e.g., theme role chosen for purple identity),
   and any deviation with rationale — deviations also get flagged to the user before
   merge.
4. **Regression**: when touching a shared element (chassis, chips, checkbox
   vocabulary), re-verify §0 plus every section that consumes it.

## 14. Top-of-plan banners [F1.2c]

*(Added 2026-07-15 — this component exists in the e2e mockup (a colored one-line callout strip at
the very top of the plan body, above `h1.ptitle`) but was previously undocumented in the contract
and design spec. Numbered §14 after the §13 protocol, which keeps its number for back-references.)*

- [ ] A banner strip renders at the top of the plan body, above the title, driven by the derived
  display-state (not raw `Status`). Each banner: flex row, 8px radius, 8×12px padding, 12px text,
  a bold `<b>` lead phrase + detail.
- [ ] Banner kind → color follows §0/§1 roles: **warn** (modified — guard-hold / gate),
  **err** (deleted — task failed), **ok** (created/success — plan complete). Border + bg + text
  all take the kind's role.
- [ ] The four canonical banners match the mockup copy pattern: guard-hold ("✋ Holding at a
  guarded step…"), task-failed ("✕ Task n failed — amendment rev m proposed…"), gate
  ("◆ Gate at task n — staging evidence attached…"), complete ("✓ Plan complete. n/m acceptance ·
  ticket coverage a/b · PR #n").
- [ ] Banners are informational; any action they reference is reached via the matching card/panel
  (no new action surface introduced by the banner itself).
