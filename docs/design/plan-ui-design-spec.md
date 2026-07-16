# Plan — UI Design Specification v1.0

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

### 3.9 Top-of-plan banners [F1.2c]
A colored one-line callout strip at the very top of the plan body, above `h1.ptitle`, announcing
the plan's most important current states. Each banner: flex row, 8px radius, 8×12px padding, 12px
text / 1.5 line-height, a bold lead phrase + detail; border+bg+text take the kind's role — **warn**
(modified: guard-hold / gate), **err** (deleted: task failed), **ok** (created: complete). Driven
by the derived display-state, not raw status. Canonical set: "✋ Holding at a guarded step…" ·
"✕ Task n failed — amendment rev m proposed…" · "◆ Gate at task n — staging evidence attached…" ·
"✓ Plan complete. n/m acceptance · ticket coverage a/b · PR #n". Informational — actions live in
the matching card/panel.

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

**F5.7 attention queue — 2026-07-16 revision:** shipped as a rendered **"NEEDS YOU" section in the
plan panel** (see §4) for the active plan, NOT the pill click-cycle. The pill still conveys the
aggregate needs-you state via its fragment/pulse; enumerating the items lives in the panel section.
Cross-plan cycling, the pill-cycle click, and precise scroll-to-item are deferred (the app follows a
single thread-bound plan today).

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
