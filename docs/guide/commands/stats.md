# stats

Show repository commit statistics.

**Synopsis**:
```bash
gcop-rs stats [OPTIONS]
```

**Description**:

Analyzes commit history and reports:
- overview (total commits, contributors, time span)
- top contributors
- recent weekly activity (last 4 weeks)
- daily activity heatmap (last 30 days)
- current and longest commit streak

**Options**:

| Option | Description |
|--------|-------------|
| `--format <FORMAT>`, `-f` | Output format: `text` (default), `json`, or `markdown` |
| `--json` | Shortcut for `--format json` |
| `--author <NAME>` | Filter all statistics by author name or email |

**Examples**:

```bash
# Basic usage (text format)
gcop-rs stats

# Output as JSON for automation
gcop-rs stats --format json
gcop-rs stats --json

# Output as Markdown for reports
gcop-rs stats --format markdown > STATS.md

# Filter by specific author
gcop-rs stats --author "john"
gcop-rs stats --author "john@example.com"
```

> **Note**: In `json`/`markdown` formats, stats output is non-interactive (no step/spinner UI lines).

**Output Format (text)**:

```
ℹ Repository Statistics
────────────────────────────────────────

  ▸ Overview
    Total commits:    170
    Contributors:     6
    Time span:        2025-12-16 ~ 2026-02-12 (57 days)

  ▸ Top Contributors
    #1  AptS-1547 <esaps@esaps.net>  133 commits (78.2%)
    #2  AptS-1738 <apts-1738@esaps.net>  32 commits (18.8%)

  ▸ Recent Activity (last 4 weeks)
    2026-W07: █                    4
    2026-W06: ████████████████████ 45
    2026-W05:                      0
    2026-W04: ██████               14

  ▸ Commit Activity (last 30 days)
    01/14 ▂······▄▂·············▂▂▄█···▂ 02/12  peak: 31

  ▸ Streak
    Current streak:   1 days
    Longest streak:   9 days
```

**Output Format (json)**:

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

**Tips**:
- Use `--format json` for CI/CD integration or scripts
- Use `--author` to focus on one contributor
- Markdown output includes commit activity by day (non-zero days only)

## See Also

- [Command Overview](../commands.md)
