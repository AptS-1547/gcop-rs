use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Datelike, Duration, IsoWeek, Local};
use serde::Serialize;

use super::format::OutputFormat;
use super::options::StatsOptions;
use crate::commands::json::{self, JsonOutput};
use crate::error::Result;
use crate::git::{CommitInfo, GitOperations, repository::GitRepository};
use crate::ui;

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
pub fn run(options: &StatsOptions<'_>, colored: bool) -> Result<()> {
    let result = run_internal(options, colored);
    if let Err(ref e) = result
        && options.format.is_json()
    {
        let _ = json::output_json_error::<RepoStats>(e);
    }
    result
}

fn run_internal(options: &StatsOptions<'_>, colored: bool) -> Result<()> {
    let repo = GitRepository::open(None)?;
    let skip_ui = options.format.is_machine_readable();
    let effective_colored = options.effective_colored(colored);

    if !skip_ui {
        ui::step("1/2", &rust_i18n::t!("stats.analyzing"), effective_colored);
    }
    let commits = repo.get_commit_history()?;

    if commits.is_empty() {
        if !skip_ui {
            ui::warning(&rust_i18n::t!("stats.no_commits"), effective_colored);
        }
        return Ok(());
    }

    if !skip_ui {
        ui::step(
            "2/2",
            &rust_i18n::t!("stats.calculating"),
            effective_colored,
        );
    }
    let stats = RepoStats::from_commits(&commits, options.author);

    // 输出
    match options.format {
        OutputFormat::Json => output_json(&stats)?,
        OutputFormat::Markdown => output_markdown(&stats, effective_colored),
        OutputFormat::Text => output_text(&stats, effective_colored),
    }

    Ok(())
}

/// 文本格式输出
fn output_text(stats: &RepoStats, colored: bool) {
    println!();
    println!("{}", ui::info(&rust_i18n::t!("stats.title"), colored));
    println!("{}", "=".repeat(40));

    // Overview
    println!();
    ui::step("", &rust_i18n::t!("stats.overview"), colored);
    println!(
        "  {}  {}",
        rust_i18n::t!("stats.total_commits"),
        stats.total_commits
    );
    println!(
        "  {}   {}",
        rust_i18n::t!("stats.contributors"),
        stats.total_authors
    );

    if let (Some(first), Some(last)) = (stats.first_commit_date, stats.last_commit_date) {
        let days = stats.days_span().unwrap_or(0);
        println!(
            "  {}      {} ~ {} ({} {})",
            rust_i18n::t!("stats.time_span"),
            first.format("%Y-%m-%d"),
            last.format("%Y-%m-%d"),
            days,
            rust_i18n::t!("stats.days")
        );
    }

    // Top Contributors
    if !stats.authors.is_empty() {
        println!();
        ui::step("", &rust_i18n::t!("stats.top_contributors"), colored);

        let top_n = stats.authors.iter().take(10);
        for (i, author) in top_n.enumerate() {
            let percentage = if stats.total_commits > 0 {
                (author.commits as f64 / stats.total_commits as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "  #{:<2} {} <{}>  {} {} ({:.1}%)",
                i + 1,
                author.name,
                author.email,
                author.commits,
                rust_i18n::t!("stats.commits"),
                percentage
            );
        }

        if stats.authors.len() > 10 {
            println!(
                "  {}",
                rust_i18n::t!("stats.and_more", count = stats.authors.len() - 10)
            );
        }
    }

    // Recent Activity
    if !stats.commits_by_week.is_empty() {
        println!();
        ui::step("", &rust_i18n::t!("stats.recent_activity"), colored);

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
    println!("{}\n", rust_i18n::t!("stats.md_title"));

    println!("{}\n", rust_i18n::t!("stats.md_overview"));
    println!(
        "| {} | {} |",
        rust_i18n::t!("stats.md_metric"),
        rust_i18n::t!("stats.md_value")
    );
    println!("|--------|-------|");
    println!(
        "| {} | {} |",
        rust_i18n::t!("stats.md_total_commits"),
        stats.total_commits
    );
    println!(
        "| {} | {} |",
        rust_i18n::t!("stats.md_contributors"),
        stats.total_authors
    );

    if let (Some(first), Some(last)) = (stats.first_commit_date, stats.last_commit_date) {
        let days = stats.days_span().unwrap_or(0);
        println!(
            "| {} | {} ~ {} ({} {}) |",
            rust_i18n::t!("stats.md_time_span"),
            first.format("%Y-%m-%d"),
            last.format("%Y-%m-%d"),
            days,
            rust_i18n::t!("stats.days")
        );
    }

    if !stats.authors.is_empty() {
        println!("\n{}\n", rust_i18n::t!("stats.md_top_contributors"));
        println!(
            "| {} | {} | {} | {} | {} |",
            rust_i18n::t!("stats.md_rank"),
            rust_i18n::t!("stats.md_name"),
            rust_i18n::t!("stats.md_email"),
            rust_i18n::t!("stats.md_commits"),
            rust_i18n::t!("stats.md_percent")
        );
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
        println!("\n{}\n", rust_i18n::t!("stats.md_recent"));
        println!(
            "| {} | {} |",
            rust_i18n::t!("stats.md_week"),
            rust_i18n::t!("stats.md_commits_col")
        );
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
    let output = JsonOutput {
        success: true,
        data: Some(stats.clone()),
        error: None,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
