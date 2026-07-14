# M2 · plan_server + skill — implementation plan (the keystone)

> **For Claude:** REQUIRED SUB-SKILL when executing — superpowers:executing-plans; and
> superpowers:test-driven-development for the tool-logic tasks.

**Status:** awaiting sign-off (not started).
**Feature IDs:** F6.1 (skill), F6.2 (MCP server + tools), F6.3 (hooks), F6.4 (ACP bridge —
partial), F2.0 / F2.3b (question + assumption protocol).
**Goal:** Validate the entire agent contract — skill adherence, hook enforcement, per-task
re-reads — by driving a real task through the plan MCP server in **plain Claude Code**, with
zero GPUI. If the loop doesn't work here, no UI saves it (PRD Part II §4).

**Architecture (revised from PRD Part II §1):** the server is **Rust**, not TypeScript. A
new `plan_server` **binary crate** uses the `rmcp` crate (v2.2.0; stdio transport) and
depends on `plan_core`, so the schema, atomic store, validation, and (later) lint have a
**single source of truth** — no re-implementation, no drift. Decided after the M0 finding
that Zed does all its own agent tooling in Rust, plus a passing rmcp connectivity spike
(see `docs/zed-notes.md` / the spike record). `plan.json` remains the only IPC: tools mutate
it via `plan_core::store` (atomic temp+rename); `plan_get` always reads fresh from disk.

**Tech stack:** Rust, `rmcp` (server/macros/transport-io), `tokio` (standalone binary only —
never linked into Zed), `plan_core`, `serde`/`serde_json`, `anyhow`. Skill = Markdown.
Hooks = subcommands of the `plan_server` binary (reuse `plan_core`) invoked by Claude Code.

