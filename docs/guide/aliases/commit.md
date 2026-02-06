# Commit Aliases

## `git c` - Quick Commit

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

## `git ac` - Add and Commit

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

## `git acp` - Add, Commit, and Push

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

## See Also

- [Aliases Overview](../aliases.md)
- [Review Aliases](./review.md)
- [Workflows & Best Practices](./workflows.md)
