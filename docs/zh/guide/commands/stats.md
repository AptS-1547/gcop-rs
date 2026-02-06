# stats

显示仓库提交统计。

**语法**:
```bash
gcop-rs stats [OPTIONS]
```

**说明**:

分析提交历史并显示统计信息，包括总提交数、贡献者、时间跨度和最近活动。

**选项**:

| 选项 | 说明 |
|------|------|
| `--format <FORMAT>`, `-f` | 输出格式: `text`（默认）、`json` 或 `markdown` |
| `--json` | `--format json` 的快捷方式 |
| `--author <NAME>` | 按作者名称或邮箱过滤统计 |

**示例**:

```bash
# 基本用法（文本格式）
gcop-rs stats

# 输出为 JSON 用于自动化
gcop-rs stats --format json
gcop-rs stats --json

# 输出为 Markdown 用于文档
gcop-rs stats --format markdown > STATS.md

# 按特定作者过滤
gcop-rs stats --author "john"
gcop-rs stats --author "john@example.com"
```

**输出格式 (text)**:

```
ℹ Repository Statistics
========================================

[] Overview
  Total commits:  156
  Contributors:   3
  Time span:      2024-06-15 ~ 2025-12-23 (192 days)

[] Top Contributors
  #1  AptS-1547 <esaps@esaps.net>  142 commits (91.0%)
  #2  bot <noreply@github.com>      8 commits  (5.1%)
  #3  contributor <x@y.com>         6 commits  (3.8%)

[] Recent Activity (last 4 weeks)
  2025-W52: ████████████           12
  2025-W51: ████████████████████   20
  2025-W50: ██████                  6
  2025-W49: ████████████████       16
```

**输出格式 (json)**:

```json
{
  "success": true,
  "data": {
    "total_commits": 156,
    "total_authors": 3,
    "first_commit_date": "2024-06-15T10:30:00+08:00",
    "last_commit_date": "2025-12-23T15:43:34+08:00",
    "authors": [
      {"name": "AptS-1547", "email": "esaps@esaps.net", "commits": 142},
      {"name": "bot", "email": "noreply@github.com", "commits": 8}
    ],
    "commits_by_week": {
      "2025-W49": 16,
      "2025-W50": 6,
      "2025-W51": 20,
      "2025-W52": 12
    }
  }
}
```

**提示**:
- 使用 `--format json` 集成到 CI/CD 或脚本
- 使用 `--author` 查看单个贡献者的统计
- ASCII 柱状图显示各周的相对活跃度

## 参考

- [命令总览](../commands.md)
