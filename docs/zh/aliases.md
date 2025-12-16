# Git 别名

gcop-rs git 别名完整指南 - 简化日常 Git 工作流程的便捷快捷方式。

## 概述

gcop-rs 提供 11 个精心设计的 git 别名，简化常见任务：

| 别名 | 命令 | 说明 |
|------|------|------|
| `git c` | `gcop-rs commit` | 快速 AI 提交 |
| `git r` | `gcop-rs review` | AI 审查变更 |
| `git ac` | `git add -A && gcop-rs commit` | 添加所有并提交 |
| `git acp` | `git add -A && gcop-rs commit && git push` | 添加、提交并推送 |
| `git cop` | `gcop-rs` | gcop-rs 主入口 |
| `git gcommit` | `gcop-rs commit` | 完整命令别名 |
| `git ghelp` | `gcop-rs --help` | 显示帮助 |
| `git gconfig` | `gcop-rs config edit` | 编辑配置 |
| `git p` | `git push` | 快速推送 |
| `git pf` | `git push --force-with-lease` | 更安全的强制推送 |
| `git undo` | `git reset --soft HEAD^` | 撤销最后一次提交 |

## 安装

### 快速安装

```bash
# 安装所有别名
gcop-rs alias

# 验证安装
gcop-rs alias --list
```

### 在初始化时安装

```bash
# init 命令会提示你
gcop-rs init
```

当提示"安装 git 别名？"时，选择 `是` 自动安装所有别名。

### 验证

检查已安装的别名：

```bash
gcop-rs alias --list
```

输出：
```
ℹ 可用的 git 别名:

  git cop        → gcop-rs 主入口                           [✓ 已安装]
  git gcommit    → AI 生成提交信息并提交                    [✓ 已安装]
  git c          → 'git gcommit' 的简写                     [✓ 已安装]
  git r          → AI 审查未提交的变更                      [✓ 已安装]
  ...
```

## 可用别名

### 提交相关别名

#### `git c` - 快速提交

创建 AI 提交信息的最快方式。

**命令**: `gcop-rs commit`

**用法**:
```bash
# 暂存你的变更
git add src/auth.rs

# 生成并提交
git c

# 或使用选项
git c --no-edit    # 跳过编辑器
git c --yes        # 跳过确认菜单
```

**何时使用**: 作为主要的提交命令。用它代替 `git commit` 来获取 AI 生成的提交信息。

---

#### `git ac` - 添加并提交

一步完成添加所有变更并提交。

**命令**: `git add -A && gcop-rs commit`

**用法**:
```bash
# 修改了多个文件？
git ac
```

**等同于**:
```bash
git add -A
git c
```

**何时使用**: 当你想提交所有变更而不想手动暂存时。

---

#### `git acp` - 添加、提交并推送

完整工作流：添加所有变更、AI 提交并推送到远程。

**命令**: `git add -A && gcop-rs commit && git push`

**用法**:
```bash
# 完成一个功能并推送
git acp
```

**等同于**:
```bash
git add -A
git c
git push
```

**何时使用**: 快速迭代时，确定要立即推送的情况。

**⚠️ 注意**: 仅在确定要推送时使用。只有前面的命令成功才会执行提交和推送。

---

### 审查相关别名

#### `git r` - 审查变更

对未提交的变更进行 AI 代码审查。

**命令**: `gcop-rs review`

**用法**:
```bash
# 提交前审查变更
git r

# 使用不同格式审查
git r --format json
git r --format markdown
```

**审查内容**: 工作目录中所有未提交的变更（包括已暂存和未暂存）。

**何时使用**:
- 提交前发现潜在问题
- 快速代码质量检查
- 获取改进建议

