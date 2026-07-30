## Tasks

### Config

- [ ] Add `project_id: Option<String>` to `LinearConfig` in `src/config.rs`
      (next to `api_key`, `team_id`).
- [ ] Add a unit test in `src/config.rs` that a `[linear]` section with
      `project_id` round-trips through load/save, and that omitting it still
      loads (no required-companion error).
- [ ] Confirm `AppConfig::validate` requires `team_id` but imposes **no** new
      requirement on `project_id`; update the table-driven validation rows if a
      row references the linear fields.

### Backend — inbound filter

- [ ] Store `project_id: Option<String>` on `LinearBackend` and thread it
      through `LinearBackend::new` from `build_board` (`src/board/mod.rs`).
- [ ] In `list_assigned`, inject `project: { id: { eq: "<id>" } }` into the
      `issues(filter: {…})` query only when `project_id` is `Some`.
- [ ] Add a test (extract the query-building into a small pure helper or assert
      on the assembled filter string) covering: no `project_id` → no project
      clause; `Some(id)` → project clause present with the correct id.

### Backend — sub-issue creation

- [ ] In `create_sub_issue`, pass `projectId: "<project_id>"` in the
      `issueCreate` input when `project_id` is `Some`; omit it when `None`.
- [ ] Add a test asserting the `issueCreate` variables/input include `projectId`
      when set and omit it when unset.

### Init wizard

- [ ] Add `fetch_linear_projects(api_key, team_id) -> Vec<LinearProjectItem>`
      returning `{ id, name }` for the selected team
      (`projects(filter: { team: { id: { eq } } })`).
- [ ] After team selection in `run_linear_init`, prompt with a `Select` of
      "None" + project names; default cursor to the existing `project_id` on
      re-init, else "None".
- [ ] Write `project_id` (the chosen id, or omit the field for "None") into the
      `[linear]` section in the saved config.

### Spec

- [ ] Add the "Project scoping" requirement under
      `openspec/specs/linear-backend/spec.md` (via this change's delta at
      `specs/linear-backend/spec.md`) with scenarios for: filter applied to
      `list_assigned`; direct-ID ops ungated; sub-issue attached to project;
      `project_id` optional; init writes the id.
- [ ] Leave the "Team as board primitive" requirement unchanged; verify its
      scenarios still hold.

### Validation & docs

- [ ] `cargo build` clean.
- [ ] `cargo test --lib` green (new + existing linear/config tests).
- [ ] `cargo test` (all targets) green.
- [ ] Update `AGENTS.md` only if the `[linear]` section description needs the
      new optional field noted (it lists config sections generically; likely no
      edit needed — confirm).
- [ ] `openspec validate linear-project-scoping` passes.
