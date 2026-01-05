# Changelog

All notable changes to gcop-rs will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2025-01-05

### Added

- **Real API Health Checks for Providers**: Enhanced `validate()` methods for all LLM providers
  - Claude/OpenAI: Send minimal test requests (`max_tokens=1`) to verify API connectivity
  - Ollama: Check `/api/tags` endpoint and verify configured model exists
  - FallbackProvider: Validate all providers and aggregate results
  - `gcop config validate` now provides detailed status and helpful error suggestions
  - New test suite: `tests/provider_validation_test.rs` with 9 comprehensive tests

- **Git Repository Test Coverage**: New comprehensive test suite for repository operations
  - Added `tests/git_repository_test.rs` with 14 tests covering edge cases
  - Tests: empty repos, large file limits, Unicode paths, first commit, invalid inputs
  - Tests: Detached HEAD state, error handling, concurrent test safety (using `serial_test`)

- **Review Command Tests**: Added integration tests for code review functionality
  - New `tests/review_command_test.rs` with 6 tests
  - Tests all 4 target types: Changes/Commit/Range/File routing
  - Tests error handling: empty diff validation, LLM failure propagation
  - Refactored `review.rs` with dependency injection support for testability

- **MSRV Declaration**: Fixed Minimum Supported Rust Version to 1.92.0
  - Added `rust-toolchain.toml` for consistent toolchain across environments
  - Added `rust-version = "1.92.0"` in `Cargo.toml` for crates.io compliance
  - Added MSRV check job in CI/CD pipeline
  - Updated all documentation (README, installation guides) with Rust 1.92.0 requirement

### Changed

- **Review Command Architecture**: Refactored for better testability
  - Split `run()` (public API) and `run_internal()` (accepts trait objects)
  - Enables dependency injection for testing without changing public interface

- **Config Validation Output**: Improved error messages and suggestions
  - More detailed provider validation status display
  - Better error messages with actionable suggestions

### Improved

- **Test Coverage**: Increased from 248 to 277 tests (+29 tests, +11.7%)
  - All P0 and P1 priority improvements from TODO completed
  - Code quality score improved: A- with comprehensive test coverage

### Documentation

- Added verbose mode security warnings to troubleshooting guides
- Updated system requirements in all documentation (English/Chinese)
- Fixed typo in Chinese README link
- Added detailed release notes (English/Chinese)

## [0.5.1] - 2025-01-05

### Fixed

