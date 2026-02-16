use crate::error::{GcopError, Result};

/// Call the system editor to edit text
///
/// Use the `edit` crate to automatically select editors by priority:
/// $VISUAL > $EDITOR > Platform default list (nano/vim/vi/emacs/...)
/// If the editor pointed to by the environment variable does not exist, it will automatically fall back to the next available editor.
///
/// # Arguments
/// * `initial_content` - initial content
///
/// # Returns
/// * `Ok(String)` - edited content
/// * `Err(GcopError::UserCancelled)` - The user cleared the content
/// * `Err(_)` - other errors
pub fn edit_text(initial_content: &str) -> Result<String> {
    let edited = edit::edit(initial_content)?;

    // Remove leading and trailing whitespace and check if it is empty
    let trimmed = edited.trim();

    if trimmed.is_empty() {
        return Err(GcopError::UserCancelled);
    }

    // Returns the edited content (preserving the user's formatting)
    Ok(edited)
}
