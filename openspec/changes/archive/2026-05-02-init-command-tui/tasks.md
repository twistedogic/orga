## 1. Dependencies & Module Setup

- [x] 1.1 Add `inquire` to `Cargo.toml` dependencies
- [x] 1.2 Create `src/init.rs` module file
- [x] 1.3 Declare `pub mod init` in `src/lib.rs`

## 2. Config Updates

- [x] 2.1 Add `AppConfig::try_load(path: &Path) -> Option<AppConfig>` that returns `None` on missing file or parse error
- [x] 2.2 Update config tests to cover `try_load` returning `None` for missing/invalid files

## 3. Init Module Implementation

- [x] 3.1 Define `TrelloMeResponse` and `TrelloBoardItem` structs (id, name) for deserialising API responses
- [x] 3.2 Implement `fetch_me(api_key, token) -> Result<TrelloMeResponse, OrgaError>` calling `GET /1/members/me`
- [x] 3.3 Implement `fetch_boards(api_key, token) -> Result<Vec<TrelloBoardItem>, OrgaError>` calling `GET /1/members/me/boards`
- [x] 3.4 Implement `run_init(config_path: &Path) -> Result<(), OrgaError>`:
  - Call `AppConfig::try_load()` to get optional existing values
  - Prompt agent name with `inquire::Text`, default from existing config
  - Prompt API key with `inquire::Text`, default from existing config
  - Prompt token with `inquire::Password`
  - Call `fetch_me` and print "Authenticated as @<username> (<full_name>)"; return error on 401
  - Call `fetch_boards`; return error if empty
  - Prompt board selection with `inquire::Select`, pre-select existing board if matched
  - Format TOML string and write to config path (create parent dirs with `fs::create_dir_all`)
  - Call `AppConfig::load()` on the written file to self-validate; return error if it fails
  - Print "Config written to <path>"

## 4. CLI Wiring

- [x] 4.1 Add `Commands::Init` variant to the `Commands` enum in `src/main.rs`
- [x] 4.2 Add match arm for `Commands::Init` that calls `run_init(&config_path)` before any `AppConfig::load()` is called
- [x] 4.3 Verify `orga --help` lists `init` with a description

## 5. Tests

- [x] 5.1 Add unit test for `run_init` with a temp file path: mock or skip network calls, assert the written TOML parses correctly via `AppConfig::load()`
- [x] 5.2 Add test that `run_init` on an existing config pre-populates defaults (verify the written file preserves unchanged values when defaults are accepted)
