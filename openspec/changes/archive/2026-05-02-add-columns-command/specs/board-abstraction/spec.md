## ADDED Requirements

### Requirement: Column data model
The `Board` trait SHALL operate on a shared `Column` type that is backend-agnostic. The `Column` type SHALL include: `id` (String) and `name` (String). It SHALL derive `Debug`, `Clone`, `Serialize`, and `Deserialize`.

#### Scenario: Column serialization
- **WHEN** a column is returned from any backend
- **THEN** it can be serialized to JSON with exactly the fields `id` and `name`, without backend-specific fields leaking

### Requirement: list_columns trait method
The `Board` trait SHALL define a `list_columns() -> Result<Vec<Column>, OrgaError>` method. All backend implementations SHALL implement this method.

#### Scenario: Columns returned
- **WHEN** `list_columns()` is called on a valid board
- **THEN** it returns a `Vec<Column>` with one entry per column on the board

#### Scenario: Backend failure
- **WHEN** the underlying API call fails
- **THEN** `list_columns()` returns an `Err(OrgaError)` with an appropriate variant
