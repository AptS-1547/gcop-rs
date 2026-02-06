# config

Manage gcop-rs configuration.

**Synopsis**:
```bash
gcop-rs config [SUBCOMMAND]
```

If no subcommand is provided, it defaults to `gcop-rs config edit`.

**Subcommands**:

## `config edit`

Open configuration file in your default editor with validation.

**Usage**:
```bash
gcop-rs config edit
```

**Opens**: Config file (platform-specific location) in `$VISUAL` / `$EDITOR` (platform default if not set)

**Validation**: After saving, the configuration is automatically validated (like `visudo`). If validation fails, you'll see a menu:

```
✗ Config validation failed: TOML parse error...

? What would you like to do?
> ✎ Re-edit the config file
  ↩ Keep original config
  ⚠ Ignore errors and save anyway (dangerous)
```

**Recovery**: Even if your config file is corrupted, `config edit` will still work, allowing you to fix it.

**When to use**: Modify API keys, models, or custom prompts.

> **Tip**: Always use `gcop-rs config edit` instead of editing the config file directly to benefit from automatic validation.

## `config validate`

Validate configuration and test provider connection.

**Usage**:
```bash
gcop-rs config validate
```

**Checks**:
- Loads and parses configuration (defaults + config file + `GCOP_*` env overrides)
- Lists configured providers
- Validates provider connections through the configured provider chain (`default_provider` + `fallback_providers`)
- Succeeds if at least one configured provider validates

**Example output**:
```
[1/2] Loading configuration...
✓ Configuration loaded successfully

Configured providers:
  • claude

[2/2] Testing provider connection...
✓ Provider 'claude' validated successfully
```

**When to use**:
- After editing configuration
- Troubleshooting connection issues
- Verifying API keys

## See Also

- [Provider Health Checks](../provider-health.md) - Validation flow and endpoint checks
- [Configuration Guide](../configuration.md) - Full configuration reference
- [LLM Providers](../providers.md) - Provider setup examples
