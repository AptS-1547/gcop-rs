use crate::error::{GcopError, Result};

/// Commit 流程状态
#[derive(Debug, Clone, PartialEq)]
pub enum CommitState {
    /// 需要生成/重新生成 message
    Generating {
        attempt: usize,
        feedbacks: Vec<String>,
    },
    /// 展示生成的 message 并等待用户操作
    WaitingForAction {
        message: String,
        attempt: usize,
        feedbacks: Vec<String>,
    },
    /// 用户接受，准备提交
    Accepted { message: String },
    /// 用户取消
    Cancelled,
}

/// 用户动作抽象
#[derive(Debug, Clone, PartialEq)]
pub enum UserAction {
    Accept,
    Edit { new_message: String },
    EditCancelled,
    Retry,
    RetryWithFeedback { feedback: Option<String> },
    Quit,
}

/// 生成结果抽象
#[derive(Debug, Clone)]
pub enum GenerationResult {
    Success(String),
    MaxRetriesExceeded,
}

impl CommitState {
    /// 检查是否达到重试上限
    pub fn is_at_max_retries(&self, max_retries: usize) -> bool {
        matches!(self, CommitState::Generating { attempt, .. } if *attempt >= max_retries)
    }

    /// 处理生成结果（纯函数）
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
            _ => unreachable!("handle_generation called in wrong state"),
        }
    }

    /// 处理用户动作（纯函数）
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
            _ => unreachable!("handle_action called in wrong state"),
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

        assert!(result.is_err());
        if let Err(GcopError::Other(msg)) = result {
            assert!(msg.contains("Too many retries"));
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
