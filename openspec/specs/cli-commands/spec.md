# cli-commands Specification

## Purpose
TBD - created by archiving change agent-board-cli. Update Purpose after archive.
## Requirements
### Requirement: Ticket list command
The CLI SHALL provide `orga ticket list` to output all tickets currently assigned to the configured agent that are not completed AND where the latest comment was not posted by an agent. With `--completed`, output SHALL include only completed tickets. With `--all`, output SHALL include all tickets regardless of completion state or latest commenter. Output SHALL include ticket ID, title, list name, and URL. With `--json`, output SHALL be a JSON array of ticket objects.

#### Scenario: Tickets assigned to agent (default)
- **WHEN** the agent has one or more open assigned tickets where the latest comment is not from an agent
- **THEN** each such ticket is printed with its ID, title, current list, and URL

#### Scenario: Agent-last tickets hidden by default
- **WHEN** an assigned open ticket's latest comment was posted by an agent
- **THEN** that ticket SHALL NOT appear in the default `ticket list` output

#### Scenario: No open tickets needing response
- **WHEN** all assigned open tickets have their latest comment from an agent (or there are no assigned open tickets)
- **THEN** the command exits with code 0 and prints nothing (or empty JSON array with `--json`)

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a valid JSON array of ticket objects with fields: `id`, `title`, `list`, `url`, `description`, `completed`, `last_commenter_is_agent`

#### Scenario: Completed tickets only
- **WHEN** `--completed` flag is passed
- **THEN** only tickets with `completed: true` are printed (agent-last filter does not apply)

#### Scenario: All tickets
- **WHEN** `--all` flag is passed
- **THEN** all tickets assigned to the agent are printed regardless of completion state or latest commenter

#### Scenario: `--completed` and `--all` are mutually exclusive
- **WHEN** both `--completed` and `--all` are passed
- **THEN** the command exits with a non-zero code and prints a usage error

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

### Requirement: Ticket assign command
The CLI SHALL provide `orga ticket assign <id> <username>` to assign a ticket to a teammate by their board username or handle.

#### Scenario: Assignment succeeds
- **WHEN** a valid ticket ID and valid username are provided
- **THEN** the ticket is assigned to that user and a success message is printed

#### Scenario: Unknown username
- **WHEN** the username does not exist on the board
- **THEN** the command exits with a non-zero code and prints an error to stderr

### Requirement: Ticket move command
The CLI SHALL provide `orga ticket move <id> <list>` to move a ticket to a different list (column) by list name.

#### Scenario: Move succeeds
- **WHEN** a valid ticket ID and valid list name are provided
- **THEN** the ticket is moved to that list and a success message is printed

#### Scenario: Agent cannot close tickets
- **WHEN** the target list is a "Done" or "Closed" equivalent list
- **THEN** the command SHALL still execute the move — closing restriction is a policy enforced by the agent, not the CLI

#### Scenario: Unknown list
- **WHEN** the list name does not exist on the board
- **THEN** the command exits with a non-zero code and prints an error to stderr

### Requirement: Ticket create-sub command
The CLI SHALL provide `orga ticket create-sub <parent-id> <title>` to create a sub-ticket linked to a parent ticket.

#### Scenario: Sub-ticket created
- **WHEN** a valid parent ticket ID and title are provided
- **THEN** a new ticket is created on the same board, linked to the parent, and its ID and URL are printed

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a JSON object with the new ticket's `id`, `title`, and `url`

### Requirement: Checklist add command
The CLI SHALL provide `orga checklist add <ticket-id> <item-text>` to add a checklist item to a ticket.

#### Scenario: Item added
- **WHEN** a valid ticket ID and item text are provided
- **THEN** the checklist item is added and a success message with the item ID is printed

### Requirement: Checklist check command
The CLI SHALL provide `orga checklist check <ticket-id> <item-id>` to mark a checklist item as complete.

#### Scenario: Item checked
- **WHEN** a valid ticket ID and item ID are provided
- **THEN** the checklist item is marked complete and a success message is printed

#### Scenario: Item not found
- **WHEN** the item ID does not exist on the ticket
- **THEN** the command exits with a non-zero code and prints an error to stderr

### Requirement: Global --json flag
All read commands SHALL support a `--json` flag that switches output to machine-readable JSON. Write commands SHALL output a plain success line or a JSON `{"ok": true}` object when `--json` is passed.

#### Scenario: Read command with --json
- **WHEN** any read command is invoked with `--json`
- **THEN** output is valid JSON to stdout

#### Scenario: Error with --json
- **WHEN** any command fails and `--json` is passed
- **THEN** error output is `{"error": "<message>"}` to stderr with a non-zero exit code


### Requirement: Init command
The CLI SHALL provide `orga init` as a top-level command (not under any subcommand group) that launches the interactive setup wizard. It SHALL be listed in `orga --help` output.

#### Scenario: Init appears in help
- **WHEN** `orga --help` is run
- **THEN** `init` is listed as an available command with a brief description

#### Scenario: Init does not require existing config
- **WHEN** `orga init` is run before any config file exists
- **THEN** the command starts successfully without a config-not-found error

### Requirement: Columns command in CLI
The CLI SHALL provide `orga columns` as a top-level command (alongside `init`, `ticket`, `checklist`, `memory`). It SHALL appear in `orga --help` output with a brief description.

#### Scenario: Columns appears in help
- **WHEN** `orga --help` is run
- **THEN** `columns` is listed as an available command with a brief description

#### Scenario: Columns does not require a subcommand
- **WHEN** `orga columns` is run with no additional arguments
- **THEN** the command executes and outputs the list of columns

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

