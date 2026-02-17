//! Commit 状态机集成测试示例
//!
//! 展示如何使用 MockGitOperations 和自定义 MockLLMProvider

use async_trait::async_trait;
use gcop_rs::error::Result;
use gcop_rs::git::{GitOperations, MockGitOperations};
use gcop_rs::llm::{CommitContext, LLMProvider, ReviewResult, ReviewType};

/// 测试用的 MockLLMProvider 示例
struct MockLLMProvider {
    message: String,
}

impl MockLLMProvider {
    fn new(message: String) -> Self {
        Self { message }
    }
}

#[async_trait]
impl LLMProvider for MockLLMProvider {
    async fn send_prompt(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _progress: Option<&dyn gcop_rs::llm::ProgressReporter>,
    ) -> Result<String> {
        Ok(self.message.clone())
    }

    async fn generate_commit_message(
        &self,
        _diff: &str,
        _context: Option<CommitContext>,
        _progress: Option<&dyn gcop_rs::llm::ProgressReporter>,
    ) -> Result<String> {
        Ok(self.message.clone())
    }

    async fn review_code(
        &self,
        _diff: &str,
        _review_type: ReviewType,
        _custom_prompt: Option<&str>,
        _progress: Option<&dyn gcop_rs::llm::ProgressReporter>,
    ) -> Result<ReviewResult> {
        unimplemented!("review not used in commit tests")
    }

    fn name(&self) -> &str {
        "MockLLMProvider"
    }

    async fn validate(&self) -> Result<()> {
        Ok(())
    }
}

// === Mock 基础功能测试 ===

#[tokio::test]
async fn test_git_mock_basic() {
    // 测试 MockGitOperations 的基本功能
    let mut mock_git = MockGitOperations::new();
    mock_git
        .expect_has_staged_changes()
        .times(1)
        .returning(|| Ok(true));

    assert!(mock_git.has_staged_changes().unwrap());
}

#[tokio::test]
async fn test_llm_mock_basic() {
    // 测试 MockLLMProvider 的基本功能
    let mock_llm = MockLLMProvider::new("feat: test commit".to_string());

    let result = mock_llm
        .generate_commit_message("diff", None, None)
        .await
        .unwrap();

    assert_eq!(result, "feat: test commit");
}

#[tokio::test]
async fn test_git_mock_multiple_calls() {
    // 测试 mock 的多次调用
    let mut mock_git = MockGitOperations::new();

    mock_git
        .expect_get_staged_diff()
        .times(1)
        .returning(|| Ok("diff --git a/test.rs\n+new line".to_string()));

    mock_git.expect_get_diff_stats().times(1).returning(|_| {
        Ok(gcop_rs::git::DiffStats {
            files_changed: vec!["test.rs".to_string()],
            insertions: 1,
            deletions: 0,
        })
    });

    let diff = mock_git.get_staged_diff().unwrap();
    assert!(diff.contains("test.rs"));

    let stats = mock_git.get_diff_stats(&diff).unwrap();
    assert_eq!(stats.insertions, 1);
}

// 注意：完整的 commit 流程集成测试需要重构 commit.rs 的 run() 函数
// 以接受 trait 对象参数。当前 run() 函数直接创建具体实现，无法注入 mock。
// 由于状态机纯逻辑已有 13 个测试覆盖，暂不实施深度集成测试。
