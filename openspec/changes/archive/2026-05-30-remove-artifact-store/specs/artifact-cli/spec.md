# artifact-cli Specification (delta)

## REMOVED Requirements

### Requirement: artifact commit subcommand
**Reason**: Artifact store has been removed entirely.
**Migration**: Use `orga workspace` commands or `write_file` agent tool for per-ticket file storage.

### Requirement: artifact list subcommand
**Reason**: Artifact store has been removed entirely.
**Migration**: Use `list_files` agent tool or inspect the workspace directory directly.

### Requirement: artifact get subcommand
**Reason**: Artifact store has been removed entirely.
**Migration**: Use `read_file` agent tool or read files from the workspace directory directly.
