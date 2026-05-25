## Why

orga currently requires an external LLM skill (e.g. a Crush skill) to drive it — the CLI is purely on-demand. This means the agent only acts when a human or orchestrator explicitly invokes it. Adding a native agent loop makes orga self-driving: it polls the board, processes tickets with an embedded LLM using tool calling, and hands work back — all without any external scheduler or skill wrapper.

## What Changes

- New `orga agent` subcommand that runs a poll-act-sleep loop (daemon) or a single pass (`--once`)
- Embedded LLM client via the `rig` crate; provider and model are configurable
- Tool-calling interface: the LLM drives board actions (comment, move, assign, artifact, memory, etc.) by calling typed tools; orga executes them and feeds results back
- `done(comment?)` tool maps to `return_ticket` — the agent hands the ticket back to its creator when work is complete
- `skip()` tool — leave ticket untouched for this cycle
- `dry_run` mode — log what would happen without executing any board mutations
- Max-actions-per-ticket cap enforced per cycle
- `[llm]` config section: provider, model, api_key, optional endpoint override, poll_interval_secs, max_actions_per_ticket
- Artifact interaction: agent can read and write artifacts within a ticket cycle, just like memory

## Capabilities

### New Capabilities

- `agent-loop`: Self-driving poll-act loop; polls assigned tickets, builds LLM context, executes tool calls, respects safety limits (max actions, dry run)
- `llm-client`: Embedded LLM client wrapping `rig`; supports Anthropic and OpenAI-compatible providers with configurable endpoint override
- `agent-tools`: The typed tool set exposed to the LLM during a ticket cycle (comment, move, assign, create_sub, checklist ops, memory, artifact read/write, compact, done, skip)

### Modified Capabilities

- `config`: New `[llm]` section added to `AppConfig` with provider, model, api_key, endpoint, poll_interval_secs, max_actions_per_ticket, dry_run flag

## Impact

- New dependency: `rig-core` (async LLM client); requires adding `tokio` async runtime for the agent loop
- `config.rs`: extended with `LlmConfig` struct
- `main.rs`: new `Agent` subcommand with `--once` and `--dry-run` flags
- New module: `src/agent/` — loop, context builder, tool dispatcher
- Existing board/memory/artifact/compaction APIs are reused unchanged — the loop is an orchestration layer on top
- No breaking changes to existing CLI commands
