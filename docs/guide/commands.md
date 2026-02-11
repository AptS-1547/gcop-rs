# Command Reference

This page is now a **navigation hub**. Detailed command docs are split into focused pages so each page stays shorter and easier to scan.

## Global Options

These options can be used with any command:

| Option | Description |
|--------|-------------|
| `--provider <NAME>`, `-p` | Override default LLM provider for LLM commands (`commit` / `review`) |
| `--verbose`, `-v` | Enable verbose logging (shows API requests and responses) |
| `--help`, `-h` | Show help information |
| `--version`, `-V` | Show version information |

**Example**:
```bash
gcop-rs --provider openai commit
gcop-rs -v review changes
```

## Command Pages

| Command | Use case | Detailed page |
|--------|----------|---------------|
| `init` | First-time setup wizard | [init](./commands/init.md) |
| `commit` | Generate commit message and commit | [commit](./commands/commit.md) |
| `review` | Review changes/commit/range/file | [review](./commands/review.md) |
| `config` | Edit and validate configuration | [config](./commands/config.md) |
| `alias` | Install/list/remove git aliases | [alias](./commands/alias.md) |
| `stats` | Repository commit statistics | [stats](./commands/stats.md) |
| `hook` | Install/uninstall `prepare-commit-msg` hook | [hook](./commands/hook.md) |

## Scripting and Environment

- [Automation, Exit Codes, Env Vars](./commands/automation.md)

## See Also

- [Git Aliases Guide](aliases.md) - Detailed guide to git aliases
- [Configuration Reference](configuration.md) - All configuration options
- [Provider Setup](providers.md) - Configure LLM providers
- [Provider Health Checks](provider-health.md) - How `config validate` checks providers
- [Troubleshooting](troubleshooting.md) - Common issues
