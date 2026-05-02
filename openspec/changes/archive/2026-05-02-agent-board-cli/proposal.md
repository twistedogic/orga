## Why

LLM agents need a way to participate in team workflows without requiring a separate chat interface. By treating the agent as a first-class member of a shared kanban board, teams can assign work to agents, track progress transparently, and collaborate through the board they already use.

## What Changes

- New CLI binary `orga` that agents invoke as a skill to interact with kanban boards
- TOML config file to identity the agent, board backend, and credentials
- `Board` trait abstraction enabling multiple backend integrations
- Trello backend as the first implementation
- Per-ticket memory system for agents to persist context across invocations
- Commands for listing assigned tickets, reading ticket context, commenting, assigning, moving, creating sub-tickets, and managing checklists

## Capabilities

### New Capabilities

- `cli-commands`: The full command surface exposed by the `orga` binary — ticket list, ticket show, ticket comment, ticket assign, ticket move, ticket create-sub, checklist add, checklist check, memory get, memory set
- `board-abstraction`: The `Board` trait and its contract — the interface all backend adapters must satisfy
- `trello-backend`: Trello API integration implementing the `Board` trait
- `agent-memory`: Per-ticket local memory store allowing agents to persist and retrieve working context across skill invocations
- `config`: TOML config file schema and loading — agent identity, board backend, credentials, memory path

### Modified Capabilities

## Impact

- New Rust binary (replaces the deleted Go project)
- External dependency on Trello REST API
- Local memory storage at `~/.orga/memory.db` (SQLite or flat files)
- No server required — purely CLI + Trello + local memory
