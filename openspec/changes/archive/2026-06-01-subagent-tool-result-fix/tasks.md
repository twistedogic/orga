## 1. Analysis of current code

- [x] 1.1 Confirm that `Message::assistant(text)` drops tool call content from the assistant history entry in both `process_ticket` and `run_subagent_loop`
- [x] 1.2 Confirm that `CompletionRequestBuilder` always appends the prompt as a trailing user message via `chat_history.push(prompt)`
- [x] 1.3 Confirm that on turns 2+, this creates an invalid sequence: `[..., user{tool_results}, user{prompt}]` → "tool call result does not follow tool call"
- [x] 1.4 Confirm both bugs exist in both `process_ticket` and `run_subagent_loop`

## 2. Fix: Store full assistant response in history

- [x] 2.1 Replace `Message::assistant(text)` with `Message::Assistant { content: OneOrMany::many(choices) }` in `process_ticket`
- [x] 2.2 Replace `Message::assistant(text)` with `Message::Assistant { content: OneOrMany::many(choices) }` in `run_subagent_loop`
- [x] 2.3 Verify tool call content is preserved in the assistant history entry

## 3. Fix: Remove spurious user prompt on continuation turns

- [x] 3.1 In `process_ticket`: initialise history with `[system, user]`; construct `CompletionRequest` directly and call `model.completion(req)` instead of using the builder
- [x] 3.2 In `run_subagent_loop`: same approach — history starts with `[system, user]`, continuation turns only append `assistant` + `tool_results`
- [x] 3.3 Verify the message sequence on turn 2+ is `[system, user, assistant{tool_calls}, user{tool_results}]` with no extra user message appended

## 4. Tests

- [x] 4.1 Unit tests pass verifying message construction (plain user vs tool_result)
- [x] 4.2 All 30 existing tests pass after the fix
- [x] 4.3 Live verification: agents complete multi-step tickets without 400 errors

## 5. Documentation

- [x] 5.1 Update the llm-client spec with notes on history correctness requirements
- [x] 5.2 Update proposal.md and design.md to reflect the actual root causes and fixes found during implementation
