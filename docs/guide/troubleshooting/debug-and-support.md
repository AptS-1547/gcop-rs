# Debug Mode

For any issue, enable verbose mode to get detailed information:

```bash
gcop-rs -v commit
gcop-rs -v review changes
```

This shows:
- Configuration loading
- API requests and responses
- Commit prompts (system + user) when running `gcop-rs -v commit`
- Response parsing

> **Security Notice**: Verbose mode (`-v` or `RUST_LOG=debug`) logs full API requests and responses, which may include:
> - Your code diffs and changes
> - Partial API keys in error messages
> - Generated commit messages
>
> Do not share verbose logs publicly or commit them to version control.

## Getting Help

If you encounter an issue not listed here:

1. Run with `--verbose` and check the logs
2. Check the [Configuration Reference](../configuration.md)
3. Review the [Provider Setup Guide](../providers.md)
4. Open an issue on GitHub with:
   - Your config file (remove API keys!)
   - Command you ran
   - Error message
   - Output from `gcop-rs -v` (remove sensitive info)

