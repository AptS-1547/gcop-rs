# alias

Manage git aliases for gcop-rs.

**Synopsis**:
```bash
gcop-rs alias [OPTIONS]
```

**Options**:

| Option | Description |
|--------|-------------|
| *(none)* | Install all aliases (default action) |
| `--list`, `-l` | List all available aliases and their status |
| `--force`, `-f` | Force install, overwriting conflicts |
| `--remove`, `-r` | Remove aliases (requires `--force` to confirm) |

**Examples**:

## Install Aliases

```bash
# Install all aliases
gcop-rs alias

# Output:
[1/2] Installing git aliases...
  ✓  git c          → AI commit
  ✓  git r          → AI review
  ℹ  git p          → Push (already set)

✓ Installed 14 aliases
ℹ Skipped 1 alias (already exists or conflicts)
```

## List Aliases

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

## Force Install

```bash
# Overwrite conflicting aliases
gcop-rs alias --force
```

## Remove Aliases

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

## See Also

- [Git Aliases Guide](../aliases.md) - Full alias workflows and best practices
- [Command Overview](../commands.md) - All command entry points
