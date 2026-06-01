## Why

When the main agent or subagent calls tools, the LLM receives 400 Bad Request errors from MiniMax with messages like `tool result's tool id(call_function_xxx) not found` or `tool call result does not follow tool call`. These errors prevent agents from completing any multi-step task.

## What Changes

Two distinct bugs were found and fixed in the agentic LLM loop (`src/agent/mod.rs`):

1. **Assistant message was text-only**: The history stored `Message::assistant(text)` which dropped all tool call content. When tool results were subsequently added referencing those call IDs, the LLM had no record of them → "tool id not found".

2. **Spurious user message after tool results**: The `CompletionRequestBuilder` always appends the prompt as a new user message. On turns 2+, this created an invalid sequence `[..., assistant{tool_calls}, user{tool_results}, user{prompt}]` with two consecutive user messages → "tool call result does not follow tool call".

## Fix

- Store the full assistant response (text + tool calls) in history using `Message::Assistant { content: OneOrMany::many(choices) }`
- Manage conversation history entirely ourselves and call `model.completion(request)` directly with a manually constructed `CompletionRequest`, bypassing the builder which always appends a prompt
- System and initial user messages are placed into history once at the start; subsequent turns only append assistant messages and tool results

## Capabilities

### Modified Capabilities

- `llm-client`: Clarify that `tool_result` messages must use tool call IDs from the same LLM context, and that the assistant message containing tool calls must be preserved in history. Also clarify that the conversation structure must not include extra user messages between tool results and the next assistant turn.

## Impact

- `src/agent/mod.rs` — both `process_ticket` and `run_subagent_loop`: fix history management and request construction
- `openspec/specs/llm-client/spec.md` — add clarifying notes on history correctness requirements
- No config changes, no new dependencies
