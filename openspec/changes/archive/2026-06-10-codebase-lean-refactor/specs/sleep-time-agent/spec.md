## MODIFIED Requirements

### Requirement: Sleep-time agent triggered after done()
The agent loop SHALL invoke the sleep-time reflection agent after `done()` resolves successfully, using `run_llm_loop`. The sleep-time agent SHALL receive the completed ticket's full context (title, description, comments) and the current memory file tree index, and SHALL be prompted to persist cross-ticket learnings into topic files in the context repository. The sleep-time and defrag passes SHALL each open a fresh `ContextRepository` exactly once at the start of that pass.

#### Scenario: done() triggers reflection
- **WHEN** the main agent calls `done()` and the board operation succeeds
- **THEN** a sleep-time agent is invoked before the ticket cycle exits

#### Scenario: done() failure does not trigger reflection
- **WHEN** the main agent calls `done()` and the board operation fails
- **THEN** no sleep-time agent is invoked

#### Scenario: skip() does not trigger reflection
- **WHEN** the main agent calls `skip()`
- **THEN** no sleep-time agent is invoked

#### Scenario: ContextRepository opened once per reflection pass
- **WHEN** the sleep-time agent runs
- **THEN** `ContextRepository::open` is called once at the start of the pass, not once per iteration
