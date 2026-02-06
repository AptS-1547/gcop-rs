# Git Aliases

Complete guide to gcop-rs git aliases - convenient shortcuts for your daily Git workflow.

## Overview

gcop-rs provides 14 carefully designed git aliases that streamline common tasks:

| Alias | Command | Description |
|-------|---------|-------------|
| `git c` | `gcop-rs commit` | Quick AI-powered commit |
| `git r` | `gcop-rs review <TARGET>` | AI review of changes |
| `git s` | `gcop-rs stats` | Repository statistics |
| `git ac` | `git add -A && gcop-rs commit` | Add all and commit |
| `git cp` | `gcop-rs commit && git push` | Commit and push |
| `git acp` | `git add -A && gcop-rs commit && git push` | Add, commit, and push |
| `git cop` | `gcop-rs` | Main gcop-rs entry point |
| `git gcommit` | `gcop-rs commit` | Full command alias |
| `git ghelp` | `gcop-rs --help` | Show help |
| `git gconfig` | `gcop-rs config edit` | Edit configuration |
| `git p` | `git push` | Quick push |
| `git pf` | `git push --force-with-lease` | Safer force push |
| `git amend` | `git commit --amend` | Amend last commit |
| `git undo` | `git reset --soft HEAD^` | Undo last commit |

## Installation

### Quick Install

```bash
# Install all aliases
gcop-rs alias

# Verify installation
gcop-rs alias --list
```

### During Initial Setup

```bash
# The init command will prompt you
gcop-rs init
```

When prompted "Install git aliases?", select `Yes` to install all aliases automatically.

### Verification

Check installed aliases:

```bash
gcop-rs alias --list
```

Output:
```
ℹ Available git aliases for gcop-rs:

  git cop        → Main entry point for gcop-rs              [✓ installed]
  git gcommit    → AI commit message and commit changes      [✓ installed]
  git c          → Shorthand for 'git gcommit'               [✓ installed]
  git r          → AI review of uncommitted changes          [✓ installed]
  ...
```

## Available Aliases

### Commit Aliases

#### `git c` - Quick Commit

The fastest way to create AI-powered commits.

**Command**: `gcop-rs commit`

**Usage**:
```bash
# Stage your changes
git add src/auth.rs

# Generate and commit
git c

# Or with options
git c --no-edit    # Skip editor
git c --yes        # Skip confirmation menu
```

**When to use**: Your primary commit command. Use this instead of `git commit` for AI-generated messages.

---

#### `git ac` - Add and Commit

Add all changes and commit in one step.

**Command**: `git add -A && gcop-rs commit`

**Usage**:
```bash
# Modified several files?
git ac
```

**Equivalent to**:
```bash
git add -A
git c
```

**When to use**: When you want to commit all changes without manually staging them first.

---

#### `git acp` - Add, Commit, and Push

Complete workflow: add all changes, commit with AI, and push to remote.

**Command**: `git add -A && gcop-rs commit && git push`

**Usage**:
```bash
# Complete a feature and push
git acp
```

**Equivalent to**:
```bash
git add -A
git c
git push
```

**When to use**: For quick iterations when you're confident about pushing immediately after committing.

**⚠️ Note**: Only use when you're sure you want to push. The commit and push will only happen if the previous command succeeds.

---

### Review Aliases

#### `git r` - Review Changes

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
git r changes --format json
git r changes --format markdown
```

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

---

### Utility Aliases

#### `git amend` - Amend Last Commit

Amend the last commit (equivalent to `git commit --amend`).

**Command**: `git commit --amend`

**Usage**:
```bash
git amend
```

#### `git undo` - Undo Last Commit

Safely undo the last commit while keeping your changes staged.

**Command**: `git reset --soft HEAD^`

**Usage**:
```bash
# Just made a commit but want to modify it?
git undo

# Your changes are still staged, edit them
vim src/auth.rs

# Commit again with new message
git c
```

**What it does**:
- Moves HEAD back one commit (`HEAD^` = previous commit)
- **Keeps changes in staging area** (ready to commit)
- Preserves your working directory

**When to use**:
- Wrong commit message
- Forgot to include a file
- Want to split the commit
- Need to amend the changes

**⚠️ Safety**: This is safe for local commits. If you've already pushed, see "Undoing Pushed Commits" below.

**Example**:
```bash
$ git log --oneline
abc123 feat: add auth (current HEAD)
def456 fix: typo

$ git undo

$ git log --oneline
def456 fix: typo (current HEAD)

$ git status
Changes to be committed:
  modified:   src/auth.rs
  # Your changes are still staged!
