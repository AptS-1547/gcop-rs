//! Commit process state machine
//!
//! Purely functional state machine manages commit message generation and user interaction processes.
//!
//! # State transition diagram
//! ```text
//! Generating ──────────> WaitingForAction ──────────> Accepted
//!     │                        │                           │
//!     │ ├──> Generating (retry) └──> Execute commit
//!     │                        └──> Cancelled
//!     └──> MaxRetriesExceeded ──> Cancelled
//! ```
//!
//! # Design
//! - State transitions are pure functions (no side effects)
//! - IO operations are handled externally (`commands/commit.rs`)
//! - Easy to test and reason about
//!
//! # Usage example
//! ```no_run
//! use gcop_rs::commands::commit_state_machine::{
//!     CommitState, UserAction, GenerationResult
//! };
//!
//! # fn main() -> anyhow::Result<()> {
//! // 1. Initial state
//! let state = CommitState::Generating {
//!     attempt: 0,
//!     feedbacks: vec![],
//! };
//!
//! // 2. Process the generated results
//! let state = state.handle_generation(
//!     GenerationResult::Success("feat: add login".to_string()),
//!     false, // not auto-accept
//! )?;
//!
//! // 3. Process user actions
//! let state = state.handle_action(UserAction::Accept);
//!
//! // 4. Check the final status
//! if let CommitState::Accepted { message } = state {
//!     println!("Ready to commit: {}", message);
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{GcopError, Result};

/// Commit process status
///
/// There are four states of the state machine, each state corresponds to a user-visible stage.
///
/// # Variants
/// - [`Generating`] - Generating commit message
/// - [`WaitingForAction`] - Waiting for user action
/// - [`Accepted`] - The user accepted the message
/// - [`Cancelled`] - User canceled or maximum retries reached
///
/// [`Generating`]: CommitState::Generating
/// [`WaitingForAction`]: CommitState::WaitingForAction
/// [`Accepted`]: CommitState::Accepted
/// [`Cancelled`]: CommitState::Cancelled
#[derive(Debug, Clone, PartialEq)]
pub enum CommitState {
    /// Generating commit message
    ///
    /// The initial state or the state after the user selects Retry.
    ///
    /// # Fields
    /// - `attempt`: current number of attempts (starting from 0)
    /// - `feedbacks`: list of user-provided feedback (used for regeneration)
    Generating {
        /// Zero-based attempt counter used for max-retry checks.
        attempt: usize,
        /// Collected user feedback messages from previous retries.
        feedbacks: Vec<String>,
    },
    /// Wait for user action
    ///
    /// Display the generated message and wait for the user to select: Accept/Edit/Retry/Quit.
    ///
    /// # Fields
    /// - `message`: generated commit message
    /// - `attempt`: current number of attempts
    /// - `feedbacks`: historical feedback list
    WaitingForAction {
        /// Latest generated commit message shown to the user.
        message: String,
        /// Attempt counter carried from the generating phase.
        attempt: usize,
        /// Feedback history carried into future retries.
        feedbacks: Vec<String>,
    },
    /// User accepts message
    ///
    /// Termination status, ready to commit.
    ///
    /// # Fields
    /// - `message`: confirmed commit message
    Accepted {
        /// Commit message accepted by the user.
        message: String,
    },
    /// User cancels or maximum retries reached
    ///
    /// Termination status, no commit is performed.
    Cancelled,
}

