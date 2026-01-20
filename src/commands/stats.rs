use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Datelike, Duration, IsoWeek, Local};
use serde::Serialize;

use crate::commands::json::ErrorJson;
use crate::error::{GcopError, Result};
use crate::git::{CommitInfo, GitOperations, repository::GitRepository};
use crate::ui;

/// JSON 输出格式（统一结构）
#[derive(Debug, Serialize)]
pub struct StatsJsonOutput {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<RepoStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorJson>,
}

/// 作者统计
#[derive(Debug, Clone, Serialize)]
pub struct AuthorStats {
    pub name: String,
    pub email: String,
    pub commits: usize,
}

/// 仓库统计
#[derive(Debug, Clone, Serialize)]
pub struct RepoStats {
    pub total_commits: usize,
    pub total_authors: usize,
    pub first_commit_date: Option<DateTime<Local>>,
    pub last_commit_date: Option<DateTime<Local>>,
    pub authors: Vec<AuthorStats>,
    pub commits_by_week: BTreeMap<String, usize>,
}

impl RepoStats {
    /// 从 commit 历史计算统计数据
    pub fn from_commits(commits: &[CommitInfo], author_filter: Option<&str>) -> Self {
        // 过滤 commits
        let filtered: Vec<&CommitInfo> = if let Some(filter) = author_filter {
            let filter_lower = filter.to_lowercase();
            commits
                .iter()
                .filter(|c| {
                    c.author_name.to_lowercase().contains(&filter_lower)
                        || c.author_email.to_lowercase().contains(&filter_lower)
                })
                .collect()
        } else {
            commits.iter().collect()
        };

        // 基础统计
        let total_commits = filtered.len();

        // 时间范围（commits 按时间降序，第一个是最新的）
        let last_commit_date = filtered.first().map(|c| c.timestamp);
        let first_commit_date = filtered.last().map(|c| c.timestamp);

        // 作者统计
        let mut author_map: HashMap<String, AuthorStats> = HashMap::new();
        for commit in &filtered {
            let key = format!("{} <{}>", commit.author_name, commit.author_email);
            author_map
                .entry(key)
                .or_insert_with(|| AuthorStats {
                    name: commit.author_name.clone(),
                    email: commit.author_email.clone(),
                    commits: 0,
                })
                .commits += 1;
        }

        let mut authors: Vec<AuthorStats> = author_map.into_values().collect();
        authors.sort_by(|a, b| b.commits.cmp(&a.commits));
        let total_authors = authors.len();

        // 最近 4 周的统计
        let now = Local::now();
        let four_weeks_ago = now - Duration::days(28);
        let mut commits_by_week: BTreeMap<String, usize> = BTreeMap::new();

        // 初始化最近 4 周
        for i in 0..4 {
            let week_start = now - Duration::days((i * 7) as i64);
            let week_key = format_week(&week_start);
            commits_by_week.insert(week_key, 0);
        }

        // 统计每周 commit 数
        for commit in &filtered {
            if commit.timestamp >= four_weeks_ago {
                let week_key = format_week(&commit.timestamp);
                *commits_by_week.entry(week_key).or_insert(0) += 1;
            }
        }

        Self {
            total_commits,
            total_authors,
            first_commit_date,
            last_commit_date,
            authors,
            commits_by_week,
        }
    }

    /// 计算时间跨度（天数）
    pub fn days_span(&self) -> Option<i64> {
        match (self.first_commit_date, self.last_commit_date) {
            (Some(first), Some(last)) => Some((last - first).num_days()),
            _ => None,
        }
    }
}

/// 格式化周标识 (e.g., "2025-W51")
fn format_week(dt: &DateTime<Local>) -> String {
    let week: IsoWeek = dt.iso_week();
    format!("{}-W{:02}", week.year(), week.week())
}

/// 生成 ASCII 柱状图
fn render_bar(count: usize, max_count: usize, max_width: usize) -> String {
    if max_count == 0 {
        return String::new();
    }
    let width = (count * max_width) / max_count;
    "█".repeat(width)
}

