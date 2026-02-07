# alias

管理 gcop-rs 的 git 别名。

**语法**:
```bash
gcop-rs alias [OPTIONS]
```

**选项**:

| 选项 | 说明 |
|------|------|
| *(无)* | 安装所有别名（默认操作） |
| `--list`, `-l` | 列出所有可用的别名及其状态 |
| `--force`, `-f` | 强制安装，覆盖冲突 |
| `--remove`, `-r` | 删除别名（需要 `--force` 确认） |

**示例**:

## 安装别名

```bash
# 安装所有 14 个别名
gcop-rs alias

# 输出:
[1/2] 正在安装 git 别名...
  ✓  git c          → AI 提交
  ✓  git r          → AI 审查
  ℹ  git p          → 推送 (已设置)

✓ 已安装 14 个别名
ℹ 已跳过 1 个别名（已存在或冲突）
```

## 列出别名

```bash
gcop-rs alias --list

# 输出:
ℹ 可用的 git 别名:

  git cop        → 主入口                                [✓ 已安装]
  git c          → AI 提交                               [✓ 已安装]
  git r          → AI 审查                               [  未安装]
  git p          → 推送                                  [⚠ 冲突: !my-push]
  ...
```

## 强制安装

```bash
# 覆盖冲突的别名
gcop-rs alias --force
```

## 删除别名

```bash
# 预览将删除什么
gcop-rs alias --remove

# 输出:
⚠ 这将删除所有 gcop 相关的 git 别名

ℹ 将删除的别名:
  - git c
  - git r
  - git ac
  ...

ℹ 使用 --force 确认:
  gcop-rs alias --remove --force

# 实际删除
gcop-rs alias --remove --force
```

**何时使用**:
- 安装后：安装别名以获得便利
- gcop-rs 更新后：用 `--force` 重新安装
- 卸载时：用 `--remove --force` 删除

## 参考

- [Git 别名指南](../aliases.md) - 完整别名工作流与最佳实践
- [命令总览](../commands.md) - 所有命令入口
