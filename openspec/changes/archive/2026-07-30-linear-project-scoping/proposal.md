## Why

orga's Linear backend treats a Linear **Team** as the board primitive: every
issue the agent sees, and every workflow state (`list_columns`), is scoped to a
single `team_id` in `[linear]`. For a single team that is the whole world. But
a team frequently holds several concurrent **Projects** (longer-running
initiatives that group issues), and an orga agent assigned across that team has
no way to narrow its working set to one project. The agent's queue mixes
unrelated initiatives, so the sleep-time/dispatch logic, the `last_commenter`
filtering, and the agent's own focus all span projects the operator never
intended it to touch.

A Linear Project is a *grouping* axis, not a *state* axis — issues inside a
project still carry their team's workflow states, and a project has no
per-issue columns. That makes project scoping an additive filter on the agent's
inbound queue, layered on top of the existing team scoping, rather than a
replacement for it.

## What Changes

- **Add an optional `project_id` field to `[linear]`.** `LinearConfig` gains
  `project_id: Option<String>`. Unset = today's behavior exactly; no migration,
  no behavior change for existing configs. (`src/config.rs:20`.)
- **Filter `list_assigned` by project when set.** The `issues(filter: …)` query
  gains a `project: { id: { eq: "<project_id>" } }` clause, intersected with
  the existing `team` + `assignee` clauses. Only the inbound queue is narrowed.
  (`src/board/linear.rs:329-331`.)
- **Do NOT gate direct-ID operations.** `get_ticket`, `comment`, `assign`, and
  `return_ticket` keep operating on any ticket by id regardless of project
  scope. Scope means "what shows up in my queue," not "what I may touch."
- **Attach created sub-issues to the configured project.** `create_sub` →
  `issueCreate` passes `projectId: "<project_id>"` when set, so a sub-task the
  agent creates stays visible to its own scoped queue (Linear sub-issues do not
  inherit project membership automatically). (`src/board/linear.rs:449-457`.)
- **Project picker in `orga init board` (linear).** After team selection, the
  wizard fetches the team's projects and offers "None / <list>", defaulting the
  cursor to the existing `project_id` on re-init. The config stores the stable
  project UUID, never the (renameable) name. (`src/init.rs:302-397`,
  `fetch_linear_projects`.)

No `Board` trait method changes signature. No new dependency. Trello is
unaffected.

## Capabilities

### New Capabilities
None at the capability level — this extends an existing capability.

### Modified Capabilities

- `linear-backend`: add an optional project scope. The team remains the board
  primitive (source of workflow states and columns); a configured project
  additionally narrows the agent's inbound ticket queue and attaches
  agent-created sub-issues to that project. > Added `Project scoping`
  requirement.

## Impact

- **Code**: `src/config.rs` (`LinearConfig` + `validate`), `src/board/mod.rs`
  (`build_board` threads `project_id`), `src/board/linear.rs` (filter clause +
  `create_sub`), `src/init.rs` (project fetch + picker step).
- **Config schema**: `[linear]` gains an optional `project_id`. Existing
  configs load unchanged; no migration shim.
- **Dependencies**: none added. `reqwest`/`inquire` already in use.
- **Specs**: `linear-backend` gains one `## ADDED Requirements` block
  ("Project scoping"). The existing "Team as board primitive" requirement is
  intentionally left untouched — a project does not displace the team as the
  workflow/columns source.
- **Build / test**: `cargo build`, `cargo test --lib`, `cargo test --test *`
  must pass. New unit tests cover (a) the `list_assigned` filter clause
  assembly with and without `project_id`, and (b) `create_sub` passing
  `projectId` only when set.
- **Known drift, out of scope**: the live `linear-backend` spec still describes
  `add_checklist_item` / `check_item` / `checklists`, which the current code
  does not implement (it uses `create_sub` + `sub_tickets`). This proposal
  targets the actual code, not the stale spec wording; reconciling that drift
  is a separate change.
