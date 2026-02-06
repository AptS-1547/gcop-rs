# 审查相关别名

## `git r` - 审查变更

对变更进行 AI 代码审查。

**命令**: `gcop-rs review <TARGET>`

**用法**:
```bash
# 审查工作区未暂存变更（类似 `git diff`）
git r changes

# 审查其他目标
git r commit HEAD
git r range HEAD~3..HEAD
git r file src/auth.rs

# 使用不同格式审查
git r changes --format json
git r changes --format markdown
```

**审查内容**: `git r changes` 会审查未暂存的工作区变更（类似 `git diff`）。

**何时使用**:
- 提交前发现潜在问题
- 快速代码质量检查
- 获取改进建议

**示例工作流**:
```bash
# 做出变更
vim src/auth.rs

# 审查变更
git r changes

📝 总结:
添加了 JWT 令牌验证和适当的错误处理。

🔍 发现的问题:
  1. WARNING: 考虑为令牌验证添加速率限制

💡 建议:
  • 为边界情况添加单元测试
  • 记录令牌验证逻辑

# 解决问题后提交
git c
```

## 参考

- [别名总览](../aliases.md)
- [提交类别名](./commit.md)
- [故障排除：审查与 Git](../troubleshooting/review-and-git.md)
