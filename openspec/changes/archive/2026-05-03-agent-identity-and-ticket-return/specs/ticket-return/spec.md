## ADDED Requirements

### Requirement: ticket return command
The CLI SHALL provide `orga ticket return <id>` as a subcommand under `ticket`. It SHALL fetch the ticket, optionally post a comment (with agent tag if `agent.name` is set), then reassign the ticket to its creator. If the ticket has no known creator, the command SHALL exit with a non-zero code and print an error.

#### Scenario: Return without comment
- **WHEN** `orga ticket return <id>` is run and the ticket has a creator
- **THEN** the ticket is reassigned to its creator and a success message is printed

#### Scenario: Return with comment
- **WHEN** `orga ticket return <id> --comment "need more context"` is run
- **THEN** the comment is posted (with agent tag if applicable) and then the ticket is reassigned to its creator

#### Scenario: No creator
- **WHEN** `orga ticket return <id>` is run and the ticket has no known creator
- **THEN** the command exits with a non-zero code and prints an error to stderr

#### Scenario: JSON output
- **WHEN** `orga ticket return <id> --json` is run successfully
- **THEN** output is `{"ok": true}`

#### Scenario: Comment posted before reassignment
- **WHEN** `orga ticket return <id> --comment <text>` is run
- **THEN** the comment is posted before the reassignment occurs
