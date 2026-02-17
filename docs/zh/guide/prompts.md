# 自定义 Prompt

gcop-rs 允许你自定义发送给 LLM 的指令，用于生成提交信息和代码审查。

## 工作方式

gcop-rs 使用“拆分 prompt”的方式：

- **System prompt**：给模型的指令
- **User message**：实际要处理的内容（diff / 上下文）

diff/上下文始终会放在 **user message** 中。`custom_prompt` 的行为取决于模式：

- **普通 commit 模式**：`custom_prompt` 会替换基础 commit system prompt。
- **split commit 模式**（`commit --split` 或 `[commit].split = true`）：`custom_prompt` 会作为额外分组约束追加到内置规则后。
- **review 模式**：`custom_prompt` 作为 review system prompt 基础，并始终追加 JSON 输出约束。

> **重要**：`custom_prompt` 只是纯文本指令，不支持 `{diff}` 之类的占位符替换。写在里面会原样发送。

## Commit Prompt（`[commit].custom_prompt`）

- 在普通 commit 模式下，`custom_prompt` 会作为提交信息生成的 **system prompt**。
- **User message** 总是包含：
  - 已暂存的 diff（等价于 `git diff --cached`）
  - 上下文（修改文件列表、插入/删除行数）
  - 当前分支名（如果能获取到）
  - “带反馈重试”累积的反馈（如果使用过）

当启用 split commit 模式时，gcop-rs 会使用内置分组规则，并将你的 `custom_prompt` 作为附加约束追加。

**示例**：

```toml
[commit]
custom_prompt = """
请用中文生成简洁的 conventional commit 提交信息。

要求：
- 第一行：type(scope): 概要（<= 50 字符）
- 只输出提交信息，不要解释
"""
```

## Review Prompt（`[review].custom_prompt`）

- 你的 `custom_prompt` 会作为代码审查的 **system prompt** 基础。
- gcop-rs 会**始终追加** JSON 输出约束（用于解析结果）。
- **User message** 总是包含 diff（或在 `review file` 时包含文件内容）。

**示例**：

```toml
[review]
custom_prompt = """
你是资深代码审查者。

重点关注：
1. 正确性（bug、边界情况）
2. 安全问题
3. 性能退化
4. 可维护性
"""
```

## 调试

- `gcop-rs -v commit` 会在调用 provider 前打印 system prompt 和 user message。
- `gcop-rs -v review ...` 仅开启 debug 日志，不会打印完整 prompt 内容。

## 备注

- review 需要模型返回合法 JSON。gcop-rs 会尝试去掉常见的 Markdown code fence（如 ```json），但仍必须是可解析的 JSON。

## 参考

- [配置参考](configuration.md) - 所有配置选项
- [Provider 设置](providers.md) - 配置 LLM providers
- [故障排除](troubleshooting.md) - 常见问题
