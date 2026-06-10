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
The CLI SHALL provide `orga ticket show <id>` to output the full context of a ticket: title, description, current list, creator, assignees, sub-tickets, and all comments in chronological order. If the ticket's current column matches a `[[workflow]]` entry in config, the resolved prompt text SHALL be included in the output.

#### Scenario: Ticket exists
- **WHEN** a valid ticket ID is provided
- **THEN** full ticket context is printed including creator (if known), all comments and sub-tickets

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a JSON object with fields: `id`, `title`, `description`, `list`, `creator`, `assignees`, `sub_tickets`, `comments`

#### Scenario: JSON output with workflow prompt
- **WHEN** `--json` flag is passed and the ticket's column has a matching workflow entry
- **THEN** the JSON object additionally includes `workflow_prompt` containing the resolved prompt text

#### Scenario: JSON output without workflow prompt
- **WHEN** `--json` flag is passed and the ticket's column has no matching workflow entry
- **THEN** the JSON object does NOT include a `workflow_prompt` field

#### Scenario: Human-readable output with workflow prompt
- **WHEN** no `--json` flag and the ticket's column has a matching workflow entry
- **THEN** the output includes a `## Workflow` section at the end containing the prompt text

#### Scenario: Human-readable output without workflow prompt
- **WHEN** no `--json` flag and the ticket's column has no matching workflow entry
- **THEN** no `## Workflow` section appears in the output

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

### Requirement: Ticket create-sub command
The CLI SHALL provide `orga ticket create-sub <parent-id> <title> [--description <text>] [--list <column-name>]` to create a sub-ticket linked to a parent ticket. The sub-ticket SHALL be created unassigned. If `--list` is omitted, the sub-ticket SHALL be placed in the same list as the parent. If `--list` is provided, the CLI SHALL error if no list with that name exists.

#### Scenario: Sub-ticket created
- **WHEN** a valid parent ticket ID and title are provided
- **THEN** a new ticket is created on the same board, linked to the parent, unassigned, in the parent's list, and its ID and URL are printed

#### Scenario: Sub-ticket created with description
- **WHEN** `--description <text>` is provided
- **THEN** the sub-ticket is created with the given description

#### Scenario: Sub-ticket created with explicit list
- **WHEN** `--list <column-name>` is provided and that column exists
- **THEN** the sub-ticket is placed in that list instead of the parent's list

#### Scenario: List not found
- **WHEN** `--list <column-name>` is provided and no column with that name exists
- **THEN** the command exits with a non-zero code and prints an error naming the missing list

#### Scenario: JSON output
- **WHEN** `--json` flag is passed
- **THEN** output is a JSON object with the new ticket's `id`, `title`, and `url`

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

### Requirement: Ticket compact command
The CLI SHALL provide `orga ticket compact <id> --summary <text>` to store a compaction record for a ticket. The `--summary` argument SHALL be required and non-empty. On success the command SHALL print `{"ok": true}` with `--json` or a human-readable confirmation without it.

#### Scenario: Compact with summary
- **WHEN** `ticket compact <id> --summary "..."` is called with a non-empty summary
- **THEN** the compaction record is stored and the command exits with code 0

#### Scenario: Empty summary rejected
- **WHEN** `ticket compact <id> --summary ""` is called
- **THEN** the command exits with a non-zero code and prints an error

#### Scenario: JSON success output
- **WHEN** `ticket compact <id> --summary "..."` succeeds with `--json`
- **THEN** output is `{"ok": true}`

### Requirement: Ticket decompact command
The CLI SHALL provide `orga ticket decompact <id>` to delete the stored compaction record for a ticket. On success the command SHALL print `{"ok": true}` with `--json` or a human-readable confirmation without it.

#### Scenario: Decompact existing record
- **WHEN** `ticket decompact <id>` is called and a compaction record exists
- **THEN** the record is deleted and the command exits with code 0

#### Scenario: Decompact with no record is a no-op
- **WHEN** `ticket decompact <id>` is called and no compaction record exists
- **THEN** the command exits with code 0 without error

#### Scenario: JSON success output
- **WHEN** `ticket decompact <id>` succeeds with `--json`
- **THEN** output is `{"ok": true}`


### Requirement: Systemd subcommand
The CLI SHALL expose a top-level `systemd` subcommand with an `install` sub-subcommand. The `systemd install` command SHALL accept an optional `--system` flag. The global `--config` flag SHALL be respected to resolve the config path written into the generated unit file.

#### Scenario: Systemd install appears in help
- **WHEN** `orga --help` is run
- **THEN** `systemd` appears as a top-level subcommand in the output

