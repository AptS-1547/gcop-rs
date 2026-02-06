# init

Initialize gcop-rs configuration with an interactive wizard.

**Synopsis**:
```bash
gcop-rs init [OPTIONS]
```

**Description**:

Interactive setup that guides you through:
1. Creating configuration directory (platform-specific location)
2. Copying example configuration file
3. Setting secure file permissions (Unix/Linux/macOS only)
4. Optionally installing git aliases

**Options**:

| Option | Description |
|--------|-------------|
| `--force`, `-f` | Force overwrite existing config |

**Example** (Linux):
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

**What it creates**:
- Config file at platform-specific location (from `examples/config.toml.example`)
- Git aliases in `~/.gitconfig` (if you choose to install them)

**When to use**: First time setup or when reconfiguring from scratch.

## See Also

- [Command Overview](../commands.md)
- [Configuration Guide](../configuration.md)
