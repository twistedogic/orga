## Context

Agents failed with 400 Bad Request errors from the MiniMax LLM API on every multi-step ticket. Two distinct error messages were observed:

```
invalid params, tool result's tool id(call_function_xxx) not found (2013)
invalid params, tool call result does not follow tool call (2013)
```

## Root Causes

### Bug 1: Tool calls dropped from assistant history message

In both `process_ticket` and `run_subagent_loop`, the assistant response was stored as:

```rust
history.push(Message::assistant(text));  // only stores text
```

`Message::assistant(text)` creates `Message::Assistant { content: OneOrMany::one(AssistantContent::text(text)) }`. It discards all `AssistantContent::ToolCall` items from the response. On the next turn, `tool_result` messages reference tool call IDs that don't appear in the history the LLM sees → "tool id not found".

### Bug 2: Extra user message appended after tool results

`CompletionRequestBuilder::new(model, prompt)` always appends the prompt as a new user message at the end of the history in `build()`:

```rust
chat_history.push(prompt.clone());
```

On turns 2+, the history ended with `user{tool_results}`, and then the prompt added another `user{...}` message. The final sequence sent to MiniMax was:

```
[system, user(initial), assistant{tool_calls}, user{tool_results}, user(prompt)]
```

Two consecutive user messages after the assistant is invalid → "tool call result does not follow tool call".

## Goals / Non-Goals

**Goals:**
- Fix both history bugs so multi-step agentic loops work correctly
- Apply fix to both main agent loop and subagent loop

**Non-Goals:**
- Changing agent behavior, tool interface, or configuration
- Adding new capabilities

## Decisions

### Decision: Store full assistant response in history

Replace `Message::assistant(text)` with `Message::Assistant { content: OneOrMany::many(all_choices) }` to preserve both text and tool call content in the assistant turn.

**Rationale**: The LLM needs to see its own tool calls in history for subsequent `tool_result` messages to be valid.

### Decision: Bypass `CompletionRequestBuilder`, use `model.completion(req)` directly

Construct a `CompletionRequest` manually with the full history and call `model.completion(req)` directly. System and initial user messages are placed into history once at the start. Subsequent turns only append assistant messages and tool results — no extra user prompt.

**Rationale**: The builder always appends a prompt as a user message, making it structurally impossible to avoid duplicate user messages on continuation turns. Direct construction gives full control over message ordering.

**Alternative considered**: Pass an empty string as the prompt. Rejected — it still appends `user{text:""}` which is invalid.

**Alternative considered**: Add a "Continue" message as a user turn after tool results. Rejected — any additional user message between tool results and the next LLM turn is invalid per the MiniMax/OpenAI tool-call protocol.

## Spec Changes

### llm-client spec: Clarify history correctness requirements

Add notes under the tool-call requirement:

1. The assistant message containing tool calls MUST be stored in history in full (including all `tool_call` content, not just text).
2. After tool results, no additional user message may appear before the next LLM call — the conversation must go `[..., assistant{tool_calls}, user{tool_results}]` with the next LLM call reading directly from this state.
3. `tool_result` IDs must reference tool calls from the same LLM conversation context.

## Risks / Trade-offs

- `CompletionRequest` struct fields are public in rig-core; if upstream adds required fields this will break at compile time (acceptable)
- No behavioral risks: fix only affects message ordering/content in history

## Open Questions

None.
