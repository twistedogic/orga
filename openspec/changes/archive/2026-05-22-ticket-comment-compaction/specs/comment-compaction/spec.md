## ADDED Requirements

### Requirement: Compaction record storage
The system SHALL maintain a `comment_compaction` table in the local SQLite store (`memory.db`). Each record SHALL store the ticket ID, agent-written summary text, the `compacted_through` timestamp (ISO8601), the count of comments compacted, and an `updated_at` timestamp. There SHALL be at most one compaction record per ticket ID. Writing a new record for an existing ticket ID SHALL overwrite the previous record.

#### Scenario: Store compaction record
- **WHEN** `ticket compact <id> --summary "..."` is called
- **THEN** a record is upserted in `comment_compaction` with the summary, `compacted_through` set to the timestamp of the most recent comment on the ticket, and `compacted_count` set to the number of comments fetched

#### Scenario: Overwrite existing compaction record
- **WHEN** `ticket compact <id>` is called on a ticket that already has a compaction record
- **THEN** the existing record is replaced with the new summary and updated boundary values

#### Scenario: Delete compaction record
- **WHEN** `ticket decompact <id>` is called
- **THEN** the compaction record for that ticket is deleted; subsequent `ticket show` returns all comments

#### Scenario: Delete non-existent record
- **WHEN** `ticket decompact <id>` is called for a ticket with no compaction record
- **THEN** the command exits with code 0 and returns `{"ok": true}` (no error)

### Requirement: Compaction applied on ticket show
When a compaction record exists for a ticket, `ticket show` SHALL apply it: the `comments` array SHALL contain only comments with `at > compacted_through`, and the response SHALL include a `comment_compaction` object with the stored summary, boundary, and count.

#### Scenario: Compaction applied to JSON output
- **WHEN** `ticket show --json` is called and a compaction record exists
- **THEN** the JSON output includes `comment_compaction: { summary, compacted_through, compacted_count }` and `comments` contains only comments after `compacted_through`

#### Scenario: No compaction record — full comments returned
- **WHEN** `ticket show` is called and no compaction record exists
- **THEN** all comments are returned and no `comment_compaction` field is present

### Requirement: Compaction suggested hint
When no compaction record exists and the total number of comments on a ticket exceeds `comment_compaction_threshold`, `ticket show` SHALL include `compaction_suggested: true` in its output to signal to the agent that compaction is recommended.

#### Scenario: Hint present when over threshold
- **WHEN** `ticket show --json` is called, no compaction record exists, and comment count exceeds the configured threshold
- **THEN** the JSON output includes `"compaction_suggested": true`

#### Scenario: Hint absent when under threshold
- **WHEN** `ticket show --json` is called, no compaction record exists, and comment count is at or below threshold
- **THEN** the JSON output does NOT include `compaction_suggested`

#### Scenario: Hint absent when compaction record exists
- **WHEN** `ticket show --json` is called and a compaction record exists
- **THEN** `compaction_suggested` is NOT present regardless of remaining comment count

### Requirement: Configurable compaction threshold
The config file SHALL support a `comment_compaction_threshold` key. When absent, the default value SHALL be 5.

#### Scenario: Custom threshold respected
- **WHEN** `comment_compaction_threshold: 10` is set in config and a ticket has 11 uncompacted comments
- **THEN** `ticket show --json` includes `"compaction_suggested": true`

#### Scenario: Default threshold used when not configured
- **WHEN** `comment_compaction_threshold` is absent from config and a ticket has 6 uncompacted comments
- **THEN** `ticket show --json` includes `"compaction_suggested": true`
