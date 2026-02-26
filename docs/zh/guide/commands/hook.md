# hook

管理由 gcop-rs 安装的仓库级 `prepare-commit-msg` hook。

**语法**:
```bash
gcop-rs hook <COMMAND>
```

**子命令**:

| 子命令 | 语法 | 说明 |
|-------|------|------|
| Install | `gcop-rs hook install` | 在当前仓库安装 `prepare-commit-msg` hook |
| Uninstall | `gcop-rs hook uninstall` | 卸载由 gcop-rs 安装的 `prepare-commit-msg` hook |

## `hook install`

在 `.git/hooks/prepare-commit-msg` 安装 hook 脚本。

**选项**:

| 选项 | 说明 |
|------|------|
| `--force`, `-f` | 覆盖已存在的非 gcop hook |

**行为说明**:
- 如果已安装 gcop-rs hook，则不会重复安装。
- 如果已有其他 hook 且未加 `--force`，会提示并跳过覆盖。
- 在类 Unix 系统上，安装后会设置可执行权限（`755`）。

## `hook uninstall`

仅在 hook 是由 gcop-rs 安装时，才会删除 `.git/hooks/prepare-commit-msg`。

**行为说明**:
- 如果 hook 文件不存在，会输出提示信息。
- 如果 hook 存在但不是 gcop-rs 安装的，会为安全起见跳过删除。

## 提交时的工作方式

安装完成后，执行 `git commit` 时会由 Git 内部触发 `gcop-rs hook run ...`。

hook 会在以下场景生成提交信息：
- 普通提交（`source` 为空或未知）：仅当存在已暂存变更时生成
- `--amend` 提交（`source=commit` 且 `sha` 非空）：基于被 amend 的目标提交 diff 生成；若同时存在已暂存变更，会合并两部分 diff

以下情况会跳过生成：
- `message`（例如 `git commit -m`）
- `merge`
- `squash`
- `commit` 且 `sha` 为空（例如 `git commit -C` / `-c`）

Hook 日志写入 **stderr**，避免污染常规 git 输出。

## 示例

```bash
# 在当前仓库安装 hook
gcop-rs hook install

# 覆盖已有 prepare-commit-msg hook
gcop-rs hook install --force

# 卸载由 gcop-rs 安装的 hook
gcop-rs hook uninstall
```

## 参考

- [命令总览](../commands.md)
- [commit](./commit.md)
- [配置指南](../configuration.md)
