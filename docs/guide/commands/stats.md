# stats

Show repository commit statistics.

**Synopsis**:
```bash
gcop-rs stats [OPTIONS]
```

**Description**:

Analyzes commit history and displays statistics including total commits, contributors, time span, and recent activity.

**Options**:

| Option | Description |
|--------|-------------|
| `--format <FORMAT>`, `-f` | Output format: `text` (default), `json`, or `markdown` |
| `--json` | Shortcut for `--format json` |
| `--author <NAME>` | Filter statistics by author name or email |

**Examples**:

```bash
# Basic usage (text format)
gcop-rs stats

# Output as JSON for automation
gcop-rs stats --format json
gcop-rs stats --json

# Output as Markdown for documentation
gcop-rs stats --format markdown > STATS.md

# Filter by specific author
gcop-rs stats --author "john"
gcop-rs stats --author "john@example.com"
```

**Output Format (text)**:

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

**Output Format (json)**:

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

**Tips**:
- Use `--format json` for CI/CD integration or scripting
- Use `--author` to see individual contributor statistics
- The ASCII bar chart shows relative activity across weeks

## See Also

- [Command Overview](../commands.md)
