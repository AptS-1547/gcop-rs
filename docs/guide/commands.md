# Command Reference

Complete reference for all gcop-rs commands and options.

## Global Options

These options can be used with any command:

| Option | Description |
|--------|-------------|
| `--provider <NAME>`, `-p` | Override default LLM provider (claude, openai, ollama, or custom) |
| `--verbose`, `-v` | Enable verbose logging (shows API requests and responses) |
| `--help`, `-h` | Show help information |
| `--version`, `-V` | Show version information |

**Example**:
```bash
gcop-rs --provider openai commit
gcop-rs -v review changes
```

---

## Commands

### init

Initialize gcop-rs configuration with an interactive wizard.

**Synopsis**:
```bash
gcop-rs init [OPTIONS]
```

**Description**:

Interactive setup that guides you through:
1. Creating configuration directory (platform-specific location)
2. Copying example configuration file
3. Setting secure file permissions (Unix/Linux/macOS only)
4. Optionally installing git aliases

**Options**:

| Option | Description |
|--------|-------------|
| `--force`, `-f` | Force overwrite existing config |

**Example** (Linux):
```bash
$ gcop-rs init

✓ Created config directory: /home/user/.config/gcop
✓ Created config file: /home/user/.config/gcop/config.toml
✓ Set file permissions: 600

ℹ Next steps:
  1. Edit config file: gcop-rs config edit
  2. Set your API key for your preferred provider
  3. Test with: gcop-rs commit --help

Install git aliases? (Y/n): y

[1/2] Installing git aliases...
  ✓  git c          → AI commit
  ✓  git r          → AI review
  ...

✓ Installed 14 aliases
```

**What it creates**:
- Config file at platform-specific location (from `examples/config.toml.example`)
- Git aliases in `~/.gitconfig` (if you choose to install them)

**When to use**: First time setup or when reconfiguring from scratch.

---

### commit

Generate AI-powered commit message and create a commit.

**Synopsis**:
```bash
gcop-rs commit [OPTIONS] [FEEDBACK...]
```

**Description**:

Analyzes your staged changes, generates a conventional commit message using AI, and creates a git commit after your approval.

**Options**:

| Option | Description |
|--------|-------------|
| `--format <FORMAT>`, `-f` | Output format: `text` (default) or `json` (json implies no commit) |
| `--json` | Shortcut for `--format json` |
| `--no-edit`, `-n` | Skip opening editor for manual editing |
| `--yes`, `-y` | Skip confirmation menu and accept generated message |
| `--dry-run`, `-d` | Only generate and print commit message, do not commit |
| `--provider <NAME>`, `-p` | Use specific provider (overrides default) |

**Feedback (optional)**:

You can append free-form text after the options to guide commit message generation.

```bash
# With quotes (recommended)
gcop-rs commit "use Chinese and be concise"

# Or without quotes (will be treated as one combined instruction)
gcop-rs commit use Chinese and be concise
```

> **Note**: In JSON mode (`--json` / `--format json`), gcop-rs runs non-interactively and **does not create a commit** (it only prints JSON output).

**Interactive Actions**:

After generating a message, you'll see a menu:

1. **Accept** - Use the generated message and create commit
2. **Edit** - Open your `$VISUAL` / `$EDITOR` (platform default if not set) to manually modify the message (returns to menu after editing)
3. **Retry** - Regenerate a new message without additional instructions
4. **Retry with feedback** - Provide instructions for regeneration (e.g., "use Chinese", "be more concise", "add more details"). Feedback accumulates across retries, allowing you to progressively refine the message
5. **Quit** - Cancel the commit process

**Examples**:

```bash
# Basic usage
git add src/auth.rs
gcop-rs commit

# Skip all prompts
git add .
gcop-rs commit --no-edit --yes

# Use different provider
gcop-rs commit --provider openai

# Verbose mode (see API calls)
gcop-rs -v commit

# JSON output for automation (does not create commit)
gcop-rs commit --json > commit.json
```

**Workflow**:

```bash
$ git add src/auth.rs src/middleware.rs
$ gcop-rs commit

[1/4] Analyzing staged changes...
2 files changed, 45 insertions(+), 12 deletions(-)

ℹ Generated commit message:
feat(auth): implement JWT token validation

Add middleware for validating JWT tokens with proper
error handling and expiration checks.

[3/4] Choose next action...
Choose next action:
> Accept
  Edit
  Retry
  Retry with feedback
  Quit

[Selected: Accept]

[4/4] Creating commit...
✓ Commit created successfully!
```

**Tips**:
- Stage only the changes you want in this commit before running
- Use `--yes` in CI/CD pipelines to skip interactive prompts
- Use `--json` / `--format json` to generate a message for automation (no commit)
- Try "Retry with feedback" if the message doesn't capture your intent

**Output Format (json)**:

