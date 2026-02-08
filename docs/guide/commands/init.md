# init

Initialize gcop-rs configuration.

**Synopsis**:
```bash
gcop-rs init [OPTIONS]
```

**Description**:

By default, `gcop-rs init` runs an interactive user-level setup that guides you through:
1. Creating configuration directory (platform-specific location)
2. Copying example configuration file
3. Setting secure file permissions (Unix/Linux/macOS only)
4. Optionally installing git aliases

Use `--project` to initialize a repository-level config (`.gcop/config.toml`) for team-shared, non-secret settings.

**Options**:

| Option | Description |
|--------|-------------|
| `--force`, `-f` | Force overwrite existing config |
| `--project` | Initialize project-level `.gcop/config.toml` in the current git repository |

> **Note**: If no git repository is detected, `--project` falls back to the current directory and creates `./.gcop/config.toml`.

## User-Level Example (Linux)

```bash
$ gcop-rs init

✓ Created config directory: /home/user/.config/gcop
✓ Created config file: /home/user/.config/gcop/config.toml
✓ Set file permissions: 600

ℹ Next steps:
  1. Edit config file: gcop-rs config edit
  2. Set your API key for your preferred provider
  3. Test with: gcop-rs commit --help

Install git aliases? (Y/n): y

[1/2] Installing git aliases...
  ✓  git c          → AI commit
  ✓  git r          → AI review
  ...

✓ Installed 14 aliases
```

## Project-Level Example

```bash
$ gcop-rs init --project

✓ Created project config directory: /path/to/repo/.gcop
✓ Created project config file: /path/to/repo/.gcop/config.toml

ℹ Next steps:
  1. Edit .gcop/config.toml for team conventions (prompt/convention/review rules)
  2. Keep API keys in user config or environment variables (do not put secrets in project config)
```

**What it creates**:
- `gcop-rs init` (default): user config at platform-specific location (from `examples/config.toml.example`)
- `gcop-rs init --project`: repository config at `.gcop/config.toml` (from `examples/project-config.toml.example`)
- Git aliases in `~/.gitconfig` (only for default interactive mode, if you choose to install them)

**When to use**: First time setup or when reconfiguring from scratch.

## See Also

- [Command Overview](../commands.md)
- [Configuration Guide](../configuration.md)
