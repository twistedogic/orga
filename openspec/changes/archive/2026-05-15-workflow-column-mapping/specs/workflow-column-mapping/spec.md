## ADDED Requirements

### Requirement: Workflow column mapping config
The config SHALL support an optional `[[workflow]]` array of tables. Each entry SHALL specify a `column` name and exactly one of `prompt` (inline text) or `prompt_file` (path to a file). Having neither or both SHALL be a config error at load time. Tilde expansion SHALL apply to `prompt_file` paths.

#### Scenario: Entry with inline prompt
- **WHEN** a `[[workflow]]` entry has `column` and `prompt` fields
- **THEN** the config loads successfully and the prompt text is available for that column

#### Scenario: Entry with prompt_file
- **WHEN** a `[[workflow]]` entry has `column` and `prompt_file` pointing to an existing readable file
- **THEN** the config loads successfully and the file contents are available as the prompt for that column

#### Scenario: prompt_file with tilde path
- **WHEN** `prompt_file = "~/.orga/prompts/explore.md"` and the file exists
- **THEN** the tilde is expanded to the user's home directory and the file is read

#### Scenario: Missing prompt_file fails at load
- **WHEN** a `[[workflow]]` entry has `prompt_file` pointing to a non-existent path
- **THEN** the CLI exits with a non-zero code and prints a config error identifying the missing file

#### Scenario: Both prompt and prompt_file fails at load
- **WHEN** a `[[workflow]]` entry has both `prompt` and `prompt_file`
- **THEN** the CLI exits with a non-zero code and prints a config error

#### Scenario: Neither prompt nor prompt_file fails at load
- **WHEN** a `[[workflow]]` entry has `column` but neither `prompt` nor `prompt_file`
- **THEN** the CLI exits with a non-zero code and prints a config error

#### Scenario: No workflow section
- **WHEN** the config has no `[[workflow]]` entries
- **THEN** the config loads successfully; all commands work normally with no workflow injection

### Requirement: Column name matching
Workflow entries SHALL be matched against a ticket's current column name using case-insensitive comparison.

#### Scenario: Exact case match
- **WHEN** the ticket is in column "To Do" and a workflow entry has `column = "To Do"`
- **THEN** the matching entry's prompt is returned

#### Scenario: Case-insensitive match
- **WHEN** the ticket is in column "To Do" and a workflow entry has `column = "to do"`
- **THEN** the matching entry's prompt is returned

#### Scenario: No match
- **WHEN** no workflow entry matches the ticket's column name
- **THEN** no workflow prompt is returned; output is unchanged

### Requirement: Workflow prompt in ticket show output
When `orga ticket show` is run and the ticket's column has a matching workflow entry, the resolved prompt text SHALL be included in the output.

#### Scenario: JSON output with matching workflow
- **WHEN** `orga ticket show --json <id>` is run and the ticket's column has a workflow entry
- **THEN** the JSON output includes a `workflow_prompt` field containing the resolved prompt text

#### Scenario: JSON output without matching workflow
- **WHEN** `orga ticket show --json <id>` is run and the ticket's column has no workflow entry
- **THEN** the JSON output does NOT include a `workflow_prompt` field

#### Scenario: Human-readable output with matching workflow
- **WHEN** `orga ticket show <id>` is run (no `--json`) and the ticket's column has a workflow entry
- **THEN** the output includes a `## Workflow` section containing the resolved prompt text

#### Scenario: Human-readable output without matching workflow
- **WHEN** `orga ticket show <id>` is run and the ticket's column has no workflow entry
- **THEN** the output does NOT include a `## Workflow` section

#### Scenario: Workflow prompt not in ticket list
- **WHEN** `orga ticket list` is run regardless of workflow config
- **THEN** no workflow prompt appears in the output
