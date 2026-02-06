# Management

## Listing Aliases

See all available aliases and their installation status:

```bash
gcop-rs alias --list
```

Output shows:
- ✓ **Installed**: Alias is configured and ready
- ⚠ **Conflicts**: Alias name already used by another command
- **Not installed**: Alias is not configured

## Updating Aliases

Reinstall all aliases (useful after updates):

```bash
gcop-rs alias --force
```

This will overwrite any conflicting aliases.

## Removing Aliases

Remove all gcop-rs aliases:

```bash
# Preview what will be removed
gcop-rs alias --remove

# Actually remove (requires --force)
gcop-rs alias --remove --force
```

**⚠️ Warning**: This removes all gcop-rs aliases from your global git config.

## Advanced Usage

## Combining Aliases

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

## Custom Workflows

Create your own aliases that build on gcop-rs:

```bash
# Add to your shell rc file (~/.bashrc, ~/.zshrc)
alias gac="git ac"          # Even shorter add-commit
alias gacp="git acp"        # Even shorter add-commit-push
alias review="git r changes"  # Plain 'review' command
```

## Troubleshooting

## Alias Already Exists

**Problem**: You see "conflicts with: existing-command"

**Solution**:
```bash
# Option 1: Force overwrite
gcop-rs alias --force

# Option 2: Remove the conflicting alias first
git config --global --unset alias.c
gcop-rs alias
```

## Command Not Found

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

## Alias Not Working After Update

**Problem**: Alias uses old command syntax

**Solution**:
```bash
# Reinstall all aliases
gcop-rs alias --force
```

## See Also

- [Aliases Overview](../aliases.md)
- [Alias Command Reference](../commands/alias.md)
- [Troubleshooting Overview](../troubleshooting.md)
