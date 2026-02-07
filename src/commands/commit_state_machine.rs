//! Commit 流程状态机
//!
//! 纯函数式状态机，管理 commit message 生成和用户交互流程。
//!
//! # 状态转换图
//! ```text
//! Generating ──────────> WaitingForAction ──────────> Accepted
//!     │                        │                           │
//!     │                        ├──> Generating (retry)     └──> 执行 commit
//!     │                        └──> Cancelled
//!     └──> MaxRetriesExceeded ──> Cancelled
//! ```
//!
//! # 设计理念
//! - 状态转换是纯函数（无副作用）
//! - IO 操作由外部处理（`commands/commit.rs`）
//! - 便于测试和推理
//!
//! # 使用示例
//! ```no_run
//! use gcop_rs::commands::commit_state_machine::{
//!     CommitState, UserAction, GenerationResult
//! };
//!
//! # fn main() -> anyhow::Result<()> {
//! // 1. 初始状态
//! let state = CommitState::Generating {
//!     attempt: 0,
//!     feedbacks: vec![],
//! };
//!
//! // 2. 处理生成结果
//! let state = state.handle_generation(
//!     GenerationResult::Success("feat: add login".to_string()),
//!     false, // 非 auto-accept
//! )?;
//!
//! // 3. 处理用户动作
//! let state = state.handle_action(UserAction::Accept);
//!
//! // 4. 检查最终状态
//! if let CommitState::Accepted { message } = state {
//!     println!("Ready to commit: {}", message);
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{GcopError, Result};

/// Commit 流程的状态
///
/// 状态机的四种状态，每种状态对应一个用户可见的阶段。
///
/// # 变体
/// - [`Generating`] - 正在生成 commit message
/// - [`WaitingForAction`] - 等待用户操作
/// - [`Accepted`] - 用户接受 message
/// - [`Cancelled`] - 用户取消或达到最大重试次数
///
/// [`Generating`]: CommitState::Generating
/// [`WaitingForAction`]: CommitState::WaitingForAction
/// [`Accepted`]: CommitState::Accepted
/// [`Cancelled`]: CommitState::Cancelled
#[derive(Debug, Clone, PartialEq)]
pub enum CommitState {
    /// 正在生成 commit message
    ///
    /// 初始状态或用户选择 Retry 后的状态。
    ///
    /// # 字段
    /// - `attempt`: 当前尝试次数（从 0 开始）
    /// - `feedbacks`: 用户提供的反馈列表（用于重新生成）
    Generating {
        attempt: usize,
        feedbacks: Vec<String>,
    },
    /// 等待用户操作
    ///
    /// 显示生成的 message，等待用户选择：Accept/Edit/Retry/Quit。
    ///
    /// # 字段
    /// - `message`: 生成的 commit message
    /// - `attempt`: 当前尝试次数
    /// - `feedbacks`: 历史反馈列表
    WaitingForAction {
        message: String,
        attempt: usize,
        feedbacks: Vec<String>,
    },
    /// 用户接受 message
    ///
    /// 终止状态，准备执行 commit。
    ///
    /// # 字段
    /// - `message`: 确认的 commit message
    Accepted { message: String },
    /// 用户取消或达到最大重试次数
    ///
    /// 终止状态，不执行 commit。
    Cancelled,
}

/// 用户操作
///
/// 在 [`CommitState::WaitingForAction`] 状态下，用户可以选择的操作。
///
/// # 变体
/// - [`Accept`] - 接受当前 message 并提交
/// - [`Edit`] - 编辑 message
/// - [`EditCancelled`] - 编辑被取消（ESC 或关闭编辑器）
/// - [`Retry`] - 重新生成（不提供反馈）
/// - [`RetryWithFeedback`] - 重新生成并提供反馈
/// - [`Quit`] - 退出（不提交）
///
/// [`Accept`]: UserAction::Accept
/// [`Edit`]: UserAction::Edit
/// [`EditCancelled`]: UserAction::EditCancelled
/// [`Retry`]: UserAction::Retry
/// [`RetryWithFeedback`]: UserAction::RetryWithFeedback
/// [`Quit`]: UserAction::Quit
#[derive(Debug, Clone, PartialEq)]
pub enum UserAction {
    /// 接受当前 message 并提交
    Accept,
    /// 编辑 message
    ///
    /// # 字段
    /// - `new_message`: 编辑后的 commit message
    Edit { new_message: String },
    /// 编辑被取消（ESC 或关闭编辑器）
    EditCancelled,
    /// 重新生成（不提供反馈）
    Retry,
    /// 重新生成并提供反馈
    ///
    /// # 字段
    /// - `feedback`: 用户提供的反馈（可选）
    RetryWithFeedback { feedback: Option<String> },
    /// 退出（不提交）
    Quit,
}