/// User operations
///
/// In the [`CommitState::WaitingForAction`] state, the user can choose the action.
///
/// # Variants
/// - [`Accept`] - accept the current message and submit it
/// - [`Edit`] - edit message
/// - [`EditCancelled`] - Editing was canceled (ESC or close the editor)
/// - [`Retry`] - regenerate (no feedback)
/// - [`RetryWithFeedback`] - Regenerate with feedback
/// - [`Quit`] - Quit (without committing)
///
/// [`Accept`]: UserAction::Accept
/// [`Edit`]: UserAction::Edit
/// [`EditCancelled`]: UserAction::EditCancelled
/// [`Retry`]: UserAction::Retry
/// [`RetryWithFeedback`]: UserAction::RetryWithFeedback
/// [`Quit`]: UserAction::Quit
#[derive(Debug, Clone, PartialEq)]
pub enum UserAction {
    /// Accept the current message and submit it
    Accept,
    /// edit message
    ///
    /// # Fields
    /// - `new_message`: edited commit message
    Edit {
        /// Commit message content returned by the editor.
        new_message: String,
    },
    /// Editing canceled (ESC or close editor)
    EditCancelled,
    /// Retry (no feedback provided)
    Retry,
    /// Regenerate and provide feedback
    ///
    /// # Fields
    /// - `feedback`: feedback provided by the user (optional)
    RetryWithFeedback {
        /// Optional free-form feedback passed back to the model.
        feedback: Option<String>,
    },
    /// Exit (without submitting)
    Quit,
}

/// Generate result abstraction
///
/// LLM generates the result of commit message.
///
/// # Variants
/// - [`Success`] - generated successfully
/// - [`MaxRetriesExceeded`] - Maximum number of retries reached
///
/// [`Success`]: GenerationResult::Success
/// [`MaxRetriesExceeded`]: GenerationResult::MaxRetriesExceeded
#[derive(Debug, Clone)]
pub enum GenerationResult {
    /// Generated successfully
    ///
    /// # Fields
    /// - Generated commit message
    Success(String),
    /// Maximum number of retries reached
    MaxRetriesExceeded,
}

impl CommitState {
    /// Check if the maximum number of retries has been reached
    ///
    /// # Parameters
    /// - `max_retries`: configured maximum number of retries
    ///
    /// # Returns
    /// - `true`: The maximum number of retries has been reached
    /// - `false`: You can continue to try again
    ///
    /// # Example
    /// ```
    /// # use gcop_rs::commands::commit_state_machine::CommitState;
    /// let state = CommitState::Generating { attempt: 5, feedbacks: vec![] };
    /// assert!(state.is_at_max_retries(5)); // attempt 5 = 6th attempt
    /// assert!(!state.is_at_max_retries(10));
    /// ```
    pub fn is_at_max_retries(&self, max_retries: usize) -> bool {
        matches!(self, CommitState::Generating { attempt, .. } if *attempt >= max_retries)
    }

    /// Process generated results (pure function)
    ///
    /// Convert the [`CommitState::Generating`] state to the next state.
    ///
    /// # Parameters
    /// - `result`: LLM generated results
    /// - `auto_accept`: whether to automatically accept (`--yes` flag)
    ///
    /// # Returns
    /// - `Ok(next_state)` - Conversion successful
    /// - `Err(_)` - Maximum number of retries reached or status mismatch
    ///
    /// #State transition
    /// - `Success` + `auto_accept=false` → `WaitingForAction`
    /// - `Success` + `auto_accept=true` → `Accepted`
    /// - `MaxRetriesExceeded` → `Err(MaxRetriesExceeded)`
    ///
    /// # Errors
    /// - Calling this method in a non-`Generating` state will return [`GcopError::InvalidInput`]
    ///
    /// # Example
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

    /// Handle user actions (pure function)
    ///
    /// Transition the [`CommitState::WaitingForAction`] state to the next state.
    ///
    /// # Parameters
    /// - `action`: the action selected by the user
    ///
    /// # Returns
    /// next state (always successful)
    ///
    /// #State transition
    /// - `Accept` → `Accepted`
    /// - `Edit { new_message }` → `WaitingForAction` (keep attempt and feedbacks)
    /// - `EditCancelled` → `WaitingForAction` (retain original message)
    /// - `Retry` → `Generating` (attempt + 1, retain feedbacks)
    /// - `RetryWithFeedback { feedback }` → `Generating` (attempt + 1, append feedback)
    /// - `Quit` → `Cancelled`
    ///
    /// # Error handling
    /// Calling this method in a non-`WaitingForAction` state will:
    /// - Record error log
    /// - Return `Cancelled` status (defensive handling)
    ///
    /// # Example
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

    // === Initial state test ===

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

    // === Generating state transition test ===

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

    // === WaitingForAction state transition test ===

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