- **Empty Repository Support** (#11): Fixed `UnbornBranch` error when running commands in empty repositories
  - `gcop commit` now works correctly in repositories with no commits yet
  - `gcop stats` shows friendly warning instead of crashing in empty repos
  - Added `is_empty()` method to detect unborn branch state
  - Empty repositories now compare staged changes against empty tree instead of HEAD
  - Supports creating the first commit in a new repository

## [0.5.0] - 2025-12-24

### Added

- **Provider Fallback Support**: New `fallback_providers` config option for automatic failover
  - When primary provider fails, automatically tries the next provider in list
  - Shows warning messages when switching providers
  - Supports both streaming and non-streaming modes
- **Claude Streaming Support**: Claude provider now supports streaming responses
  - Real-time typing effect like ChatGPT when generating commit messages
  - Uses SSE (Server-Sent Events) for efficient streaming
- **Retry-After Header Support**: Enhanced API retry mechanism
  - Respects `Retry-After` header from API responses (429 rate limits)
  - New `max_retry_delay_ms` config option to cap maximum wait time (default: 60s)
- **Colored Provider Output**: LLM providers now display colored warning/info messages

### Changed

- **Improved User Experience**: Enhanced commit and review command interactions
- **Better Error Handling**: Restructured error types for clearer, more user-friendly messages
- **Refactored LLM Module**: Extracted common prompt building and response processing logic

### Fixed

- **Streaming Error Handling**: Improved error handling and log levels for streaming responses

### Documentation

- Updated streaming output documentation
- Added Claude configuration examples
- Added installation update/uninstall instructions for various package managers

## [0.4.3] - 2025-12-24

### Changed

- **PyPI native wheels**: Migrated from Python wrapper to **maturin** build system
  - Pre-built native wheels for 6 platforms (Linux/macOS/Windows × AMD64/ARM64)
  - Faster installation (no runtime binary download)
  - Better reliability (no network dependency after install)

### Added

- **VitePress documentation**: Migrated docs to VitePress
  - Modern, fast documentation site with local search
  - Multi-language support (English + Chinese)
  - Auto language redirect based on browser preference
  - Improved navigation with sidebar

## [0.4.2] - 2025-12-23

### Added

- **PyPI support**: Install via `pipx install gcop-rs` or `pip install gcop-rs`
  - Python wrapper that auto-downloads pre-compiled Rust binary on first run
  - Supports all platforms (macOS, Linux, Windows)
- **Colored CLI help**: `--help` output now displays with color highlighting
  - Headers (Usage, Commands) in bold green
  - Commands and options in bold cyan

## [0.4.1] - 2025-12-23

### Added

- **Homebrew tap support**: Install via `brew tap AptS-1547/gcop-rs && brew install gcop-rs`
  - Supports macOS (Intel/Apple Silicon) and Linux (x86_64/ARM64)
  - Auto-updated on each release via GitHub Actions
- **cargo-binstall support**: Install pre-compiled binaries via `cargo binstall gcop-rs`
  - No compilation required, downloads platform-specific binary directly

## [0.4.0] - 2025-12-23

### Added

- **New `stats` command**: Show repository commit statistics
  - Total commits and contributors count
  - Repository time span (first to last commit)
  - Top contributors ranking with commit counts and percentages
  - Recent activity (last 4 weeks) with ASCII bar chart
  - Multiple output formats: `text` (default), `json`, `markdown`
  - Author filter: `--author <name>` to filter by author name or email
- **New `--dry-run` option for commit command**: Generate and print commit message without actually committing
- **New `git s` alias**: Shorthand for `gcop-rs stats`

### Dependencies

- Added `chrono = "0.4"` for date/time handling in stats

## [0.3.1] - 2025-12-23

### Added

- **Extended CI build platforms**:
  - `aarch64-unknown-linux-gnu` (Linux ARM64) - for Raspberry Pi 64-bit, AWS Graviton, etc.
  - `x86_64-apple-darwin` (macOS Intel) - restored support
  - `aarch64-pc-windows-msvc` (Windows ARM64)

### Changed

- **git2 dependency optimization**: Disabled default features, removed openssl-related dependencies
  - Simplified dependency tree, reduced compile time
  - Improved cross-platform build compatibility

### Documentation

- Updated README with `gcop config edit` command usage

## [0.3.0] - 2025-12-22

### Added

- **Streaming output for OpenAI provider**: Real-time typing effect like ChatGPT when generating commit messages
  - New SSE (Server-Sent Events) parser module (`llm/provider/streaming.rs`)
  - New streaming UI component (`ui/streaming.rs`)
  - `LLMProvider` trait extended with `supports_streaming()` and `generate_commit_message_streaming()` methods
  - Non-streaming providers automatically fallback to spinner mode
- **New `streaming` config option** in `[ui]` section (default: `true`)
- Colored prompt for retry feedback input

### Changed

- Simplified retry option text: "Retry with feedback - Add instructions" (was "Regenerate with instructions")
- Commit generation now returns `(message, already_displayed)` tuple to avoid duplicate display in streaming mode

### Dependencies

- Added `bytes = "1.10"` for stream byte handling
- Added `futures = "0.3"` for async stream processing
- `reqwest` now uses `stream` feature

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

[0.6.0]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.6.0
[0.5.1]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.5.1
[0.5.0]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.5.0
[0.4.3]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.4.3
[0.4.2]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.4.2
[0.4.1]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.4.1
[0.4.0]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.4.0
[0.3.1]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.3.1
[0.3.0]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.3.0
[0.2.1]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.2.1
[0.2.0]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.2.0
[0.1.6]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.6
[0.1.5]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.5
[0.1.4]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.4
[0.1.3]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.3
[0.1.2]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.2
[0.1.1]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.1
[0.1.0]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.0
