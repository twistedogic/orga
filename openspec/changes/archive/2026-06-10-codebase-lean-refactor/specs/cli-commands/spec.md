## ADDED Requirements

### Requirement: CLI display and error output module
Human-readable display functions (`print_column_list`, `print_ticket_summary_list`, `print_ticket_detail`) and the `exit_error` helper SHALL live in `src/output.rs` and be imported into `main.rs`. No display logic SHALL remain defined directly in `main.rs`.

#### Scenario: Output functions in dedicated module
- **WHEN** `main.rs` prints ticket details or a column list
- **THEN** it calls functions from `crate::output`, not locally defined functions

#### Scenario: exit_error in output module
- **WHEN** the CLI encounters a fatal error and must exit
- **THEN** it calls `output::exit_error`, which logs to the logger and prints to stderr
