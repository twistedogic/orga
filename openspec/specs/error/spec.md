# error Specification

## Purpose
Defines the `OrgaError` enum and its variants, which represent all error conditions orga can return to its callers.

## Requirements

### Requirement: LLM error variant
The `OrgaError` enum SHALL include a `LlmError { kind: LlmErrorKind, message: String }` variant for failures from LLM completion calls. The `LlmErrorKind` enum SHALL be `#[non_exhaustive]` with at minimum the variants `Network`, `RateLimited`, `Auth`, `Parse`, `Backend`, and `Other`. The variant SHALL let callers (notably the metrics layer) classify failures by `kind` without string-sniffing the formatted error.

#### Scenario: LlmError Display contains kind and message
- **WHEN** an `OrgaError::LlmError { kind: LlmErrorKind::RateLimited, message: "rate limited" }` is formatted
- **THEN** the output contains both the kind label and the message

#### Scenario: LlmErrorKind is non-exhaustive
- **WHEN** downstream code matches on `LlmErrorKind` without a wildcard arm
- **THEN** the compiler warns that the match is non-exhaustive, so future variants do not break consumers silently