```json
{
  "success": true,
  "data": {
    "message": "feat(auth): implement JWT token validation",
    "diff_stats": {
      "files_changed": ["src/auth.rs", "src/middleware.rs"],
      "insertions": 45,
      "deletions": 12,
      "total_changes": 57
    },
    "committed": false
  }
}
```

---

### review

Perform AI-powered code review of changes, commits, or files.

**Synopsis**:
```bash
gcop-rs review [OPTIONS] <COMMAND>
```

**Commands**:

| Command | Syntax | Description |
|--------|--------|-------------|
| Changes | `gcop-rs review changes` | Review working tree changes (similar to `git diff`) |
| Commit | `gcop-rs review commit <HASH>` | Review a specific commit |
| Range | `gcop-rs review range <RANGE>` | Review commit range (e.g., `HEAD~3..HEAD`) |
| File | `gcop-rs review file <PATH>` | Review a single file (directories currently unsupported) |

**Options**:

| Option | Description |
|--------|-------------|
| `--format <FORMAT>`, `-f` | Output format: `text` (default), `json`, or `markdown` |
| `--json` | Shortcut for `--format json` |
| `--provider <NAME>`, `-p` | Use specific provider |

**Examples**:

```bash
# Review working tree changes
gcop-rs review changes

# Review last commit
gcop-rs review commit HEAD
gcop-rs review commit abc123

# Review last 3 commits
gcop-rs review range HEAD~3..HEAD

# Review a file
gcop-rs review file src/auth.rs

# Output as JSON for automation
gcop-rs review changes --format json > review.json

# Output as markdown for documentation
gcop-rs review changes --format markdown > REVIEW.md
```

> **Note**: `review changes` currently reviews unstaged changes only (index → working tree). Staged changes are not included.
>
> **Note**: `review file <PATH>` currently supports files only (directories are not supported).

**Output Format (text)**:

```
ℹ Review: Uncommitted changes

📝 Summary:
Added JWT authentication with proper error handling.
Overall code quality is good.

🔍 Issues found:

  1. WARNING: Missing error handling in token refresh
     Location: src/auth.rs:45

  2. INFO: Consider adding rate limiting
     Location: src/middleware.rs:12

💡 Suggestions:
  • Add unit tests for edge cases
  • Document the token validation logic
  • Consider extracting validation into separate function
```

**Tips**:
- Use before committing to catch issues early
- Use `--format json` for CI/CD integration
- Configure `min_severity` in config to filter noise

---

### config

Manage gcop-rs configuration.

**Synopsis**:
```bash
gcop-rs config [SUBCOMMAND]
```

If no subcommand is provided, it defaults to `gcop-rs config edit`.

**Subcommands**:

#### `config edit`

Open configuration file in your default editor with validation.

**Usage**:
```bash
gcop-rs config edit
```

**Opens**: Config file (platform-specific location) in `$VISUAL` / `$EDITOR` (platform default if not set)

**Validation**: After saving, the configuration is automatically validated (like `visudo`). If validation fails, you'll see a menu:

```
✗ Config validation failed: TOML parse error...

? What would you like to do?
> ✎ Re-edit the config file
  ↩ Keep original config
  ⚠ Ignore errors and save anyway (dangerous)
```

**Recovery**: Even if your config file is corrupted, `config edit` will still work, allowing you to fix it.

**When to use**: Modify API keys, models, or custom prompts.

> **Tip**: Always use `gcop-rs config edit` instead of editing the config file directly to benefit from automatic validation.

---

#### `config validate`

Validate configuration and test provider connection.

**Usage**:
```bash
gcop-rs config validate
```

**Checks**:
- Loads and parses configuration (defaults + config file + `GCOP_*` env overrides)
- Lists configured providers
- Validates provider connections by sending minimal test requests (default provider + configured `fallback_providers`)
- Succeeds if at least one configured provider validates

**Example output**:
```
[1/2] Loading configuration...
✓ Configuration loaded successfully

Configured providers:
  • claude

[2/2] Testing provider connection...
✓ Provider 'claude' validated successfully
```

**When to use**:
- After editing configuration
- Troubleshooting connection issues
- Verifying API keys

---

### alias

Manage git aliases for gcop-rs.

**Synopsis**:
```bash
gcop-rs alias [OPTIONS]
```

**Options**:

| Option | Description |
|--------|-------------|
| *(none)* | Install all aliases (default action) |
| `--list`, `-l` | List all available aliases and their status |
| `--force`, `-f` | Force install, overwriting conflicts |
| `--remove`, `-r` | Remove aliases (requires `--force` to confirm) |

**Examples**:

#### Install Aliases

```bash
# Install all aliases
gcop-rs alias

# Output:
[1/2] Installing git aliases...
  ✓  git c          → AI commit
  ✓  git r          → AI review
  ℹ  git p          → Push (already set)

✓ Installed 10 aliases
ℹ Skipped 1 alias (already exists or conflicts)
```

#### List Aliases

