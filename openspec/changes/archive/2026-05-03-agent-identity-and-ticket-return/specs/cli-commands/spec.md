## ADDED Requirements

### Requirement: whoami command in CLI
The CLI SHALL provide `orga whoami` as a top-level command alongside `init`, `ticket`, `checklist`, `memory`, and `columns`. It SHALL appear in `orga --help` output with a brief description.

#### Scenario: whoami appears in help
- **WHEN** `orga --help` is run
- **THEN** `whoami` is listed as an available command with a brief description

### Requirement: ticket return subcommand in CLI
The CLI SHALL provide `orga ticket return <id>` as a subcommand under `ticket`. It SHALL accept an optional `--comment <text>` flag and appear in `orga ticket --help` output.

#### Scenario: ticket return appears in ticket help
- **WHEN** `orga ticket --help` is run
- **THEN** `return` is listed as an available subcommand with a brief description

#### Scenario: --comment flag accepted
- **WHEN** `orga ticket return <id> --comment "some text"` is run
- **THEN** the command accepts the flag without error

## MODIFIED Requirements

### Requirement: Ticket show command
The CLI SHALL provide `orga ticket show <id>` to output the full context of a ticket: title, description, current list, creator, assignees, checklist items, and all comments in chronological order.

#### Scenario: Ticket exists
- **WHEN** a valid ticket ID is provided
- **THEN** full ticket context is printed including creator (if known), all comments and checklist items

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a JSON object with fields: `id`, `title`, `description`, `list`, `creator`, `assignees`, `checklists`, `comments`

#### Scenario: Ticket not found
- **WHEN** an invalid or nonexistent ticket ID is provided
- **THEN** the command exits with a non-zero code and prints an error message to stderr

### Requirement: Ticket comment command
The CLI SHALL provide `orga ticket comment <id> <text>` to post a comment on a ticket as the configured agent. When `agent.name` is set in config, the comment SHALL be tagged with `\n\n_[orga:<agent-name>]_` before posting.

#### Scenario: Comment posted successfully
- **WHEN** a valid ticket ID and non-empty comment text are provided
- **THEN** the comment is posted (with agent tag if applicable) and a success message is printed

#### Scenario: Empty comment rejected
- **WHEN** the comment text is empty
- **THEN** the command exits with a non-zero code and prints an error to stderr
