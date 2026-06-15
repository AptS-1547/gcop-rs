use crate::error::{GcopError, Result};
use std::fs;
use std::path::Path;

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
    edit_text_with_suffix(initial_content, "")
}

/// Call the system editor with a temporary file suffix.
///
/// The suffix lets editors infer the correct syntax highlighting from the
/// temporary filename, e.g. `.toml` for configuration.
pub fn edit_text_with_suffix(initial_content: &str, suffix: &str) -> Result<String> {
    let mut builder = edit::Builder::new();
    builder.suffix(suffix);
    let edited = edit::edit_with_builder(initial_content, &builder)?;
    validate_edited_text(edited)
}

/// Call the system editor with a specific temporary filename.
///
/// Some editors detect syntax from well-known filenames instead of extensions,
/// such as `COMMIT_EDITMSG` for Git commit messages.
pub fn edit_text_with_filename(initial_content: &str, filename: &str) -> Result<String> {
    let file_name = Path::new(filename);
    if file_name.file_name() != Some(file_name.as_os_str()) {
        return Err(GcopError::InvalidInput(format!(
            "Editor filename must not contain path separators: {}",
            filename
        )));
    }

    let temp_dir = tempfile::Builder::new().prefix("gcop-editor-").tempdir()?;
    let path = temp_dir.path().join(file_name);
    fs::write(&path, initial_content)?;
    edit::edit_file(&path)?;
    let edited = fs::read_to_string(&path)?;
    validate_edited_text(edited)
}

fn validate_edited_text(edited: String) -> Result<String> {
    // Remove leading and trailing whitespace and check if it is empty
    let trimmed = edited.trim();

    if trimmed.is_empty() {
        return Err(GcopError::UserCancelled);
    }

    // Returns the edited content (preserving the user's formatting)
    Ok(edited)
}
