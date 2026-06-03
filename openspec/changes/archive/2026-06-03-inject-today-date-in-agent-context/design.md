## Context

`build_user_message()` in `src/agent/context.rs` constructs the user-facing portion of every agent prompt. It is the single shared path for both main agent and subagent context — making it the ideal injection point for any metadata that should be universally available.

Currently the function includes ticket metadata (title, ID, column, URL, creator, assignees) but no temporal context. Agents asked to reason about time must either infer the date from other signals or ask — both are unreliable.

`chrono` is already a direct dependency.

## Goals / Non-Goals

**Goals:**
- Inject today's date into every agent prompt (main and subagent)
- Use a single injection point so there's no risk of divergence

**Non-Goals:**
- Timezone configuration — local system time is sufficient
- Time-of-day precision — date only
- Surfacing date to ticket comments or external outputs

## Decisions

**Inject in `build_user_message()`, not in `build_system_prompt()` or `build_subagent_system_prompt()`**

The user message already contains ticket metadata rendered as markdown key-value pairs. Today's date is operationally similar — it contextualizes the task, not the agent's identity or instructions. Placing it alongside `**ID:**`, `**Column:**`, etc. is the most natural fit.

Alternatives considered:
- System prompt injection: works, but mixes temporal state into the static role definition; system prompts are conceptually stable across invocations
- Separate parameter: unnecessary indirection for a one-liner

**Format: `**Today's date:** YYYY-MM-DD`**

ISO 8601 is unambiguous for LLMs. The label `Today's date` was explicitly chosen (over `Date`) to make the semantics clear — this is the current date at invocation time, not a ticket date.

## Risks / Trade-offs

- [Clock skew] If the host system clock is wrong, agents receive an incorrect date → Mitigation: out of scope; no worse than any other system-time dependency
- [Token cost] One line added per invocation → negligible

## Migration Plan

No migration needed. The change is purely additive to prompt content. No config, schema, or interface changes.
