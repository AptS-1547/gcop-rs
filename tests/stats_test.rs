/// stats.rs 测试
///
/// 测试 RepoStats::from_commits() 的计算逻辑：
/// - 基本统计（total_commits, total_authors）
/// - 时间范围计算（first_commit_date, last_commit_date, days_span）
/// - 作者统计（commits 排序）
/// - 周统计（commits_by_week）
/// - 作者过滤（author_filter）
/// - 边界情况（空仓库）
use chrono::{Duration, Local};
use gcop_rs::commands::stats::RepoStats;
use gcop_rs::git::CommitInfo;

/// 创建测试 commit
fn create_test_commit(
    author_name: &str,
    author_email: &str,
    days_ago: i64,
    message: &str,
) -> CommitInfo {
    CommitInfo {
        author_name: author_name.to_string(),
        author_email: author_email.to_string(),
        timestamp: Local::now() - Duration::days(days_ago),
        message: message.to_string(),
    }
}

// === 基本统计测试 ===

#[test]
fn test_repo_stats_empty_commits() {
    let commits: Vec<CommitInfo> = vec![];
    let stats = RepoStats::from_commits(&commits, None);

    assert_eq!(stats.total_commits, 0);
    assert_eq!(stats.total_authors, 0);
    assert!(stats.first_commit_date.is_none());
    assert!(stats.last_commit_date.is_none());
    assert_eq!(stats.authors.len(), 0);
    assert_eq!(stats.days_span(), None);
}

#[test]
fn test_repo_stats_single_commit() {
    let commits = vec![create_test_commit(
        "Alice",
        "alice@example.com",
        5,
        "fix: bug",
    )];

    let stats = RepoStats::from_commits(&commits, None);

    assert_eq!(stats.total_commits, 1);
    assert_eq!(stats.total_authors, 1);
    assert!(stats.first_commit_date.is_some());
    assert!(stats.last_commit_date.is_some());
    assert_eq!(stats.authors.len(), 1);
    assert_eq!(stats.authors[0].name, "Alice");
    assert_eq!(stats.authors[0].commits, 1);
    assert_eq!(stats.days_span(), Some(0)); // 同一天
}

#[test]
fn test_repo_stats_multiple_commits() {
    let commits = vec![
        create_test_commit("Alice", "alice@example.com", 1, "feat: add feature"), // 最新
        create_test_commit("Bob", "bob@example.com", 5, "fix: bug"),
        create_test_commit("Alice", "alice@example.com", 10, "docs: update"), // 最老
    ];

    let stats = RepoStats::from_commits(&commits, None);

    assert_eq!(stats.total_commits, 3);
    assert_eq!(stats.total_authors, 2);

    // 检查作者排序（按 commits 降序）
    assert_eq!(stats.authors.len(), 2);
    assert_eq!(stats.authors[0].name, "Alice"); // 2 commits
    assert_eq!(stats.authors[0].commits, 2);
    assert_eq!(stats.authors[1].name, "Bob"); // 1 commit
    assert_eq!(stats.authors[1].commits, 1);

    // 检查时间范围（允许 ±1 天误差）
    let days = stats.days_span().unwrap();
    assert!((8..=10).contains(&days), "Expected 8-10 days, got {}", days);
}

// === 作者过滤测试 ===

#[test]
fn test_repo_stats_author_filter_by_name() {
    let commits = vec![
        create_test_commit("Alice", "alice@example.com", 1, "feat: add feature"),
        create_test_commit("Bob", "bob@example.com", 2, "fix: bug"),
        create_test_commit("Alice", "alice@example.com", 3, "docs: update"),
    ];

    let stats = RepoStats::from_commits(&commits, Some("Alice"));

    assert_eq!(stats.total_commits, 2);
    assert_eq!(stats.total_authors, 1);
    assert_eq!(stats.authors[0].name, "Alice");
    assert_eq!(stats.authors[0].commits, 2);
}

#[test]
fn test_repo_stats_author_filter_by_email() {
    let commits = vec![
        create_test_commit("Alice", "alice@example.com", 1, "feat: add feature"),
        create_test_commit("Bob", "bob@example.com", 2, "fix: bug"),
    ];

    let stats = RepoStats::from_commits(&commits, Some("bob@example.com"));

    assert_eq!(stats.total_commits, 1);
    assert_eq!(stats.total_authors, 1);
    assert_eq!(stats.authors[0].name, "Bob");
}

#[test]
fn test_repo_stats_author_filter_case_insensitive() {
    let commits = vec![
        create_test_commit("Alice", "alice@example.com", 1, "feat: add feature"),
        create_test_commit("Bob", "bob@example.com", 2, "fix: bug"),
    ];

    let stats = RepoStats::from_commits(&commits, Some("ALICE"));

    assert_eq!(stats.total_commits, 1);
    assert_eq!(stats.total_authors, 1);
    assert_eq!(stats.authors[0].name, "Alice");
}

#[test]
fn test_repo_stats_author_filter_no_match() {
    let commits = vec![create_test_commit(
        "Alice",
        "alice@example.com",
        1,
        "feat: add feature",
    )];

    let stats = RepoStats::from_commits(&commits, Some("Charlie"));

    assert_eq!(stats.total_commits, 0);
    assert_eq!(stats.total_authors, 0);
}

// === 周统计测试 ===

#[test]
fn test_repo_stats_commits_by_week() {
    let commits = vec![
        create_test_commit("Alice", "alice@example.com", 1, "commit in week 1"), // 当前周
        create_test_commit("Alice", "alice@example.com", 8, "commit in week 2"), // 上周
        create_test_commit("Alice", "alice@example.com", 15, "commit in week 3"), // 2 周前
        create_test_commit("Alice", "alice@example.com", 22, "commit in week 4"), // 3 周前
        create_test_commit("Alice", "alice@example.com", 100, "old commit"), // 超过 4 周，不计入
    ];

    let stats = RepoStats::from_commits(&commits, None);

    // 应该初始化最近 4 周
    assert!(stats.commits_by_week.len() >= 4);

    // 总和应该是 4（不包括 100 天前的）
    let total_in_weeks: usize = stats.commits_by_week.values().sum();
    assert_eq!(total_in_weeks, 4);
}

// === 时间跨度测试 ===

#[test]
fn test_repo_stats_days_span() {
    let commits = vec![
        create_test_commit("Alice", "alice@example.com", 1, "recent"),
        create_test_commit("Bob", "bob@example.com", 30, "old"),
    ];

    let stats = RepoStats::from_commits(&commits, None);

    let days = stats.days_span().unwrap();
    assert!(
        (28..=30).contains(&days),
        "Expected 28-30 days, got {}",
        days
    );
}

#[test]
fn test_repo_stats_days_span_single_day() {
    let commits = vec![
        create_test_commit("Alice", "alice@example.com", 5, "commit 1"),
        create_test_commit("Bob", "bob@example.com", 5, "commit 2"),
    ];

    let stats = RepoStats::from_commits(&commits, None);

    assert_eq!(stats.days_span(), Some(0)); // 同一天
}
