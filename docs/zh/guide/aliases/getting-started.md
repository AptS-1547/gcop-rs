# 概述

gcop-rs 提供 14 个精心设计的 git 别名，简化常见任务：

| 别名 | 命令 | 说明 |
|------|------|------|
| `git c` | `gcop-rs commit` | 快速 AI 提交 |
| `git r` | `gcop-rs review <TARGET>` | AI 审查变更 |
| `git s` | `gcop-rs stats` | 仓库统计 |
| `git ac` | `git add -A && gcop-rs commit` | 添加所有并提交 |
| `git cp` | `gcop-rs commit && git push` | 提交并推送 |
| `git acp` | `git add -A && gcop-rs commit && git push` | 添加、提交并推送 |
| `git cop` | `gcop-rs` | gcop-rs 主入口 |
| `git gcommit` | `gcop-rs commit` | 完整命令别名 |
| `git ghelp` | `gcop-rs --help` | 显示帮助 |
| `git gconfig` | `gcop-rs config edit` | 编辑配置 |
| `git p` | `git push` | 快速推送 |
| `git pf` | `git push --force-with-lease` | 更安全的强制推送 |
| `git amend` | `git commit --amend` | 修改最后一次提交 |
| `git undo` | `git reset --soft HEAD^` | 撤销最后一次提交 |

## 安装

## 快速安装

```bash
# 安装所有别名
gcop-rs alias

# 验证安装
gcop-rs alias --list
```

## 在初始化时安装

```bash
# init 命令会提示你
gcop-rs init
```

当提示"安装 git 别名？"时，选择 `是` 自动安装所有别名。

## 验证

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

## 参考

- [别名总览](../aliases.md)
- [提交类别名](./commit.md)
- [工具类别名](./utility.md)