/// 生成结果抽象
///
/// LLM 生成 commit message 的结果。
///
/// # 变体
/// - [`Success`] - 生成成功
/// - [`MaxRetriesExceeded`] - 达到最大重试次数
///
/// [`Success`]: GenerationResult::Success
/// [`MaxRetriesExceeded`]: GenerationResult::MaxRetriesExceeded
#[derive(Debug, Clone)]
pub enum GenerationResult {
    /// 生成成功
    ///
    /// # 字段
    /// - 生成的 commit message
    Success(String),
    /// 达到最大重试次数
    MaxRetriesExceeded,
}

impl CommitState {
    /// 检查是否达到最大重试次数
    ///
    /// # 参数
    /// - `max_retries`: 配置的最大重试次数
    ///
    /// # 返回
    /// - `true`: 已达到最大重试次数
    /// - `false`: 还可以继续重试
    ///
    /// # 示例
    /// ```
    /// # use gcop_rs::commands::commit_state_machine::CommitState;
    /// let state = CommitState::Generating { attempt: 5, feedbacks: vec![] };
    /// assert!(state.is_at_max_retries(5));  // attempt 5 = 第 6 次尝试
    /// assert!(!state.is_at_max_retries(10));
    /// ```
    pub fn is_at_max_retries(&self, max_retries: usize) -> bool {
        matches!(self, CommitState::Generating { attempt, .. } if *attempt >= max_retries)
    }

    /// 处理生成结果（纯函数）
    ///
    /// 将 [`CommitState::Generating`] 状态转换为下一个状态。
    ///
    /// # 参数
    /// - `result`: LLM 生成结果
    /// - `auto_accept`: 是否自动接受（`--yes` flag）
    ///
    /// # 返回
    /// - `Ok(next_state)` - 转换成功
    /// - `Err(_)` - 达到最大重试次数或状态不匹配
    ///
    /// # 状态转换
    /// - `Success` + `auto_accept=false` → `WaitingForAction`
    /// - `Success` + `auto_accept=true` → `Accepted`
    /// - `MaxRetriesExceeded` → `Err(MaxRetriesExceeded)`
    ///
    /// # 错误
    /// - 在非 `Generating` 状态调用此方法会返回 [`GcopError::InvalidInput`]
    ///
    /// # 示例
    /// ```
    /// # use gcop_rs::commands::commit_state_machine::{CommitState, GenerationResult};
    /// # fn main() -> anyhow::Result<()> {
    /// let state = CommitState::Generating { attempt: 0, feedbacks: vec![] };
    /// let state = state.handle_generation(
    ///     GenerationResult::Success("feat: add feature".to_string()),
    ///     false,
    /// )?;
    /// assert!(matches!(state, CommitState::WaitingForAction { .. }));
    /// # Ok(())
    /// # }
    /// ```
    pub fn handle_generation(self, result: GenerationResult, auto_accept: bool) -> Result<Self> {
        match self {
            CommitState::Generating { attempt, feedbacks } => match result {
                GenerationResult::MaxRetriesExceeded => Err(GcopError::MaxRetriesExceeded(attempt)),
                GenerationResult::Success(message) => {
                    if auto_accept {
                        Ok(CommitState::Accepted { message })
                    } else {
                        Ok(CommitState::WaitingForAction {
                            message,
                            attempt,
                            feedbacks,
                        })
                    }
                }
            },
            _ => Err(GcopError::InvalidInput(format!(
                "handle_generation called in wrong state: {:?}",
                self
            ))),
        }
    }

