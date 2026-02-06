# 管理

## 列出别名

查看所有可用的别名及其安装状态：

```bash
gcop-rs alias --list
```

输出显示：
- ✓ **已安装**: 别名已配置并可用
- ⚠ **冲突**: 别名名称已被其他命令使用
- **未安装**: 别名未配置

## 更新别名

重新安装所有别名（更新后很有用）：

```bash
gcop-rs alias --force
```

这将覆盖任何冲突的别名。

## 删除别名

删除所有 gcop-rs 别名：

```bash
# 预览将删除什么
gcop-rs alias --remove

# 实际删除（需要 --force）
gcop-rs alias --remove --force
```

**⚠️ 警告**: 这将从全局 git 配置中删除所有 gcop-rs 别名。

## 高级用法

## 组合别名

你可以将别名与其他 git 命令链接：

```bash
# 创建新分支、提交并推送
git checkout -b feature/auth
git acp

# 审查、提交并推送
git r changes && git acp

# 撤销、编辑并重新提交
git undo && vim src/auth.rs && git c
```

## 自定义工作流

基于 gcop-rs 创建你自己的别名：

```bash
# 添加到你的 shell rc 文件 (~/.bashrc, ~/.zshrc)
alias gac="git ac"          # 更短的 add-commit
alias gacp="git acp"        # 更短的 add-commit-push
alias review="git r changes"  # 简单的 'review' 命令
```

## 故障排除

## 别名已存在

**问题**: 你看到 "冲突: existing-command"

**解决方案**:
```bash
# 方案 1: 强制覆盖
gcop-rs alias --force

# 方案 2: 先删除冲突的别名
git config --global --unset alias.c
gcop-rs alias
```

## 命令未找到

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

## 更新后别名不工作

**问题**: 别名使用旧的命令语法

**解决方案**:
```bash
# 重新安装所有别名
gcop-rs alias --force
```

## 参考

- [别名总览](../aliases.md)
- [alias 命令详解](../commands/alias.md)
- [故障排除总览](../troubleshooting.md)
