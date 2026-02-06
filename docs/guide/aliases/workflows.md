# Best Practices

## Recommended Workflow

1. **Start with `git c`**: Use as your default commit command
2. **Use `git r changes`** before committing for quality checks
3. **Use `git ac`** for quick commits of all changes
4. **Reserve `git acp`** for confident, tested changes

## When to Use Full Commands

Use full `gcop-rs` commands instead of aliases when:
- Writing scripts (for clarity)
- Documenting workflows
- Using advanced options not available in aliases

## Safety Tips

1. **Review before `git acp`**: This pushes immediately, so use `git r changes` first
2. **Use `git undo`** freely: It's safe for local changes
3. **Be careful with `git pf`**: Only force push to your own branches
4. **Check status**: Run `git status` after `git undo` to see your staged changes

## Examples

## Daily Development Workflow

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

## Fixing a Mistake

```bash
# Oops, wrong commit message
git undo

# Fix and recommit
git c --yes
```

## Code Review Workflow

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

- [Aliases Overview](../aliases.md)
- [Commit Aliases](./commit.md)
- [Review Aliases](./review.md)
