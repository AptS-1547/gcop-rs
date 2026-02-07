# Overview

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

## Quick Install

```bash
# Install all aliases
gcop-rs alias

# Verify installation
gcop-rs alias --list
```

## During Initial Setup

```bash
# The init command will prompt you
gcop-rs init
```

When prompted "Install git aliases?", select `Yes` to install all aliases automatically.

## Verification

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
  git r          → AI review of unstaged working tree changes [✓ installed]
  ...
```

## See Also

- [Aliases Overview](../aliases.md)
- [Commit Aliases](./commit.md)
- [Utility Aliases](./utility.md)
