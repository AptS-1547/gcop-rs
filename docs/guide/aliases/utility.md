# Utility Aliases

## `git amend` - Amend Last Commit

Amend the last commit (equivalent to `git commit --amend`).

**Command**: `git commit --amend`

**Usage**:
```bash
git amend
```

## `git undo` - Undo Last Commit

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

## `git p` - Quick Push

Shorthand for `git push`.

**Command**: `git push`

**Usage**:
```bash
git p
```

**When to use**: When you want a shorter push command.

---

## `git pf` - Safer Force Push

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

## `git gconfig` - Edit Configuration

Open gcop-rs configuration in your default editor.

**Command**: `gcop-rs config edit`

**Usage**:
```bash
git gconfig
```

**Opens**: your platform-specific `gcop` config file in `$EDITOR`

**When to use**: Quick access to edit your gcop-rs settings (API keys, models, prompts, etc.).

---

## `git ghelp` - Show Help

Display gcop-rs help information.

**Command**: `gcop-rs --help`

**Usage**:
```bash
git ghelp
```

---

## `git cop` - Main Entry Point

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

## `git gcommit` - Full Command Alias

Alternative to `git c` with a more descriptive name.

**Command**: `gcop-rs commit`

**Usage**:
```bash
git gcommit
```

**When to use**: If you prefer more explicit command names.

## See Also

- [Aliases Overview](../aliases.md)
- [Operations](./operations.md)
- [Workflows & Best Practices](./workflows.md)
