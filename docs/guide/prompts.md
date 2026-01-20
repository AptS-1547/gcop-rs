# Custom Prompts

gcop-rs lets you customize the instructions sent to the LLM for both commit message generation and code review.

## How Custom Prompts Work

gcop-rs uses a split prompt:

- **System prompt**: instructions for the model
- **User message**: the actual content to work on (diff/context)

Your `custom_prompt` overrides the **system prompt**. The diff/context is always included in the **user message**.

> **Important**: `custom_prompt` is treated as plain text instructions. There is **no** template/placeholder substitution. If you write `{diff}` in your custom prompt, it will be sent literally.

## Commit Prompts (`[commit].custom_prompt`)

- Your `custom_prompt` becomes the **system prompt** for commit generation.
- The **user message** always includes:
  - staged diff (`git diff --cached` equivalent)
  - context (changed files, insertions, deletions)
  - current branch name (if available)
  - accumulated feedback from “Retry with feedback” (if used)

**Example**:

```toml
[commit]
custom_prompt = """
Generate a concise conventional commit message in Chinese.

Requirements:
- First line: type(scope): summary (<= 50 chars)
- Output ONLY the commit message (no explanation)
"""
```

## Review Prompts (`[review].custom_prompt`)

- Your `custom_prompt` becomes the **base system prompt** for review.
- gcop-rs **always appends** a JSON output constraint (so it can parse the result).
- The **user message** always includes the diff (or file content when using `review file`).

**Example**:

```toml
[review]
custom_prompt = """
You are a senior code reviewer.

Focus on:
1. Correctness (bugs, edge cases)
2. Security issues
3. Performance regressions
4. Maintainability
"""
```

## Debugging

- `gcop-rs -v commit` prints the generated system prompt and user message before calling the provider.
- `gcop-rs -v review ...` enables debug logging, but does not print the full prompt text.

## Notes

- The review command expects the model to return valid JSON. gcop-rs can strip common Markdown fences like ```json, but it still requires valid JSON to parse successfully.

## See Also

- [Configuration Reference](configuration.md) - All config options
- [Provider Setup](providers.md) - Configure LLM providers
- [Troubleshooting](troubleshooting.md) - Common issues
