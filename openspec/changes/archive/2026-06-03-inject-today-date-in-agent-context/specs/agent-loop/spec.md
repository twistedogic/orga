## MODIFIED Requirements

### Requirement: User message contains ticket metadata
The user message SHALL include the following ticket metadata fields, in order: title, ID, column, URL, today's date, creator (if present), and assignees (if present).

#### Scenario: User message includes today's date field
- **WHEN** any agent (main or subagent) constructs a user message
- **THEN** the message SHALL include `**Today's date:** YYYY-MM-DD` after the URL field and before the creator field

#### Scenario: User message includes all other metadata
- **WHEN** any agent constructs a user message
- **THEN** the message SHALL still include title, ID, column, URL, creator, and assignees as before