**示例工作流**:
```bash
# 做出变更
vim src/auth.rs

# 审查变更
git r

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

---

### 实用别名

#### `git undo` - 撤销最后一次提交

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

#### `git p` - 快速推送

`git push` 的简写。

**命令**: `git push`

**用法**:
```bash
git p
```

**何时使用**: 当你想要更短的推送命令时。

---

#### `git pf` - 更安全的强制推送

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

#### `git gconfig` - 编辑配置

在默认编辑器中打开 gcop-rs 配置。

**命令**: `gcop-rs config edit`

**用法**:
```bash
git gconfig
```

**打开**: 在你的 `$EDITOR` 中打开 `~/.config/gcop/config.toml`

**何时使用**: 快速访问编辑 gcop-rs 设置（API keys、模型、prompts 等）。

---

#### `git ghelp` - 显示帮助

显示 gcop-rs 帮助信息。

**命令**: `gcop-rs --help`

**用法**:
```bash
git ghelp
```

---

#### `git cop` - 主入口

直接访问 gcop-rs 命令。

**命令**: `gcop-rs`

**用法**:
```bash
git cop commit
git cop review
git cop --version
```

**何时使用**: 当你更喜欢 `git cop` 前缀而不是 `gcop-rs` 时。

---

#### `git gcommit` - 完整命令别名

`git c` 的替代，使用更具描述性的名称。

**命令**: `gcop-rs commit`

**用法**:
```bash
git gcommit
```

**何时使用**: 如果你更喜欢更明确的命令名称。

## 管理

### 列出别名

查看所有可用的别名及其安装状态：

```bash
gcop-rs alias --list
```

输出显示：
- ✓ **已安装**: 别名已配置并可用
- ⚠ **冲突**: 别名名称已被其他命令使用
- **未安装**: 别名未配置

### 更新别名

重新安装所有别名（更新后很有用）：

```bash
gcop-rs alias --force
```

这将覆盖任何冲突的别名。

### 删除别名

删除所有 gcop-rs 别名：

```bash
# 预览将删除什么
gcop-rs alias --remove

# 实际删除（需要 --force）
gcop-rs alias --remove --force
```

**⚠️ 警告**: 这将从全局 git 配置中删除所有 gcop-rs 别名。

## 高级用法

### 组合别名

你可以将别名与其他 git 命令链接：

```bash
# 创建新分支、提交并推送
git checkout -b feature/auth
git acp

# 审查、提交并推送
git r && git acp

# 撤销、编辑并重新提交
git undo && vim src/auth.rs && git c
```

### 自定义工作流

基于 gcop-rs 创建你自己的别名：

```bash
# 添加到你的 shell rc 文件 (~/.bashrc, ~/.zshrc)
alias gac="git ac"          # 更短的 add-commit
alias gacp="git acp"        # 更短的 add-commit-push
alias review="git r"        # 简单的 'review' 命令
```

## 故障排除

### 别名已存在

**问题**: 你看到 "冲突: existing-command"

**解决方案**:
```bash
# 方案 1: 强制覆盖
gcop-rs alias --force

# 方案 2: 先删除冲突的别名
git config --global --unset alias.c
gcop-rs alias
```

### 命令未找到

**问题**: `git c` 提示 "command not found"

**诊断**:
```bash
# 检查 gcop-rs 是否在 PATH 中
which gcop-rs

# 检查别名是否已安装
git config --global alias.c
```

**解决方案**:
```bash
# 如果 gcop-rs 不在 PATH 中
export PATH="$PATH:/usr/local/bin"

# 如果别名未安装
gcop-rs alias
```

### 更新后别名不工作

**问题**: 别名使用旧的命令语法

**解决方案**:
```bash
# 重新安装所有别名
gcop-rs alias --force
```

## 最佳实践

### 推荐工作流

1. **从 `git c` 开始**: 将其作为默认提交命令
2. **提交前使用 `git r`** 进行质量检查
3. **使用 `git ac`** 快速提交所有变更
4. **保留 `git acp`** 用于经过测试的、确定的变更

### 何时使用完整命令

在以下情况使用完整的 `gcop-rs` 命令而不是别名：
- 编写脚本时（为了清晰）
- 记录工作流程时
- 使用别名中不可用的高级选项时

### 安全提示

1. **`git acp` 前先审查**: 这会立即推送，所以先用 `git r` 检查
2. **自由使用 `git undo`**: 对本地变更是安全的
3. **小心使用 `git pf`**: 只对你自己的分支强制推送
4. **检查状态**: `git undo` 后运行 `git status` 查看暂存的变更

## 示例

### 日常开发工作流

```bash
# 早上：开始新功能
git checkout -b feature/user-profile

# 工作
vim src/profile.rs
vim src/routes.rs

# 审查变更
git r

# 提交（所有变更）
git ac

# 继续工作
vim tests/profile_test.rs

# 快速提交并推送
git acp
```

### 修复错误

```bash
# 糟糕，提交信息写错了
git undo

# 修复并重新提交
git c --yes
```

### 代码审查工作流

```bash
# 创建 PR 前
git r                 # 检查你的变更

# 如果发现问题，修复它们
vim src/auth.rs

# 再次审查
git r

# 满意？提交
git c
```

## 参考

- [命令参考](commands.md) - 所有 gcop-rs 命令的完整文档
- [配置指南](configuration.md) - 自定义 gcop-rs 行为
- [故障排除](troubleshooting.md) - 常见问题和解决方案
