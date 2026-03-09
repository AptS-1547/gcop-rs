# config

Manage gcop-rs configuration.

**Synopsis**:
```bash
gcop-rs config [SUBCOMMAND]
```

If no subcommand is provided, it defaults to `gcop-rs config edit`.

**Subcommands**:

## `config edit`

Open the user-level configuration file in your default editor with syntax/schema checks.

**Usage**:
```bash
gcop-rs config edit
```

**Opens**: User-level config file (platform-specific location) in `$VISUAL` / `$EDITOR` (platform default if not set)

**Validation**: After saving, gcop-rs parses the edited file and deserializes it into the config schema. It does not test provider connectivity or merged file/env/CI precedence. If parsing/deserialization fails, you'll see a menu:

```
✗ Config validation failed: TOML parse error...

? What would you like to do?
> ✎ Re-edit the config file
  ↩ Keep original config
  ⚠ Ignore errors and save anyway (dangerous)
```

**Recovery**: Even if your config file is corrupted, `config edit` will still work, allowing you to fix it.

**When to use**: Modify API keys, models, or custom prompts.

> **Tip**: Use `gcop-rs config edit` for safe syntax/schema checks, and `gcop-rs config validate` when you want full merged-config plus provider connectivity validation.

## `config validate`

Validate configuration and test provider connection.

**Usage**:
```bash
gcop-rs config validate
```

**Checks**:
- Loads and parses effective configuration (defaults + user config + optional project config + `GCOP__*` overrides + optional CI overrides)
- Lists configured providers (as loaded from config)
- Builds provider chain from `default_provider` + `fallback_providers` (providers that fail to instantiate are skipped)
- Validates provider connections through the instantiated provider chain
- Succeeds if at least one instantiated provider validates

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
