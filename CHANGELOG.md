# Changelog

All notable changes to gcop-rs will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2025-12-21

### Fixed

- **Windows alias installation** (Issue #7): Fixed `gcop-rs alias` command failure on Windows by replacing Unix-specific `which` command with cross-platform `which` crate

### Changed

- **Cross-platform documentation**: Updated all docs to support Linux/macOS/Windows with platform-specific paths and commands
- **Commit command refactoring**: Refactored to state machine pattern for better testability (no user-visible changes)

### Added

- Comprehensive unit and integration tests (500+ lines covering config, commit, error, git, llm modules)
- `which` crate for cross-platform executable detection
- `mockall` crate for testing (optional dependency)

## [0.2.0] - 2025-12-20

### Added

- **Configurable network settings**: New `[network]` config section with `request_timeout`, `connect_timeout`, `max_retries`, `retry_delay_ms`
- **Configurable file limits**: New `[file]` config section with `max_size` for review file size limit
- **LLM parameter config**: `max_tokens` and `temperature` can now be set per-provider in config file
- **Commit retry limit config**: New `max_retries` option in `[commit]` section

### Changed

- **Constants elimination**: Removed `src/constants.rs`, moved constants to their usage sites
  - LLM defaults → `src/llm/provider/base.rs`
  - UI constants → `src/ui/prompt.rs`
  - Prompt templates → `src/llm/prompt.rs`
- **Config-driven architecture**: All previously hardcoded values now read from config with sensible defaults

### Breaking Changes

- `GitRepository::open()` now takes `Option<&FileConfig>` parameter (pass `None` for defaults)

## [0.1.6] - 2025-12-20

### Added

- **HTTP timeout configuration**: Request timeout 120s, connection timeout 10s to prevent infinite hanging
- **LLM API auto-retry**: Automatically retry on connection failures and 429 rate limits with exponential backoff (1s, 2s, 4s), up to 3 retries
- **SOCKS proxy support**: Support HTTP/HTTPS/SOCKS5 proxy via environment variables
- **Enhanced error messages**: Network errors now show detailed error types and resolution suggestions

### Changed

- **Constants refactor**: Extract all constants to `src/constants.rs`, add HTTP and retry related constant modules
- **File size validation**: Optimize large file skip logic

### Fixed

- Network requests no longer hang indefinitely (timeout limits added)
- Temporary network failures and API rate limits now automatically retry

## [0.1.5] - 2025-12-20

### Changed
- **Unified editor handling**: `config edit` now uses `edit` crate instead of raw `Command::new()`, matching the pattern used in commit message editing
- **Simplified edit flow**: Removed backup/restore mechanism in favor of in-memory validation
  - Original file is only modified after validation passes
  - "Restore previous config" → "Keep original config" (file was never changed)
  - Re-edit now preserves your changes instead of reloading from disk

## [0.1.4] - 2025-12-19

### Added
- **Prompt auto-completion**: Custom prompts now automatically append missing required sections
  - Commit prompts: auto-append `{diff}` and context if missing
  - Review prompts: auto-append `{diff}` if missing, **always** append JSON output format
- **Verbose prompt output**: `-v` flag now shows the complete prompt sent to LLM (both commit and review)

### Fixed
- **JSON response parsing**: Fixed `clean_json_response` chain bug where `unwrap_or(response)` incorrectly fell back to original response
- **Defensive JSON extraction**: Now extracts content between first `{` and last `}`, robust against various LLM response wrappers

## [0.1.3] - 2025-12-19

### Added
- **Config validation on edit**: `gcop config edit` now validates configuration after saving (like `visudo`), with options to re-edit, restore backup, or ignore errors
- Colored menu options for config edit validation prompts

### Changed
- **Lazy config loading**: `config`, `init`, and `alias` commands now use default config when config file is corrupted, allowing recovery via `config edit`
- **Provider refactor**: Extracted common HTTP request logic into `send_llm_request()` function in `base.rs`, reducing ~50 lines of duplicate code

### Fixed
- OpenAI provider now returns explicit error when API response contains no choices (instead of silently returning empty string)
- `config edit` can now run even when config file is corrupted (previously would fail to start)

## [0.1.2] - 2025-12-20

### Added
- GPG commit signing support - commits now use native git CLI to properly support `commit.gpgsign` and `user.signingkey` configurations

### Changed
- **Architecture refactor**: Introduced state machine pattern for commit workflow, replacing boolean flags with explicit `CommitState` enum
- **Provider abstraction**: Extracted common LLM provider code into `src/llm/provider/base.rs`, reducing ~150 lines of duplication
- **Constants centralization**: Created `src/constants.rs` for all magic numbers and default values
- Feedback is now accumulated across retries - each "Retry with feedback" adds to previous feedback instead of replacing it
- Edit action now returns to the action menu instead of directly committing, allowing further edits or regeneration

### Fixed
- GPG signing now works correctly (previously git2-rs didn't support global GPG configuration)
- User feedback persists across retry cycles for better commit message refinement

### Removed
- Removed empty `src/utils.rs` file

## [0.1.1] - 2025-12-18
### Added
- New git alias `git cp` for committing with AI message and pushing in one command

## [0.1.0] - 2025-12-17

### Added

**Core Features**:
- AI-powered commit message generation (Claude, OpenAI, Ollama)
- AI code review with security and performance insights
- Interactive commit workflow (Accept, Edit, Retry, Retry with feedback, Quit)

**Commands**:
- `init` - Interactive configuration wizard
- `commit` - AI commit message generation with retry and feedback loop
- `review` - AI code review (changes, commit, range, file)
- `config` - Configuration management (edit, validate, show)
- `alias` - Git alias management (install, list, remove)

**Git Aliases**:
- 11 convenient git aliases (`git c`, `git r`, `git ac`, `git acp`, `git p`, `git pf`, `git undo`, `git gconfig`, `git ghelp`, `git cop`, `git gcommit`)
- Alias management with conflict detection
- Colored status display

**UI/UX**:
- Colored terminal output with configurable enable/disable
- Spinner animations for API calls
- Interactive menus with dialoguer
- Beautiful diff stats display
- Dual-language documentation (English + Chinese)

**Configuration**:
- Multiple LLM providers support (Claude, OpenAI, Ollama, custom)
- Custom prompts with template variables
- Flexible configuration (file + environment variables)
- Secure config file permissions (chmod 600)
- Configuration validation and testing

**Documentation**:
- Complete English and Chinese documentation
- Git aliases guide
- Command reference
- Configuration guide
- Installation guide
- Provider setup guide
- Custom prompts guide
- Troubleshooting guide

### Changed
- Rewrote from Python to Rust for better performance and reliability
- `git undo` uses `--soft` flag (keeps changes staged instead of unstaged)
- Simplified configuration file from 230 lines to 75 lines

### Fixed
- Edit action properly returns to menu without triggering regeneration
- Commit message display no longer duplicates after editing

[0.2.1]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.2.1
[0.2.0]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.2.0
[0.1.6]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.6
[0.1.5]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.5
[0.1.4]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.4
[0.1.3]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.3
[0.1.2]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.2
[0.1.1]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.1
[0.1.0]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.0
