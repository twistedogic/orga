## Context

The `Board` trait currently exposes `move_ticket(id, list)` as a first-class operation, surfaced both as the `ticket move` CLI subcommand and as the `move_ticket` agent tool. The project's philosophy is that agents are observers and collaborators — they comment, create sub-tickets, and track context — but workflow progression is a human responsibility. Having `move_ticket` in the agent tool registry contradicts this boundary and invites agents to autonomously advance tickets without human judgement.

## Goals / Non-Goals

**Goals:**
- Remove `move_ticket` from the `Board` trait entirely
- Remove the `ticket move` CLI subcommand
- Remove the `move_ticket` agent tool, its dispatch handler, its schema entry, and its tests
- Leave no dead code or unreachable branches behind

**Non-Goals:**
- Adding any replacement mechanism for agents to request a move (out of scope)
- Changing how human users move tickets via the board's native UI
- Modifying any other `Board` trait methods

## Decisions

**Remove from trait, not just from dispatch**

Removing `move_ticket` only from the agent dispatch table would leave a dead method on the `Board` trait and its implementations. Removing it from the trait is the cleaner approach — it enforces the boundary at the type level and avoids accumulating unused code.

*Alternative considered*: Keep the method but guard it in agent dispatch with a permission check. Rejected — this would be complexity without benefit. The method serves no other purpose.

**No deprecation period**

This is an internal CLI tool used by agents and CI, not a public library. A hard removal without a deprecation cycle is appropriate.

## Risks / Trade-offs

- [Breaking change to `Board` trait] → Any external code implementing `Board` will fail to compile. Acceptable — the trait is internal and all implementations are in-tree.
- [Agent skill (SKILL.md) may reference `move_ticket`] → The skill file should be audited and updated to remove any mention of ticket movement as an agent capability.

## Migration Plan

1. Remove `move_ticket` from `Board` trait (`src/board/mod.rs`)
2. Remove implementations in `src/board/trello.rs`, `src/board/linear.rs`, `tests/integration_test.rs`
3. Remove CLI subcommand from `src/main.rs`
4. Remove agent tool from `src/agent/tools.rs` (dispatch branch, struct, function, schema entry, test)
5. Audit `skills/orga/SKILL.md` for any mention of ticket movement and update accordingly
6. Run `cargo build` and `cargo test` to confirm clean compilation
