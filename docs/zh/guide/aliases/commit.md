# 提交相关别名

## `git c` - 快速提交

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

## `git ac` - 添加并提交

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

## `git acp` - 添加、提交并推送

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

## 参考

- [别名总览](../aliases.md)
- [审查类别名](./review.md)
- [工作流与最佳实践](./workflows.md)
