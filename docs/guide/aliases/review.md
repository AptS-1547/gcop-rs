# Review Aliases

## `git r` - Review Changes

Get AI-powered code review.

**Command**: `gcop-rs review <TARGET>`

**Usage**:
```bash
# Review working tree changes (unstaged, similar to `git diff`)
git r changes

# Review other targets
git r commit HEAD
git r range HEAD~3..HEAD
git r file src/auth.rs

# Review with different format
git r --format json changes
git r --format markdown changes
```

`--format` / `--json` belong to the `review` command, so place them before the target (`changes`, `commit`, `range`, `file`).

**What it reviews**: `git r changes` reviews unstaged working tree changes (similar to `git diff`).

**When to use**:
- Before committing to catch potential issues
- For quick code quality checks
- To get suggestions for improvements

**Example workflow**:
```bash
# Make changes
vim src/auth.rs

# Review changes
git r changes

📝 Summary:
Added JWT token validation with proper error handling.

🔍 Issues found:
  1. WARNING: Consider adding rate limiting for token validation

💡 Suggestions:
  • Add unit tests for edge cases
  • Document the token validation logic

# Address issues, then commit
git c
```

## See Also

- [Aliases Overview](../aliases.md)
- [Commit Aliases](./commit.md)
- [Troubleshooting: Review and Git](../troubleshooting/review-and-git.md)
