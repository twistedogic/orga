## ADDED Requirements

### Requirement: Agent comments are tagged on write
When `orga ticket comment` or `orga ticket return --comment` posts a comment and `agent.name` is set in config, the CLI SHALL append `\n\n_[orga:<agent-name>]_` to the comment text before sending it to the backend.

#### Scenario: Comment tagged when agent.name is set
- **WHEN** `orga ticket comment <id> "hello"` is run and config has `agent.name = "agent-1"`
- **THEN** the text posted to the board is `"hello\n\n_[orga:agent-1]_"`

#### Scenario: Comment not tagged when agent.name is absent
- **WHEN** `orga ticket comment <id> "hello"` is run and config has no `agent.name`
- **THEN** the text posted to the board is `"hello"` unmodified

### Requirement: Agent comments are parsed on read
When `ticket show` reads comments, the CLI SHALL detect the `_[orga:<name>]_` tag at the end of comment content. If detected, it SHALL strip the tag from `content` and set `agent_name` to the parsed name. If not detected, `agent_name` SHALL be `None`.

#### Scenario: Tagged comment parsed
- **WHEN** a comment ends with `\n\n_[orga:agent-1]_`
- **THEN** `Comment.agent_name` is `"agent-1"` and `Comment.content` does not include the tag

#### Scenario: Untagged comment unchanged
- **WHEN** a comment has no `_[orga:...]_` tag
- **THEN** `Comment.agent_name` is `null` and `Comment.content` is the full original text

#### Scenario: JSON output includes agent_name
- **WHEN** `ticket show <id> --json` is run and comments include tagged and untagged entries
- **THEN** each comment object includes `agent_name` (string or null)
