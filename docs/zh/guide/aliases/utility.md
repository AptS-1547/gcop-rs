# 实用别名

## `git amend` - 修改最后一次提交

修改最后一次提交（等同于 `git commit --amend`）。

**命令**: `git commit --amend`

**用法**:
```bash
git amend
```

## `git undo` - 撤销最后一次提交

安全地撤销最后一次提交，同时保持变更在暂存区。

**命令**: `git reset --soft HEAD^`

**用法**:
```bash
# 刚刚提交但想修改？
git undo

# 你的变更仍在暂存区，编辑它们
vim src/auth.rs

# 用新信息重新提交
git c
```

**它做什么**:
- 将 HEAD 回退一个提交 (`HEAD^` = 前一个提交)
- **保持变更在暂存区**（可以直接提交）
- 保留工作目录

**何时使用**:
- 提交信息写错了
- 忘记包含某个文件
- 想要拆分提交
- 需要修改变更

**⚠️ 安全性**: 对本地提交是安全的。如果已经推送，请参阅下面的"撤销已推送的提交"。

**示例**:
```bash
$ git log --oneline
abc123 feat: add auth (当前 HEAD)
def456 fix: typo

$ git undo

$ git log --oneline
def456 fix: typo (当前 HEAD)

$ git status
要提交的变更:
  modified:   src/auth.rs
  # 你的变更仍在暂存区！
```

---

## `git p` - 快速推送

`git push` 的简写。

**命令**: `git push`

**用法**:
```bash
git p
```

**何时使用**: 当你想要更短的推送命令时。

---

## `git pf` - 更安全的强制推送

使用 `--force-with-lease` 进行更安全的强制推送。

**命令**: `git push --force-with-lease`

**用法**:
```bash
# rebase 后
git rebase -i HEAD~3
git pf
```

**为什么用 `--force-with-lease`**:
- 比 `--force` 更安全
- 仅在没有其他人推送到远程时才推送
- 防止意外覆盖他人的工作

**何时使用**:
- rebase 后
- 修改提交后
- 需要重写历史时

**⚠️ 警告**: 只对你拥有的分支强制推送。永远不要对 `main` 或 `master` 强制推送！

---

## `git gconfig` - 编辑配置

在默认编辑器中打开 gcop-rs 配置。

**命令**: `gcop-rs config edit`

**用法**:
```bash
git gconfig
```

**打开**: 在你的 `$EDITOR` 中打开平台对应的 `gcop` 配置文件

**何时使用**: 快速访问编辑 gcop-rs 设置（API keys、模型、prompts 等）。

---

## `git ghelp` - 显示帮助

显示 gcop-rs 帮助信息。

**命令**: `gcop-rs --help`

**用法**:
```bash
git ghelp
```

---

## `git cop` - 主入口

直接访问 gcop-rs 命令。

**命令**: `gcop-rs`

**用法**:
```bash
git cop commit
git cop review changes
git cop --version
```

**何时使用**: 当你更喜欢 `git cop` 前缀而不是 `gcop-rs` 时。

---

## `git gcommit` - 完整命令别名

`git c` 的替代，使用更具描述性的名称。

**命令**: `gcop-rs commit`

**用法**:
```bash
git gcommit
```

**何时使用**: 如果你更喜欢更明确的命令名称。

## 参考

- [别名总览](../aliases.md)
- [管理与排障](./operations.md)
- [工作流与最佳实践](./workflows.md)
