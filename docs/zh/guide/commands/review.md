# review

对变更、提交或文件执行 AI 驱动的代码审查。

**语法**:
```bash
gcop-rs review [OPTIONS] <COMMAND>
```

**命令**:

| 命令 | 语法 | 说明 |
|------|------|------|
| 变更 | `gcop-rs review changes` | 审查工作区未暂存变更（类似 `git diff`） |
| 提交 | `gcop-rs review commit <HASH>` | 审查特定提交 |
| 范围 | `gcop-rs review range <RANGE>` | 审查提交范围（如 `HEAD~3..HEAD`） |
| 文件 | `gcop-rs review file <PATH>` | 审查单个文件（当前不支持目录） |

**选项**:

| 选项 | 说明 |
|------|------|
| `--format <FORMAT>`, `-f` | 输出格式: `text`（默认）、`json` 或 `markdown` |
| `--json` | `--format json` 的快捷方式 |
| `--provider <NAME>`, `-p` | 使用特定的 provider |

**示例**:

```bash
# 审查工作区变更（未暂存）
gcop-rs review changes

# 审查最后一次提交
gcop-rs review commit HEAD
gcop-rs review commit abc123

# 审查最近 3 次提交
gcop-rs review range HEAD~3..HEAD

# 审查单个文件
gcop-rs review file src/auth.rs

# 输出为 JSON 用于自动化
gcop-rs review changes --format json > review.json

# 输出为 markdown 用于文档
gcop-rs review changes --format markdown > REVIEW.md
```

> **注意**：当前 `review changes` 只会审查未暂存的变更（类似 `git diff`），不会包含已暂存的变更。
>
> **注意**：`review file <PATH>` 当前仅支持文件（不支持目录）。

**输出格式 (text)**:

```
ℹ 审查: 未提交的变更

📝 总结:
添加了 JWT 认证和适当的错误处理。
整体代码质量良好。

🔍 发现的问题:

  1. WARNING: 令牌刷新中缺少错误处理
     位置: src/auth.rs:45

  2. INFO: 考虑添加速率限制
     位置: src/middleware.rs:12

💡 建议:
  • 为边界情况添加单元测试
  • 记录令牌验证逻辑
  • 考虑将验证提取到单独的函数
```

**提示**:
- 提交前使用以尽早发现问题
- 使用 `--format json` 集成到 CI/CD
- 在配置中设置 `min_severity` 过滤噪音

## 参考

- [命令总览](../commands.md)
- [故障排除](../troubleshooting.md)