/// 运行 stats 命令
pub fn run(format: &str, author: Option<&str>, colored: bool) -> Result<()> {
    let repo = GitRepository::open(None)?;
    let is_json = format == "json";

    if !is_json {
        ui::step("1/2", "Analyzing commit history...", colored);
    }
    let commits = repo.get_commit_history()?;

    if commits.is_empty() {
        if !is_json {
            ui::warning("No commits found in this repository.", colored);
        }
        return Ok(());
    }

    if !is_json {
        ui::step("2/2", "Calculating statistics...", colored);
    }
    let stats = RepoStats::from_commits(&commits, author);

    // 输出
    match format {
        "json" => output_json(&stats)?,
        "markdown" | "md" => output_markdown(&stats, colored),
        _ => output_text(&stats, colored),
    }

    Ok(())
}

/// 文本格式输出
fn output_text(stats: &RepoStats, colored: bool) {
    println!();
    println!("{}", ui::info("Repository Statistics", colored));
    println!("{}", "=".repeat(40));

    // Overview
    println!();
    ui::step("", "Overview", colored);
    println!("  Total commits:  {}", stats.total_commits);
    println!("  Contributors:   {}", stats.total_authors);

    if let (Some(first), Some(last)) = (stats.first_commit_date, stats.last_commit_date) {
        let days = stats.days_span().unwrap_or(0);
        println!(
            "  Time span:      {} ~ {} ({} days)",
            first.format("%Y-%m-%d"),
            last.format("%Y-%m-%d"),
            days
        );
    }

    // Top Contributors
    if !stats.authors.is_empty() {
        println!();
        ui::step("", "Top Contributors", colored);

        let top_n = stats.authors.iter().take(10);
        for (i, author) in top_n.enumerate() {
            let percentage = if stats.total_commits > 0 {
                (author.commits as f64 / stats.total_commits as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "  #{:<2} {} <{}>  {} commits ({:.1}%)",
                i + 1,
                author.name,
                author.email,
                author.commits,
                percentage
            );
        }

        if stats.authors.len() > 10 {
            println!("  ... and {} more", stats.authors.len() - 10);
        }
    }

    // Recent Activity
    if !stats.commits_by_week.is_empty() {
        println!();
        ui::step("", "Recent Activity (last 4 weeks)", colored);

        let max_count = *stats.commits_by_week.values().max().unwrap_or(&0);

        // 按周倒序显示
        let mut weeks: Vec<_> = stats.commits_by_week.iter().collect();
        weeks.sort_by(|a, b| b.0.cmp(a.0));

        for (week, count) in weeks {
            let bar = render_bar(*count, max_count, 20);
            println!("  {}: {:20} {}", week, bar, count);
        }
    }

    println!();
}

/// Markdown 格式输出
fn output_markdown(stats: &RepoStats, _colored: bool) {
    println!("# Repository Statistics\n");

    println!("## Overview\n");
    println!("| Metric | Value |");
    println!("|--------|-------|");
    println!("| Total commits | {} |", stats.total_commits);
    println!("| Contributors | {} |", stats.total_authors);

    if let (Some(first), Some(last)) = (stats.first_commit_date, stats.last_commit_date) {
        let days = stats.days_span().unwrap_or(0);
        println!(
            "| Time span | {} ~ {} ({} days) |",
            first.format("%Y-%m-%d"),
            last.format("%Y-%m-%d"),
            days
        );
    }

    if !stats.authors.is_empty() {
        println!("\n## Top Contributors\n");
        println!("| Rank | Name | Email | Commits | % |");
        println!("|------|------|-------|---------|---|");

        for (i, author) in stats.authors.iter().take(10).enumerate() {
            let percentage = if stats.total_commits > 0 {
                (author.commits as f64 / stats.total_commits as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "| {} | {} | {} | {} | {:.1}% |",
                i + 1,
                author.name,
                author.email,
                author.commits,
                percentage
            );
        }
    }

    if !stats.commits_by_week.is_empty() {
        println!("\n## Recent Activity\n");
        println!("| Week | Commits |");
        println!("|------|---------|");

        let mut weeks: Vec<_> = stats.commits_by_week.iter().collect();
        weeks.sort_by(|a, b| b.0.cmp(a.0));

        for (week, count) in weeks {
            println!("| {} | {} |", week, count);
        }
    }
}

/// JSON 格式输出
fn output_json(stats: &RepoStats) -> Result<()> {
    let output = StatsJsonOutput {
        success: true,
        data: Some(stats.clone()),
        error: None,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// JSON 格式错误输出
pub fn output_json_error(err: &GcopError) -> Result<()> {
    let output = StatsJsonOutput {
        success: false,
        data: None,
        error: Some(ErrorJson::from_error(err)),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