    /// 处理用户动作（纯函数）
    ///
    /// 将 [`CommitState::WaitingForAction`] 状态转换为下一个状态。
    ///
    /// # 参数
    /// - `action`: 用户选择的动作
    ///
    /// # 返回
    /// 下一个状态（总是成功）
    ///
    /// # 状态转换
    /// - `Accept` → `Accepted`
    /// - `Edit { new_message }` → `WaitingForAction`（保留 attempt 和 feedbacks）
    /// - `EditCancelled` → `WaitingForAction`（保留原 message）
    /// - `Retry` → `Generating`（attempt + 1，保留 feedbacks）
    /// - `RetryWithFeedback { feedback }` → `Generating`（attempt + 1，追加 feedback）
    /// - `Quit` → `Cancelled`
    ///
    /// # 错误处理
    /// 在非 `WaitingForAction` 状态调用此方法会：
    /// - 记录错误日志
    /// - 返回 `Cancelled` 状态（防御性处理）
    ///
    /// # 示例
    /// ```
    /// # use gcop_rs::commands::commit_state_machine::{CommitState, UserAction};
    /// let state = CommitState::WaitingForAction {
    ///     message: "feat: add login".to_string(),
    ///     attempt: 0,
    ///     feedbacks: vec![],
    /// };
    ///
    /// let state = state.handle_action(UserAction::Accept);
    /// assert!(matches!(state, CommitState::Accepted { .. }));
    /// ```
    pub fn handle_action(self, action: UserAction) -> Self {
        match self {
            CommitState::WaitingForAction {
                message,
                attempt,
                feedbacks,
            } => match action {
                UserAction::Accept => CommitState::Accepted { message },

                UserAction::Edit { new_message } => CommitState::WaitingForAction {
                    message: new_message,
                    attempt,
                    feedbacks,
                },

                UserAction::EditCancelled => CommitState::WaitingForAction {
                    message,
                    attempt,
                    feedbacks,
                },

                UserAction::Retry => CommitState::Generating {
                    attempt: attempt + 1,
                    feedbacks,
                },

                UserAction::RetryWithFeedback { feedback } => {
                    let mut new_feedbacks = feedbacks;
                    if let Some(fb) = feedback {
                        new_feedbacks.push(fb);
                    }
                    CommitState::Generating {
                        attempt: attempt + 1,
                        feedbacks: new_feedbacks,
                    }
                }

                UserAction::Quit => CommitState::Cancelled,
            },
            _ => {
                tracing::error!("handle_action called in wrong state: {:?}", self);
                CommitState::Cancelled
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // === 初始状态测试 ===

    #[test]
    fn test_initial_state() {
        let state = CommitState::Generating {
            attempt: 0,
            feedbacks: vec![],
        };
        assert!(!state.is_at_max_retries(10));
    }

    #[test]
    fn test_max_retries_boundary() {
        let state_at_limit = CommitState::Generating {
            attempt: 10,
            feedbacks: vec![],
        };
        assert!(state_at_limit.is_at_max_retries(10));

        let state_before_limit = CommitState::Generating {
            attempt: 9,
            feedbacks: vec![],
        };
        assert!(!state_before_limit.is_at_max_retries(10));
    }

    // === Generating 状态转换测试 ===

    #[test]
    fn test_generating_success_no_auto_accept() {
        let state = CommitState::Generating {
            attempt: 0,
            feedbacks: vec![],
        };
        let result = state
            .handle_generation(
                GenerationResult::Success("feat: add feature".to_string()),
                false,
            )
            .unwrap();

        assert!(matches!(result, CommitState::WaitingForAction {
            message,
            attempt: 0,
            ..
        } if message == "feat: add feature"));
    }

    #[test]
    fn test_generating_success_with_auto_accept() {
        let state = CommitState::Generating {
            attempt: 0,
            feedbacks: vec![],
        };
        let result = state
            .handle_generation(
                GenerationResult::Success("feat: add feature".to_string()),
                true, // --yes flag
            )
            .unwrap();

        assert!(matches!(result, CommitState::Accepted { message }
            if message == "feat: add feature"));
    }

    #[test]
    fn test_generating_max_retries_exceeded() {
        let state = CommitState::Generating {
            attempt: 10,
            feedbacks: vec![],
        };
        let result = state.handle_generation(GenerationResult::MaxRetriesExceeded, false);

        match result {
            Err(GcopError::MaxRetriesExceeded(attempt)) => {
                assert_eq!(attempt, 10);
            }
            other => panic!("Expected MaxRetriesExceeded, got {:?}", other),
        }
    }

    #[test]
    fn test_generating_preserves_feedbacks() {
        let feedbacks = vec!["use Chinese".to_string(), "be concise".to_string()];
        let state = CommitState::Generating {
            attempt: 2,
            feedbacks: feedbacks.clone(),
        };

        let result = state
            .handle_generation(GenerationResult::Success("msg".to_string()), false)
            .unwrap();

        if let CommitState::WaitingForAction {
            feedbacks: f,
            attempt,
            ..
        } = result
        {
            assert_eq!(f, feedbacks);
            assert_eq!(attempt, 2);
        } else {
            panic!("Expected WaitingForAction");
        }
    }

    // === WaitingForAction 状态转换测试 ===

    #[test]
    fn test_waiting_accept() {
        let state = CommitState::WaitingForAction {
            message: "test msg".to_string(),
            attempt: 0,
            feedbacks: vec![],
        };

        let result = state.handle_action(UserAction::Accept);
        assert!(matches!(result, CommitState::Accepted { message }
            if message == "test msg"));
    }

    #[test]
    fn test_waiting_edit_success() {
        let state = CommitState::WaitingForAction {
            message: "original".to_string(),
            attempt: 1,
            feedbacks: vec!["fb1".to_string()],
        };

        let result = state.handle_action(UserAction::Edit {
            new_message: "edited".to_string(),
        });

        assert!(matches!(result, CommitState::WaitingForAction {
            message,
            attempt: 1,
            feedbacks
        } if message == "edited" && feedbacks.len() == 1));
    }

    #[test]
    fn test_waiting_edit_cancelled_preserves_message() {
        let state = CommitState::WaitingForAction {
            message: "original".to_string(),
            attempt: 0,
            feedbacks: vec![],
        };

        let result = state.handle_action(UserAction::EditCancelled);

        assert!(matches!(result, CommitState::WaitingForAction {
            message,
            ..
        } if message == "original"));
    }

    #[test]
    fn test_waiting_retry_increments_attempt() {
        let state = CommitState::WaitingForAction {
            message: "msg".to_string(),
            attempt: 2,
            feedbacks: vec!["old".to_string()],
        };

        let result = state.handle_action(UserAction::Retry);

        assert!(matches!(result, CommitState::Generating {
            attempt: 3,
            feedbacks
        } if feedbacks == vec!["old".to_string()]));
    }

    #[test]
    fn test_waiting_retry_with_feedback_accumulates() {
        let state = CommitState::WaitingForAction {
            message: "msg".to_string(),
            attempt: 0,
            feedbacks: vec!["first".to_string()],
        };

        let result = state.handle_action(UserAction::RetryWithFeedback {
            feedback: Some("second".to_string()),
        });

        if let CommitState::Generating { attempt, feedbacks } = result {
            assert_eq!(attempt, 1);
            assert_eq!(feedbacks, vec!["first".to_string(), "second".to_string()]);
        } else {
            panic!("Expected Generating");
        }
    }

    #[test]
    fn test_waiting_retry_with_no_feedback() {
        let state = CommitState::WaitingForAction {
            message: "msg".to_string(),
            attempt: 0,
            feedbacks: vec![],
        };

        let result = state.handle_action(UserAction::RetryWithFeedback { feedback: None });

        if let CommitState::Generating { feedbacks, .. } = result {
            assert!(feedbacks.is_empty());
        } else {
            panic!("Expected Generating");
        }
    }

    #[test]
    fn test_waiting_quit() {
        let state = CommitState::WaitingForAction {
            message: "msg".to_string(),
            attempt: 5,
            feedbacks: vec!["a".to_string(), "b".to_string()],
        };

        let result = state.handle_action(UserAction::Quit);
        assert!(matches!(result, CommitState::Cancelled));
    }
}
