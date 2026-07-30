## Context

orga's Linear backend scopes every board operation to a single **Team**. The
`[linear]` section holds `api_key` and `team_id`; `list_assigned` queries:

```graphql
issues(filter: { team: { id: { eq: "<team_id>" } }
                 assignee: { id: { eq: "<viewer_id>" } } })
```

`list_columns` returns the team's workflow states (`team_states()`), and
`create_sub` creates issues in that team. There is no concept of a Linear
**Project** anywhere in the config, backend, or init wizard.

A Linear Project is a grouping object distinct from a Team:

```
Team          owns a WORKFLOW (states, members, cycles) — required per issue
Project       groups issues (optionally across teams) — optional per issue
              has its own status (planned/started/paused/completed/canceled),
              NOT per-issue workflow states
```

The key constraint: **a project has no per-issue workflow states.** Issues
inside a project still carry their *team's* states. Therefore project scoping
cannot become the source of `list_columns` — it can only narrow which issues
the agent is handed.

Constraints carried from the existing `linear-backend` spec and `AGENTS.md`:
- The `Board` trait signature must not change (both backends implement it).
- `team_id` remains required and remains the workflow/columns source.
- Direct-ID operations (`get_ticket`, `comment`, `assign`) must keep working on
  any ticket id, because `orga ticket show <id>` is a valid CLI path and the
  agent acts on explicit ids it pulled from its queue.
- No `unwrap`/`expect` in production paths; `Result<_, OrgaError>` throughout.

## Goals / Non-Goals

**Goals:**
- Let an operator narrow a Linear agent's inbound queue to a single project,
  without changing anything for configs that do not opt in.
- Keep sub-issues the agent creates visible to its own scoped queue.
- Single source of truth for the project filter (the optional `project_id`
  field), mirroring how `team_id` is already the single team source.
- `orga init board` (linear) can pick the project interactively.

**Non-Goals:**
- No project-as-board mode (treating a project's *status* as columns). Rejected
  by the model mismatch above.
- No cycle scoping. Named here so the config shape leaves room; deliberately
  deferred (YAGNI).
- No cross-team project handling beyond the natural consequence of intersecting
  team ∩ project (a cross-team project scoped under team T shows only the
  T-team slice of that project — documented, not worked around).
- No reconciliation of the stale `checklists`/`check_item` spec wording; that
  drift predates this change.
- No change to Trello.

## Decisions

**D1. Project scope is always intersected with team, never a replacement.**

Dropping team scoping would break `list_columns` (a project exposes no
per-issue states) and silently change which workflow states exist. The filter
is `team ∩ assignee=me ∩ project`. Team stays the board primitive; project is
an additional narrowing of the inbox. This also means a cross-team project
scoped under team T surfaces only the T-team issues in that project — the
honest, unsurprising answer, not a hidden footgun.

**D2. `project_id` is optional; unset = exact current behavior.**

Zero opt-in cost, zero migration. Validation only checks that `team_id` remains
present (as today); `project_id` has no required companion.

**D3. Only `list_assigned` is filtered; direct-ID ops are ungated.**

`get_ticket`, `comment`, `assign`, `return_ticket` take an explicit id and must
keep resolving any id. Scope is "what appears in my queue," not "what I may
touch." Gating these would break `ticket show <id>` for cross-project work and
surprise anyone debugging a ticket. The agent only acts on ids it pulled from
the scoped queue, so the inbox filter is the correct and sufficient boundary.

**D4. `create_sub` attaches the new sub-issue to the configured project when
set.** *(The one decision worth a second look.)*

Linear sub-issues do **not** inherit project membership from their parent. If a
scoped agent creates a sub-task and it lands outside the configured project,
the scoped `list_assigned` will never surface that sub-task again — the agent
silently loses track of its own work, in a way that only shows up when a
sub-task "vanishes" from a future pass. Attaching via `issueCreate`'s
`projectId` input prevents that and is safe: a sub-issue belonging to the
project it was created under is never wrong, and if the operator later removes
project scoping the sub-issue simply retains a harmless membership.

Rule: when `project_id` is `Some`, every `create_sub` passes
`projectId: "<project_id>"`. When `None`, behavior is unchanged. We do not try
to read the parent's project at create time — the configured project is the
agent's working context and is the predictable choice.

**D5. Identify the project by its stable UUID, not its name.**

Linear project names are renameable; ids are not. The config stores the id.
`orga init` shows names for selection and writes the corresponding id, exactly
as the team picker already does (`team_names` for display, `team_id` stored).

**D6. The team requirement in the spec is left untouched; a new "Project
scoping" requirement is added.**

The existing "Team as board primitive" requirement still holds verbatim — team
remains the source of workflow states and columns. A project scope is
additive, so it gets its own `## ADDED Requirements` block rather than a
rewrite of the team requirement. Rewriting the team requirement would risk
contradicting its existing scenarios (e.g. "list_columns returns team workflow
states") for no benefit.

## Alternatives considered

- *Project as the board (status → columns).* Rejected: projects have no
  per-issue workflow states; the `Column`/`return` model is shaped for team
  states.
- *Read the parent's project at `create_sub` time and inherit it.* Rejected as
  the default: adds a read, and the configured project is the agent's declared
  working context. Could revisit if agents ever create subs across projects,
  but that contradicts the scoped-inbox model.
- *Gate `get_ticket`/`comment`/`assign` by project for safety.* Rejected: breaks
  `ticket show <id>` and the agent's own ability to operate on any id it was
  handed; the inbox filter is the right and sufficient trust boundary.
- *Cycle scoping in the same change.* Deferred (YAGNI); the optional-field
  pattern generalizes cleanly if cycles are wanted later.