**Dogfood target:** register `plan_server` in **Claude Code's own MCP config** (`.mcp.json` /
`claude mcp add`) and run a task in the Claude Code CLI, watching `.plans/<id>.plan.json` in
a split. (Forwarding through Zed's `context_servers` over ACP is M3+.)

Commit convention: one commit per task, imperative, prefixed `[PLAN-M2]`. Test-first for tool
logic; hooks and the skill get invocation tests + a recorded dogfood transcript.

Build env: `PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"`. `plan_server` is pure Rust (no
GPUI/Metal), so no `DEVELOPER_DIR` needed for its build/tests.

---

## Tasks

### T1 — `plan_server` crate scaffold + rmcp stdio server
`[PLAN-M2]: scaffold plan_server crate with rmcp stdio server`
- New bin crate `crates/plan_server` (`[[bin]] path = "src/plan_server.rs"`), workspace
  member; deps `rmcp` (server, macros, transport-io), `tokio`, `plan_core`, `anyhow`,
  `serde`/`serde_json`. Port the spike's `ServerHandler` skeleton with a `plan_ping` tool.
- **Acceptance:** `cargo build -p plan_server`; driving the MCP handshake over stdio returns
  the `initialize`/`tools/list` responses (reuse the spike's `handshake.jsonl` as a test).

### T2 — Day-one hook enforcement spike (§7 risk retirement)
`[PLAN-M2]: verify Claude Code PreToolUse deny + SessionStart inject`
- Before building enforcement, confirm the mechanism: a trivial `PreToolUse` hook that
  **denies** an `Edit`/`Bash` and a `SessionStart` hook that **injects** text. Configure in
  Claude Code, run once, confirm the deny actually blocks and the injection appears.
- **Acceptance:** recorded transcript showing a blocked tool call and injected context. **If
  hooks can't deny/inject, STOP** — this invalidates F6.3 and the enforcement design.

### T3 — `plan_get` (fresh read) + `plan_create` (TDD)
`[PLAN-M2]: add plan_get and plan_create tools`
- `plan_get` → `plan_core::load` (never cached); `plan_create` → build a minimal `Plan` and
  `plan_core::save`. Operate on `.plans/<id>.plan.json` under the server's working dir.
- **Tests (first):** in-process tool handlers against a temp `.plans` dir — create then get
  round-trips; `plan_get` reflects an external edit to the file (proves no caching).

### T4 — mutation tools (TDD)
`[PLAN-M2]: add plan mutation tools`
- `plan_update_section`, `plan_add_task`, `task_update`, `plan_set_status`,
  `plan_answer_question` — each loads fresh, mutates the typed model, bumps `rev`, stamps
  `history[]`, and saves atomically (sync receipts fall out of this per PRD §1).
- **Tests (first):** each tool's effect on a temp plan (e.g. `task_update` flips status +
  appends a timeline entry + bumps rev; `plan_answer_question` records the answer).

### T5 — planning SKILL.md (F6.1)
`[PLAN-M2]: add planning-mode skill`
- `plan-agent/skills/planning/SKILL.md`: lifecycle protocol, clarifying-questions-first
  (F2.0) + non-blocking assumptions (F2.3b), revision/pushback protocol, per-task loop
  (`plan_get` → work → tests → `task_update` + evidence), re-read-before-every-task, git/
  evidence rules (references only — enforcement is hooks).
- **Acceptance:** skill loads in Claude Code; a dry run shows the agent following the
  question protocol and calling `plan_get` before a task.

### T6 — hooks (F6.3)
`[PLAN-M2]: add Claude Code hooks`
- Implemented as `plan_server` subcommands (reuse `plan_core`), invoked by Claude Code:
  - `SessionStart` → `plan_get` + resume briefing.
  - `UserPromptSubmit` / `PreCompact` → re-inject the current plan.
  - `PreToolUse` → block code edits when no plan is executing; hold on step guards; enforce
    commit format (later milestones extend this).
  - `Stop` → loop guard.
- **Tests:** invoke each subcommand with fixture plan states and assert the block/allow/inject
  decision (exit code + emitted JSON) — no Claude Code needed for the unit level.

### T7 — Claude Code wiring fragment
`[PLAN-M2]: add Claude Code MCP + hooks settings fragment`
- `plan-agent/settings-fragment.json`: `mcpServers` entry pointing at the `plan_server`
  binary, plus the hooks wiring. Document `claude mcp add` / `.mcp.json` setup.
- **Acceptance:** following the doc, `claude` lists the plan tools and the hooks fire.

### T8 — Dogfood a real task (the whole point) (§7 skill-adherence)
`[PLAN-M2]: dogfood a task end-to-end in plain Claude Code`
- Run a small real task through Claude Code + plan_server + skill + hooks, watching
  `.plans/<id>.plan.json`. Tune `SKILL.md` + re-inject until a full task runs clean **3×**
  (the §7 adherence bar).
- **Acceptance:** a recorded transcript + the resulting `plan.json`; per-task re-read and
  hook enforcement observed. Notes captured in the milestone note.

### T9 — Milestone note
`[PLAN-M2]: add M2 milestone note`
- `docs/milestones/M2.md`: what works, the dogfood findings, deferred tools (review/revision
  tools → M5), surprises.

---

## Definition of done
- [ ] `cargo build -p plan_server` + `cargo test -p plan_server` green; tool logic test-first.
- [ ] MCP handshake works over stdio; all seven M2 tools present and correct against fixtures.
- [ ] Hooks can deny (PreToolUse) and inject (SessionStart) — verified in Claude Code (T2).
- [ ] A real task runs clean 3× in plain Claude Code; `plan.json` updates atomically with
      rev bumps + history stamps (sync receipts).
- [ ] `plan_server` reuses `plan_core` (no duplicated schema/store); nothing in Zed depends
      on `plan_server`.
- [ ] `docs/milestones/M2.md` written. No upstream Zed files touched (FORK_DIFF unchanged —
      `plan_server` is additive; Zed wiring is M3+).

## Open questions
1. **Hook language** — implement hooks as `plan_server` **subcommands** (one Rust binary,
   reuses `plan_core`), or as separate small scripts? *Recommend subcommands* (single source
   of truth, no second toolchain; Claude Code invokes `plan_server hook pretooluse`).
2. **`tokio` in the workspace** — `plan_server` (a standalone binary) pulls `tokio`; it is
   never linked into the Zed app. *Recommend accept* (isolated to the binary; confirm you're
   ok with it in `Cargo.lock`).
3. **Update PRD Part II §1?** It still says TypeScript. *Recommend* I add a short "revised:
   Rust + rmcp, see M2" note there so the source-of-truth doc matches reality. Your call.
4. **Which real task to dogfood (T8)** — a tiny change in this fork (e.g. a `plan_core`
   helper) so we exercise the loop on real code? *Recommend yes* — dogfooding on the fork
   itself is the fastest spec validator (PRD §8).
5. **`plans` directory location** — server operates on `.plans/` under its working directory
   (the project root Claude Code runs in). Confirm that's the intended convention (matches
   PRD §11 `.plans/<id>.plan.json`).
