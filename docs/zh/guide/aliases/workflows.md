# 最佳实践

## 推荐工作流

1. **从 `git c` 开始**: 将其作为默认提交命令
2. **提交前使用 `git r changes`** 进行质量检查
3. **使用 `git ac`** 快速提交所有变更
4. **保留 `git acp`** 用于经过测试的、确定的变更

## 何时使用完整命令

在以下情况使用完整的 `gcop-rs` 命令而不是别名：
- 编写脚本时（为了清晰）
- 记录工作流程时
- 使用别名中不可用的高级选项时

## 安全提示

1. **`git acp` 前先审查**: 这会立即推送，所以先用 `git r changes` 检查
2. **自由使用 `git undo`**: 对本地变更是安全的
3. **小心使用 `git pf`**: 只对你自己的分支强制推送
4. **检查状态**: `git undo` 后运行 `git status` 查看暂存的变更

## 示例

## 日常开发工作流

```bash
# 早上：开始新功能
git checkout -b feature/user-profile

# 工作
vim src/profile.rs
vim src/routes.rs

# 审查变更
git r changes

# 提交（所有变更）
git ac

# 继续工作
vim tests/profile_test.rs

# 快速提交并推送
git acp
```

## 修复错误

```bash
# 糟糕，提交信息写错了
git undo

# 修复并重新提交
git c --yes
```

## 代码审查工作流

```bash
# 创建 PR 前
git r changes         # 检查你的变更

# 如果发现问题，修复它们
vim src/auth.rs

# 再次审查
git r changes

# 满意？提交
git c
```

## 参考

- [别名总览](../aliases.md)
- [提交类别名](./commit.md)
- [审查类别名](./review.md)
