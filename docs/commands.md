# Command Reference

Complete reference for all gcop-rs commands and options.

## Global Options

These options can be used with any command:

| Option | Description |
|--------|-------------|
| `--provider <NAME>` | Override default LLM provider (claude, openai, ollama, or custom) |
| `--verbose`, `-v` | Enable verbose logging (shows API requests and responses) |
| `--help`, `-h` | Show help information |
| `--version`, `-V` | Show version information |

**Example**:
```bash
gcop-rs --provider openai commit
gcop-rs -v review
```

---

## Commands

### init

Initialize gcop-rs configuration with an interactive wizard.

**Synopsis**:
```bash
gcop-rs init
```

**Description**:

Interactive setup that guides you through:
1. Creating configuration directory (`~/.config/gcop/`)
2. Copying example configuration file
3. Setting secure file permissions (600)
4. Optionally installing git aliases

**Options**: None

**Example**:
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

✓ Installed 11 aliases
```

**What it creates**:
- `~/.config/gcop/config.toml` (from `examples/config.toml.example`)
- Git aliases in `~/.gitconfig` (if you choose to install them)

**When to use**: First time setup or when reconfiguring from scratch.

---

### commit

Generate AI-powered commit message and create a commit.

**Synopsis**:
```bash
gcop-rs commit [OPTIONS]
```

**Description**:

Analyzes your staged changes, generates a conventional commit message using AI, and creates a git commit after your approval.

**Options**:

| Option | Description |
|--------|-------------|
| `--no-edit` | Skip opening editor for manual editing |
| `--yes` | Skip confirmation menu and accept generated message |
| `--provider <NAME>` | Use specific provider (overrides default) |

**Interactive Actions**:

After generating a message, you'll see a menu:

1. **Accept** - Use the generated message and create commit
2. **Edit** - Open your `$EDITOR` to manually modify the message (returns to menu after editing)
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
- Try "Retry with feedback" if the message doesn't capture your intent

---

### review

Perform AI-powered code review of changes, commits, or files.

**Synopsis**:
```bash
gcop-rs review [TARGET] [OPTIONS]
```

**Targets**:

| Target | Syntax | Description |
|--------|--------|-------------|
| *(default)* | `gcop-rs review` | Review uncommitted changes |
| Commit | `--commit <HASH>` | Review a specific commit |
| Range | `--range <RANGE>` | Review commit range (e.g., `HEAD~3..HEAD`) |
| File | `--file <PATH>` | Review a specific file |

**Options**:

| Option | Description |
|--------|-------------|
| `--format <FORMAT>` | Output format: `text` (default), `json`, or `markdown` |
| `--provider <NAME>` | Use specific provider |

**Examples**:

```bash
# Review uncommitted changes (default)
gcop-rs review

# Review last commit
gcop-rs review --commit HEAD
gcop-rs review --commit abc123

# Review last 3 commits
gcop-rs review --range HEAD~3..HEAD

# Review specific file
gcop-rs review --file src/auth.rs

# Output as JSON for automation
gcop-rs review --format json > review.json

# Output as markdown for documentation
gcop-rs review --format markdown > REVIEW.md
```

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
gcop-rs config <SUBCOMMAND>
```

**Subcommands**:

#### `config edit`

Open configuration file in your default editor with validation.

**Usage**:
```bash
gcop-rs config edit
```

**Opens**: `~/.config/gcop/config.toml` in `$EDITOR` (falls back to `vi`)

**Validation**: After saving, the configuration is automatically validated (like `visudo`). If validation fails, you'll see a menu:

```
✗ Config validation failed: TOML parse error...

? What would you like to do?
> ✎ Re-edit the config file
  ↩ Restore previous config
  ⚠ Ignore errors and keep current (dangerous)
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
- Configuration file syntax
- Required fields presence
- API key format
- Provider connectivity (makes a test API call)

**Example output**:
```
✓ Configuration file is valid
✓ Claude provider validated successfully
⚠ OpenAI provider: API key not set (skipped)
```

**When to use**:
- After editing configuration
- Troubleshooting connection issues
- Verifying API keys

---

#### `config show`

Display current configuration.

**Usage**:
```bash
gcop-rs config show
```

**Example output**:
```toml
[llm]
default_provider = "claude"

[llm.providers.claude]
model = "claude-sonnet-4-5-20250929"
endpoint = "https://api.anthropic.com/v1/messages"

[commit]
show_diff_preview = true
allow_edit = true

[ui]
colored = true
```

**Note**: API keys are hidden for security.

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
| `--list` | List all available aliases and their status |
| `--force` | Force install, overwriting conflicts |
| `--remove` | Remove aliases (requires `--force` to confirm) |

**Examples**:

#### Install Aliases

```bash
# Install all 11 aliases
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

## Command Chaining

gcop-rs commands can be combined with standard git commands:

```bash
# Review then commit
gcop-rs review && gcop-rs commit

# Commit then push (if using full commands)
gcop-rs commit --yes && git push

# Or use alias
git acp  # Equivalent to: add -A && commit && push
```

## Exit Codes

gcop-rs uses standard exit codes:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (API error, git error, etc.) |
| 2 | User cancelled (Ctrl+C or selected Quit) |
| 3 | Configuration error |
| 4 | Invalid input (no changes, invalid commit hash, etc.) |

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
| `EDITOR` | Editor for `--edit` and `config edit` |
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
