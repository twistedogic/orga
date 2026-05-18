## ADDED Requirements

### Requirement: HTTP error body capture and logging
When the Trello backend receives a 4xx or 5xx response, it SHALL read the full response body as text and log it at `error` level via the injected `Logger`. The logged entry SHALL include the HTTP status code and the raw body. The `OrgaError` returned to the caller SHALL contain only the status code (not the body). All existing status-specific error mappings (429 → `RateLimited`, 401 → `Unauthorized`, 404 → `NotFound`) SHALL be preserved.

#### Scenario: 449 body logged
- **WHEN** Trello returns HTTP 449 with a body such as `{"message":"missing required field"}`
- **THEN** the log file contains an ERROR entry with "HTTP 449" and the body text
- **THEN** the error returned to the caller says "Trello returned HTTP 449"

#### Scenario: 500 body logged
- **WHEN** Trello returns HTTP 500 with any body
- **THEN** the log file contains an ERROR entry with "HTTP 500" and the body text

#### Scenario: 429 still maps to RateLimited
- **WHEN** Trello returns HTTP 429
- **THEN** `OrgaError::RateLimited` is returned and the body (if any) is logged

#### Scenario: 2xx response unaffected
- **WHEN** Trello returns a 2xx response
- **THEN** the body is parsed as JSON normally; nothing is logged

### Requirement: Logger injected into TrelloBackend
`TrelloBackend` SHALL accept an `Arc<Logger>` at construction time and use it for all HTTP error logging.

#### Scenario: Logger used for errors
- **WHEN** `TrelloBackend::new` is called with a logger
- **THEN** all subsequent HTTP errors from that backend are logged via the provided logger
