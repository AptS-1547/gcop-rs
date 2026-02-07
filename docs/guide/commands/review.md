# review

Perform AI-powered code review of changes, commits, or files.

**Synopsis**:
```bash
gcop-rs review [OPTIONS] <COMMAND>
```

**Commands**:

| Command | Syntax | Description |
|--------|--------|-------------|
| Changes | `gcop-rs review changes` | Review unstaged working tree changes (index → working tree; similar to `git diff`) |
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
gcop-rs review --format json changes > review.json

# Output as markdown for documentation
gcop-rs review --format markdown changes > REVIEW.md
```

> **Note**: `--format` / `--json` are options of the `review` command itself, so they must appear before the subcommand target (for example, `review --format json changes`).
>
> **Note**: `review changes` currently reviews unstaged changes only (index → working tree). Staged changes are not included.
>
> **Note**: `review file <PATH>` currently supports files only (directories are not supported).

> **Note**: Very large review input is truncated before sending to the LLM. You can tune this limit via `[llm].max_diff_size` in config.

**Output Format (text)**:

```
ℹ Review: Unstaged working tree changes

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

## See Also

- [Command Overview](../commands.md)
- [Troubleshooting](../troubleshooting.md)
