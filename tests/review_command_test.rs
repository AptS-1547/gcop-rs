//! Review 命令集成测试
//!
//! 测试 review 命令的：
//! - 4 种 target 类型路由（Changes/Commit/Range/File）
//! - 错误处理（空 diff、LLM 失败）

use async_trait::async_trait;
use gcop_rs::cli::ReviewTarget;
use gcop_rs::commands::{OutputFormat, ReviewOptions};
use gcop_rs::config::AppConfig;
use gcop_rs::error::{GcopError, Result};
use gcop_rs::git::MockGitOperations;
use gcop_rs::llm::{
    CommitContext, IssueSeverity, LLMProvider, ReviewIssue, ReviewResult, ReviewType,
};

// ========== Mock LLM Provider ==========

struct MockReviewLLM {
    expected_review_type: ReviewType,
    should_fail: bool,
}

impl MockReviewLLM {
    fn new(expected_review_type: ReviewType) -> Self {
        Self {
            expected_review_type,
            should_fail: false,
        }
    }

    fn with_failure() -> Self {
        Self {
            expected_review_type: ReviewType::UncommittedChanges,
            should_fail: true,
        }
    }
}

#[async_trait]
impl LLMProvider for MockReviewLLM {
    async fn review_code(
        &self,
        _diff: &str,
        review_type: ReviewType,
        _custom_prompt: Option<&str>,
        _spinner: Option<&gcop_rs::ui::Spinner>,
    ) -> Result<ReviewResult> {
        if self.should_fail {
            return Err(GcopError::LlmApi {
                status: 503,
                message: "Service Unavailable".to_string(),
            });
        }

        // 验证 review_type 正确传递
        match (&self.expected_review_type, &review_type) {
            (ReviewType::UncommittedChanges, ReviewType::UncommittedChanges) => {}
            (ReviewType::SingleCommit(a), ReviewType::SingleCommit(b)) if a == b => {}
            (ReviewType::CommitRange(a), ReviewType::CommitRange(b)) if a == b => {}
            (ReviewType::FileOrDir(a), ReviewType::FileOrDir(b)) if a == b => {}
            _ => {
                panic!(
                    "Review type mismatch: expected {:?}, got {:?}",
                    self.expected_review_type, review_type
                );
            }
        }

        Ok(ReviewResult {
            summary: "Test review summary".to_string(),
            issues: vec![ReviewIssue {
                severity: IssueSeverity::Warning,
                description: "Test issue".to_string(),
                file: Some("test.rs".to_string()),
                line: Some(42),
            }],
            suggestions: vec!["Test suggestion".to_string()],
        })
    }

    async fn generate_commit_message(
        &self,
        _diff: &str,
        _context: Option<CommitContext>,
        _spinner: Option<&gcop_rs::ui::Spinner>,
    ) -> Result<String> {
        unimplemented!("Not used in review tests")
    }

    fn name(&self) -> &str {
        "MockReviewLLM"
    }

    async fn validate(&self) -> Result<()> {
        Ok(())
    }
}

// ========== 测试用例 ==========

/// 创建测试用的 ReviewOptions
fn make_review_options(target: &ReviewTarget) -> ReviewOptions<'_> {
    ReviewOptions {
        target,
        format: OutputFormat::Text,
        verbose: false,
        provider_override: None,
    }
}

#[tokio::test]
async fn test_review_target_uncommitted_changes() {
    let mut mock_git = MockGitOperations::new();
    mock_git
        .expect_get_uncommitted_diff()
        .times(1)
        .returning(|| Ok("diff --git a/test.rs\n+new line".to_string()));

    let mock_llm = MockReviewLLM::new(ReviewType::UncommittedChanges);

    let config = AppConfig::default();
    let target = ReviewTarget::Changes;
    let options = make_review_options(&target);

    let result =
        gcop_rs::commands::review::run_internal(&options, &config, &mock_git, &mock_llm).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_review_target_single_commit() {
    let mut mock_git = MockGitOperations::new();
    mock_git
        .expect_get_commit_diff()
        .with(mockall::predicate::eq("abc123"))
        .times(1)
        .returning(|_| Ok("diff --git a/test.rs\n+new line".to_string()));

    let mock_llm = MockReviewLLM::new(ReviewType::SingleCommit("abc123".to_string()));

    let config = AppConfig::default();
    let target = ReviewTarget::Commit {
        hash: "abc123".to_string(),
    };
    let options = make_review_options(&target);

    let result =
        gcop_rs::commands::review::run_internal(&options, &config, &mock_git, &mock_llm).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_review_target_range() {
    let mut mock_git = MockGitOperations::new();
    mock_git
        .expect_get_range_diff()
        .with(mockall::predicate::eq("main..feature"))
        .times(1)
        .returning(|_| Ok("diff --git a/test.rs\n+new line".to_string()));

    let mock_llm = MockReviewLLM::new(ReviewType::CommitRange("main..feature".to_string()));

    let config = AppConfig::default();
    let target = ReviewTarget::Range {
        range: "main..feature".to_string(),
    };
    let options = make_review_options(&target);

    let result =
        gcop_rs::commands::review::run_internal(&options, &config, &mock_git, &mock_llm).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_review_target_file() {
    let mut mock_git = MockGitOperations::new();
    mock_git
        .expect_get_file_content()
        .with(mockall::predicate::eq("src/main.rs"))
        .times(1)
        .returning(|_| Ok("fn main() {}\n".to_string()));

    let mock_llm = MockReviewLLM::new(ReviewType::FileOrDir("src/main.rs".to_string()));

    let config = AppConfig::default();
    let target = ReviewTarget::File {
        path: "src/main.rs".to_string(),
    };
    let options = make_review_options(&target);

    let result =
        gcop_rs::commands::review::run_internal(&options, &config, &mock_git, &mock_llm).await;

    assert!(result.is_ok());
}

// ========== 错误处理测试 ==========

#[tokio::test]
async fn test_review_empty_uncommitted_changes_error() {
    let mut mock_git = MockGitOperations::new();
    mock_git
        .expect_get_uncommitted_diff()
        .times(1)
        .returning(|| Ok("".to_string())); // 空 diff

    let mock_llm = MockReviewLLM::new(ReviewType::UncommittedChanges);

    let config = AppConfig::default();
    let target = ReviewTarget::Changes;
    let options = make_review_options(&target);

    let result =
        gcop_rs::commands::review::run_internal(&options, &config, &mock_git, &mock_llm).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        GcopError::InvalidInput(msg) => {
            assert!(msg.contains("No uncommitted changes"));
        }
        _ => panic!("Expected InvalidInput error"),
    }
}

#[tokio::test]
async fn test_review_llm_failure() {
    let mut mock_git = MockGitOperations::new();
    mock_git
        .expect_get_uncommitted_diff()
        .times(1)
        .returning(|| Ok("diff --git a/test.rs\n+line".to_string()));

    let mock_llm = MockReviewLLM::with_failure();

    let config = AppConfig::default();
    let target = ReviewTarget::Changes;
    let options = make_review_options(&target);

    let result =
        gcop_rs::commands::review::run_internal(&options, &config, &mock_git, &mock_llm).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        GcopError::LlmApi { status, .. } => {
            assert_eq!(status, 503);
        }
        _ => panic!("Expected LlmApi error"),
    }
}