```bash
gcop-rs alias --list

# Output:
ℹ Available git aliases for gcop-rs:

  git cop        → Main entry point                  [✓ installed]
  git c          → AI commit                         [✓ installed]
  git r          → AI review                         [  not installed]
  git p          → Push                              [⚠ conflicts: !my-push]
  ...
```

#### Force Install

```bash
# Overwrite conflicting aliases
gcop-rs alias --force
```

#### Remove Aliases

```bash
# Preview what will be removed
gcop-rs alias --remove

# Output:
⚠ This will remove all gcop-related git aliases

ℹ Aliases to be removed:
  - git c
  - git r
  - git ac
  ...

ℹ Use --force to confirm:
  gcop-rs alias --remove --force

# Actually remove
gcop-rs alias --remove --force
```

**When to use**:
- After installation: Install aliases for convenience
- After gcop-rs updates: Reinstall with `--force`
- When uninstalling: Remove with `--remove --force`

---

### stats

Show repository commit statistics.

**Synopsis**:
```bash
gcop-rs stats [OPTIONS]
```

**Description**:

Analyzes commit history and displays statistics including total commits, contributors, time span, and recent activity.

**Options**:

| Option | Description |
|--------|-------------|
| `--format <FORMAT>`, `-f` | Output format: `text` (default), `json`, or `markdown` |
| `--json` | Shortcut for `--format json` |
| `--author <NAME>` | Filter statistics by author name or email |

**Examples**:

```bash
# Basic usage (text format)
gcop-rs stats

# Output as JSON for automation
gcop-rs stats --format json
gcop-rs stats --json

# Output as Markdown for documentation
gcop-rs stats --format markdown > STATS.md

# Filter by specific author
gcop-rs stats --author "john"
gcop-rs stats --author "john@example.com"
```

**Output Format (text)**:

```
ℹ Repository Statistics
========================================

[] Overview
  Total commits:  156
  Contributors:   3
  Time span:      2024-06-15 ~ 2025-12-23 (192 days)

[] Top Contributors
  #1  AptS-1547 <esaps@esaps.net>  142 commits (91.0%)
  #2  bot <noreply@github.com>      8 commits  (5.1%)
  #3  contributor <x@y.com>         6 commits  (3.8%)

[] Recent Activity (last 4 weeks)
  2025-W52: ████████████           12
  2025-W51: ████████████████████   20
  2025-W50: ██████                  6
  2025-W49: ████████████████       16
```

**Output Format (json)**:

```json
{
  "success": true,
  "data": {
    "total_commits": 156,
    "total_authors": 3,
    "first_commit_date": "2024-06-15T10:30:00+08:00",
    "last_commit_date": "2025-12-23T15:43:34+08:00",
    "authors": [
      {"name": "AptS-1547", "email": "esaps@esaps.net", "commits": 142},
      {"name": "bot", "email": "noreply@github.com", "commits": 8}
    ],
    "commits_by_week": {
      "2025-W49": 16,
      "2025-W50": 6,
      "2025-W51": 20,
      "2025-W52": 12
    }
  }
}
```

**Tips**:
- Use `--format json` for CI/CD integration or scripting
- Use `--author` to see individual contributor statistics
- The ASCII bar chart shows relative activity across weeks

---

## Command Chaining

gcop-rs commands can be combined with standard git commands:

```bash
# Review then stage + commit
gcop-rs review changes && git add -A && gcop-rs commit

# Commit then push (if using full commands)
gcop-rs commit --yes && git push

# Or use alias
git acp  # Equivalent to: add -A && commit && push
```

## Exit Codes

gcop-rs uses standard exit codes:

| Code | Meaning |
|------|---------|
| 0 | Success (also used when you cancel from interactive menus) |
| 1 | Runtime error (API error, git error, config error, etc.) |
| 2 | CLI usage error (invalid flags/args; generated by clap) |

**Usage in scripts**:
```bash
if gcop-rs commit --yes; then
    echo "Commit successful"
    git push
else
    echo "Commit failed or cancelled"
fi
```

## Environment Variables

These environment variables affect gcop-rs behavior:

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Claude API key (fallback if not in config) |
| `OPENAI_API_KEY` | OpenAI API key (fallback) |
| `VISUAL` / `EDITOR` | Editor for commit message editing and `gcop-rs config edit` |
| `GCOP_UI_LANGUAGE` | Force UI language early in startup (before config is fully loaded) |
| `GCOP_*` | Override config values via environment variables (e.g., `GCOP_UI_COLORED=false`) |
| `NO_COLOR` | Disable colored output (set to any value) |

**Example**:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export EDITOR="vim"
gcop-rs commit
```

## See Also

- [Git Aliases Guide](aliases.md) - Detailed guide to git aliases
- [Configuration Reference](configuration.md) - All configuration options
- [Provider Setup](providers.md) - Configure LLM providers
- [Troubleshooting](troubleshooting.md) - Common issues
