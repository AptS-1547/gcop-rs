# Changelog

All notable changes to gcop-rs will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[0.1.4]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.4
[0.1.3]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.3
[0.1.2]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.2
[0.1.1]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.1
[0.1.0]: https://github.com/AptS-1547/gcop-rs/releases/tag/v0.1.0
