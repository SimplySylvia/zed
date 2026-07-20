# plan-agent

The agent-side of the Plan feature: the `planning-mode` skill, Claude Code hooks, and the
wiring that connects Claude Code to the `plan_server` MCP server. During M2 this runs
against **plain Claude Code** (no Zed); Zed forwards the same server over ACP in later
milestones.

## Layout
- `skills/planning/SKILL.md` — the planning protocol the agent follows.
- `settings-fragment.json` — reference wiring (MCP server + hooks).

## Setup

1. **Build the server** (from the fork root):
   ```sh
   cargo build -p plan_server            # -> target/debug/plan_server
   ```
   Use an absolute path to that binary wherever `<PLAN_SERVER_BIN>` appears below.

2. **Register the MCP server** — either add the `mcpServers.plan` block to the project's
   `.mcp.json`, or run:
   ```sh
   claude mcp add plan /abs/path/to/target/debug/plan_server
   ```
   For a throwaway session you can instead pass `--mcp-config settings-fragment.json`
   (after substituting the path) plus `--strict-mcp-config`.

3. **Register the hooks** — copy the `hooks` block from `settings-fragment.json` into
   `.claude/settings.json` (project) or `~/.claude/settings.json` (global), substituting
   `<PLAN_SERVER_BIN>`. The hooks:
   - inject the active plan at session start and on each prompt (`SessionStart`,
     `UserPromptSubmit`, `PreCompact`);
   - **block** `Edit`/`Write`/`MultiEdit`/`NotebookEdit` unless a plan is `executing`
     (`PreToolUse`);
   - `Stop` is a placeholder (real loop guard lands with execution, M6).

4. **Make the skill discoverable** — point Claude Code at `skills/planning/SKILL.md`
   (project `.claude/skills/` or your skills directory).

5. **Plans live in `.plans/`** under the directory Claude Code runs in
   (`<id>.plan.json`). `PLAN_PLANS_DIR` overrides the location (used by tests).

## Verifying
- `plan_server` speaks MCP: `cargo test -p plan_server`.
- Hooks decide correctly: `cargo test -p plan_server --test hooks`.
- End-to-end in Claude Code: ask it to draft a plan, then to edit code before launching —
  the edit should be blocked by the `PreToolUse` hook.
