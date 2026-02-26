# hook

Manage the repository `prepare-commit-msg` hook installed by gcop-rs.

**Synopsis**:
```bash
gcop-rs hook <COMMAND>
```

**Subcommands**:

| Subcommand | Syntax | Description |
|-----------|--------|-------------|
| Install | `gcop-rs hook install` | Install `prepare-commit-msg` hook in the current repository |
| Uninstall | `gcop-rs hook uninstall` | Remove gcop-rs installed `prepare-commit-msg` hook |

## `hook install`

Install a `prepare-commit-msg` hook script at `.git/hooks/prepare-commit-msg`.

**Options**:

| Option | Description |
|--------|-------------|
| `--force`, `-f` | Overwrite an existing non-gcop hook |

**Behavior**:
- If a gcop-rs hook is already installed, install is a no-op.
- If another hook already exists and `--force` is not set, gcop-rs warns and does not overwrite.
- On Unix-like systems, the installed hook is marked executable (`755`).

## `hook uninstall`

Remove `.git/hooks/prepare-commit-msg` only if it was installed by gcop-rs.

**Behavior**:
- If the hook file does not exist, gcop-rs prints an info message.
- If the hook exists but was not installed by gcop-rs, gcop-rs skips removal for safety.

## How It Works During Commit

After installation, `git commit` triggers `gcop-rs hook run ...` internally.

The hook generates a commit message in these cases:
- Normal commit (`source` is empty/unknown): only when staged changes exist
- Amend commit (`source=commit` with non-empty `sha`): uses the amend target commit diff; if staged changes also exist, both diffs are combined

The hook skips generation for:
- `message` (for example `git commit -m`)
- `merge`
- `squash`
- `commit` with empty `sha` (for example `git commit -C` / `-c`)

Hook logs are written to **stderr** so normal git output remains clean.

## Examples

```bash
# Install hook in current repository
gcop-rs hook install

# Overwrite existing prepare-commit-msg hook
gcop-rs hook install --force

# Remove hook installed by gcop-rs
gcop-rs hook uninstall
```

## See Also

- [Command Overview](../commands.md)
- [commit](./commit.md)
- [Configuration Guide](../configuration.md)
