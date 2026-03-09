# stats

显示仓库提交统计。

**语法**:
```bash
gcop-rs stats [OPTIONS]
```

**说明**:

分析提交历史并输出：
- 概览（总提交数、贡献者、时间跨度）
- 主要贡献者
- 最近 4 周活动
- 最近 30 天提交热力图
- 当前连续提交天数与最长连续提交天数
- 可选的按作者统计的代码行级贡献数据（`--contrib`，会跳过 merge commit）

**选项**:

| 选项 | 说明 |
|------|------|
| `--format <FORMAT>`, `-f` | 输出格式: `text`（默认）、`json` 或 `markdown` |
| `--json` | `--format json` 的快捷方式 |
| `--author <NAME>` | 按作者名称或邮箱过滤全部统计结果 |
| `--contrib` | 额外输出按作者汇总的行级贡献统计 |

**示例**:

```bash
# 基本用法（文本格式）
gcop-rs stats

# 输出为 JSON 用于自动化
gcop-rs stats --format json
gcop-rs stats --json

# 输出为 Markdown 用于报告
gcop-rs stats --format markdown > STATS.md

# 按特定作者过滤
gcop-rs stats --author "john"
gcop-rs stats --author "john@example.com"

# 包含行级贡献统计
gcop-rs stats --contrib
gcop-rs stats --author "john" --contrib
```

> **注意**：`json`/`markdown` 格式为非交互输出，不会显示步骤提示或转圈 UI 行。

> **注意**：`--contrib` 会按 commit 计算插入/删除行数，并跳过 merge commit。

**输出格式 (text)**:

```
ℹ 仓库统计
────────────────────────────────────────

  ▸ 概览
    总提交数：       170
    贡献者：         6
    时间跨度：       2025-12-16 ~ 2026-02-12 (57 天)

  ▸ 主要贡献者
    #1  AptS-1547 <esaps@esaps.net>  133 次提交 (78.2%)
    #2  AptS-1738 <apts-1738@esaps.net>  32 次提交 (18.8%)

  ▸ 近期活动(最近 4 周)
    2026-W07: █                    4
    2026-W06: ████████████████████ 45
    2026-W05:                      0
    2026-W04: ██████               14

  ▸ 提交活动(最近 30 天)
    01/14 ▂······▄▂·············▂▂▄█···▂ 02/12  peak: 31

  ▸ 连续提交
    当前连续：       1 天
    最长连续：       9 天
```

**输出格式 (json)**:

```json
{
  "success": true,
  "data": {
    "total_commits": 170,
    "total_authors": 6,
    "first_commit_date": "2025-12-16T14:38:08+08:00",
    "last_commit_date": "2026-02-12T06:03:30+08:00",
    "authors": [
      {"name": "AptS-1547", "email": "esaps@esaps.net", "commits": 133},
      {"name": "AptS-1738", "email": "apts-1738@esaps.net", "commits": 32}
    ],
    "commits_by_week": {
      "2026-W04": 14,
      "2026-W05": 0,
      "2026-W06": 45,
      "2026-W07": 4
    },
    "commits_by_day": {
      "2026-02-08": 31,
      "2026-02-12": 4
    },
    "current_streak": 1,
    "longest_streak": 9
  }
}
```

**输出格式 (json + contrib)**:

```json
{
  "success": true,
  "data": {
    "total_commits": 170,
    "contrib": {
      "total_insertions": 4200,
      "total_deletions": 1800,
      "total_lines": 6000,
      "merge_commits_skipped": 3,
      "authors": [
        {
          "name": "AptS-1547",
          "email": "esaps@esaps.net",
          "insertions": 2800,
          "deletions": 900,
          "total": 3700,
          "percentage": 61.67
        }
      ]
    }
  }
}
```

**提示**:
- 使用 `--format json` 集成到 CI/CD 或脚本
- 使用 `--author` 聚焦单个贡献者
- Markdown 输出会包含按天活动（仅展示非 0 天）

## 参考

- [命令总览](../commands.md)