#### Scenario: Install subcommand appears in help
- **WHEN** `orga systemd --help` is run
- **THEN** `install` appears as a subcommand with a description

#### Scenario: --system flag accepted
- **WHEN** `orga systemd install --system` is invoked
- **THEN** the command proceeds with system-level placement logic

### Requirement: memory list subcommand
The CLI SHALL provide `orga memory list` to output the context repository file tree. With `--json`, output SHALL be a JSON array of `{"path": "...", "description": "..."}` objects.

#### Scenario: memory list in command reference
- **WHEN** `orga memory list --json` is called
- **THEN** a JSON array of topic file entries is returned

### Requirement: memory read subcommand
The CLI SHALL provide `orga memory read <path>` to output the full content of a topic file. With `--json`, output SHALL be `{"path": "...", "content": "..."}`.

#### Scenario: memory read in command reference
- **WHEN** `orga memory read <path>` is called with a valid path
- **THEN** the file content is printed to stdout

### Requirement: memory write subcommand
The CLI SHALL provide `orga memory write <path> <content>` to write (create or overwrite) a topic file. An optional `--message` flag sets the commit message.

#### Scenario: memory write in command reference
- **WHEN** `orga memory write <path> "<content>"` is called
- **THEN** the file is written and committed

### Requirement: memory search subcommand
The CLI SHALL provide `orga memory search <query>` to search across all `.md` files in the context repository. With `--json`, output SHALL be a JSON array of `{"path": "...", "line": N, "content": "..."}` objects.

#### Scenario: memory search in command reference
- **WHEN** `orga memory search "<query>"` is called
- **THEN** matching lines are returned with file path and line number

### Requirement: memory defrag subcommand
The CLI SHALL provide `orga memory defrag` as a read-only analysis report of the context repository. It SHALL NOT invoke an LLM or mutate any files. With `--json`, output SHALL be a structured JSON object.

The report SHALL include three sections:
1. **Oversized files** — `.md` files over 200 lines, listed with path and line count
2. **Duplicate candidates** — pairs of files sharing ≥ 2 significant description terms (terms ≥ 3 chars, excluding stopwords), listed with the shared terms
3. **Deletion candidates** — files where all description terms appear in at least one other file (i.e., would pass `memory_delete` guardrail), listed with which file covers them

#### Scenario: Report shows oversized files
- **WHEN** `orga memory defrag` is called and a file exceeds 200 lines
- **THEN** that file appears in the oversized section with its line count

#### Scenario: Report shows duplicate candidates
- **WHEN** two files share ≥ 2 significant description terms
- **THEN** they appear as a pair in the duplicate candidates section with the shared terms listed

#### Scenario: Report shows deletion candidates
- **WHEN** a file's description terms all appear in at least one other file
- **THEN** it appears in the deletion candidates section with the covering file named

#### Scenario: Clean repository produces empty report
- **WHEN** `orga memory defrag` is called and the repository has no oversized, duplicate, or deletable files
- **THEN** the command exits with code 0 and prints nothing (or empty arrays with `--json`)

#### Scenario: JSON output
- **WHEN** `orga memory defrag --json` is called
- **THEN** output is a valid JSON object with keys `oversized`, `duplicates`, `deletion_candidates`

### Requirement: memory delete subcommand
The CLI SHALL provide `orga memory delete <path>` to delete a topic file from the context repository, applying the same uniqueness guardrail as `ContextRepository::delete()`. With `--json`, success output SHALL be `{"ok": true, "path": "..."}`.

#### Scenario: memory delete in command reference
- **WHEN** `orga memory delete <path>` is called on a file whose description terms are covered elsewhere
- **THEN** the file is deleted and committed

#### Scenario: memory delete blocked
- **WHEN** `orga memory delete <path>` is called on a file whose description terms appear nowhere else
- **THEN** the command exits with a non-zero code and prints the guardrail error

### Requirement: CLI display and error output module
Human-readable display functions (`print_column_list`, `print_ticket_summary_list`, `print_ticket_detail`) and the `exit_error` helper SHALL live in `src/output.rs` and be imported into `main.rs`. No display logic SHALL remain defined directly in `main.rs`.

#### Scenario: Output functions in dedicated module
- **WHEN** `main.rs` prints ticket details or a column list
- **THEN** it calls functions from `crate::output`, not locally defined functions

#### Scenario: exit_error in output module
- **WHEN** the CLI encounters a fatal error and must exit
- **THEN** it calls `output::exit_error`, which logs to the logger and prints to stderr

