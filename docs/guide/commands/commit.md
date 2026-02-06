# commit

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

## See Also

- [Command Overview](../commands.md)
- [Configuration Guide](../configuration.md)
- [LLM Providers](../providers.md)
