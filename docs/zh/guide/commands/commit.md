# commit

生成 AI 驱动的提交信息并创建提交。

**语法**:
```bash
gcop-rs commit [OPTIONS] [FEEDBACK...]
```

**说明**:

分析暂存的变更，使用 AI 生成符合规范的提交信息，并在你批准后创建 git 提交。

**选项**:

| 选项 | 说明 |
|------|------|
| `--format <FORMAT>`, `-f` | 输出格式: `text`（默认）或 `json`（json 模式不会创建提交） |
| `--json` | `--format json` 的快捷方式 |
| `--no-edit`, `-n` | 跳过打开编辑器手动编辑 |
| `--yes`, `-y` | 跳过确认菜单并接受生成的信息 |
| `--dry-run`, `-d` | 仅生成并输出提交信息，不实际提交 |
| `--provider <NAME>`, `-p` | 使用特定的 provider（覆盖默认值） |

**反馈（可选）**:

你可以在选项后面追加一段自由文本，作为提交信息生成的额外指令。

```bash
# 推荐：使用引号
gcop-rs commit "用中文并保持简洁"

# 或不加引号（会被合并为一条指令）
gcop-rs commit 用中文 并 保持 简洁
```

> **注意**：在 JSON 模式（`--json` / `--format json`）下，gcop-rs 会以非交互方式运行，且**不会创建提交**（只输出 JSON）。

**交互式操作**:

生成信息后，你会看到一个菜单：

1. **Accept（接受）** - 使用生成的信息并创建提交
2. **Edit（编辑）** - 打开 `$VISUAL` / `$EDITOR`（未设置时使用系统默认编辑器）手动修改信息（编辑后返回菜单）
3. **Retry（重试）** - 不带额外指令重新生成新信息
4. **Retry with feedback（带反馈重试）** - 提供重新生成的指令（如 "用中文"、"更简洁"、"更详细"）。反馈会累积，多次重试可逐步优化结果
5. **Quit（退出）** - 取消提交过程

**示例**:

```bash
# 基本用法
git add src/auth.rs
gcop-rs commit

# 跳过所有提示
git add .
gcop-rs commit --no-edit --yes

# 使用不同的 provider
gcop-rs commit --provider openai

# 详细模式（查看 API 调用）
gcop-rs -v commit

# JSON 输出用于自动化（不会创建提交）
gcop-rs commit --json > commit.json
```

**工作流**:

```bash
$ git add src/auth.rs src/middleware.rs
$ gcop-rs commit

[1/4] 正在分析暂存的变更...
2 个文件已更改，45 处插入(+)，12 处删除(-)

ℹ 生成的提交信息:
feat(auth): 实现 JWT 令牌验证

添加用于验证 JWT 令牌的中间件，包含适当的
错误处理和过期检查。

[3/4] 选择下一步操作...
选择下一步操作:
> 接受
  编辑
  重试
  带反馈重试
  退出

[已选择: 接受]

[4/4] 正在创建提交...
✓ 提交创建成功！
```

**提示**:
- 运行前只暂存你想包含在此提交中的变更
- 在 CI/CD 流水线中使用 `--yes` 跳过交互式提示
- 使用 `--json` / `--format json` 生成提交信息用于脚本集成（不创建提交）
- 如果信息没有捕捉到你的意图，尝试"带反馈重试"

**输出格式 (json)**:

```json
{
  "success": true,
  "data": {
    "message": "feat(auth): 实现 JWT 令牌验证",
    "diff_stats": {
      "files_changed": ["src/auth.rs", "src/middleware.rs"],
      "insertions": 45,
      "deletions": 12,
      "total_changes": 57
    },
    "committed": false
  }
}
```

## 参考

- [命令总览](../commands.md)
- [配置指南](../configuration.md)
- [LLM Providers](../providers.md)