```

---

#### `git p` - Quick Push

Shorthand for `git push`.

**Command**: `git push`

**Usage**:
```bash
git p
```

**When to use**: When you want a shorter push command.

---

#### `git pf` - Safer Force Push

Force push with `--force-with-lease` for safety.

**Command**: `git push --force-with-lease`

**Usage**:
```bash
# After rebasing
git rebase -i HEAD~3
git pf
```

**Why `--force-with-lease`**:
- Safer than `--force`
- Only pushes if nobody else has pushed to the remote
- Prevents accidentally overwriting others' work

**When to use**:
- After rebasing
- After amending commits
- When you need to rewrite history

**⚠️ Warning**: Only force push to branches you own. Never force push to `main` or `master`!

---

#### `git gconfig` - Edit Configuration

Open gcop-rs configuration in your default editor.

**Command**: `gcop-rs config edit`

**Usage**:
```bash
git gconfig
```

**Opens**: `~/.config/gcop/config.toml` in your `$EDITOR`

**When to use**: Quick access to edit your gcop-rs settings (API keys, models, prompts, etc.).

---

#### `git ghelp` - Show Help

Display gcop-rs help information.

**Command**: `gcop-rs --help`

**Usage**:
```bash
git ghelp
```

---

#### `git cop` - Main Entry Point

Direct access to gcop-rs command.

**Command**: `gcop-rs`

**Usage**:
```bash
git cop commit
git cop review changes
git cop --version
```

**When to use**: When you prefer the `git cop` prefix over `gcop-rs`.

---

#### `git gcommit` - Full Command Alias

Alternative to `git c` with a more descriptive name.

**Command**: `gcop-rs commit`

**Usage**:
```bash
git gcommit
```

**When to use**: If you prefer more explicit command names.

## Management

### Listing Aliases

See all available aliases and their installation status:

```bash
gcop-rs alias --list
```

Output shows:
- ✓ **Installed**: Alias is configured and ready
- ⚠ **Conflicts**: Alias name already used by another command
- **Not installed**: Alias is not configured

### Updating Aliases

Reinstall all aliases (useful after updates):

```bash
gcop-rs alias --force
```

This will overwrite any conflicting aliases.

### Removing Aliases

Remove all gcop-rs aliases:

```bash
# Preview what will be removed
gcop-rs alias --remove

# Actually remove (requires --force)
gcop-rs alias --remove --force
```

**⚠️ Warning**: This removes all gcop-rs aliases from your global git config.

## Advanced Usage

### Combining Aliases

You can chain aliases with other git commands:

```bash
# Create a new branch, commit, and push
git checkout -b feature/auth
git acp

# Review, commit, and push
git r changes && git acp

# Undo, edit, and recommit
git undo && vim src/auth.rs && git c
```

### Custom Workflows

Create your own aliases that build on gcop-rs:

```bash
# Add to your shell rc file (~/.bashrc, ~/.zshrc)
alias gac="git ac"          # Even shorter add-commit
alias gacp="git acp"        # Even shorter add-commit-push
alias review="git r changes"  # Plain 'review' command
```

## Troubleshooting

### Alias Already Exists

**Problem**: You see "conflicts with: existing-command"

**Solution**:
```bash
# Option 1: Force overwrite
gcop-rs alias --force

# Option 2: Remove the conflicting alias first
git config --global --unset alias.c
gcop-rs alias
```

### Command Not Found

**Problem**: `git c` says "command not found"

**Diagnosis**:
```bash
# Check if gcop-rs is in PATH
which gcop-rs

# Check if alias is installed
git config --global alias.c
```

**Solution**:
```bash
# If gcop-rs not in PATH
export PATH="$PATH:/usr/local/bin"

# If alias not installed
gcop-rs alias
```

### Alias Not Working After Update

**Problem**: Alias uses old command syntax

**Solution**:
```bash
# Reinstall all aliases
gcop-rs alias --force
```

## Best Practices

### Recommended Workflow

1. **Start with `git c`**: Use as your default commit command
2. **Use `git r changes`** before committing for quality checks
3. **Use `git ac`** for quick commits of all changes
4. **Reserve `git acp`** for confident, tested changes

### When to Use Full Commands

Use full `gcop-rs` commands instead of aliases when:
- Writing scripts (for clarity)
- Documenting workflows
- Using advanced options not available in aliases

### Safety Tips

1. **Review before `git acp`**: This pushes immediately, so use `git r changes` first
2. **Use `git undo`** freely: It's safe for local changes
3. **Be careful with `git pf`**: Only force push to your own branches
4. **Check status**: Run `git status` after `git undo` to see your staged changes

## Examples

### Daily Development Workflow

```bash
# Morning: Start new feature
git checkout -b feature/user-profile

# Work on it
vim src/profile.rs
vim src/routes.rs

# Review changes
git r changes

# Commit (all changes)
git ac

# More work
vim tests/profile_test.rs

# Quick commit and push
git acp
```

### Fixing a Mistake

```bash
# Oops, wrong commit message
git undo

# Fix and recommit
git c --yes
```

### Code Review Workflow

```bash
# Before creating PR
git r changes         # Check your changes

# If issues found, fix them
vim src/auth.rs

# Review again
git r changes

# Satisfied? Commit
git c
```

## See Also

- [Command Reference](commands.md) - Full documentation of all gcop-rs commands
- [Configuration Guide](configuration.md) - Customize gcop-rs behavior
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
