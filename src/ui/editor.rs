use crate::error::{GcopError, Result};
use dialoguer::Editor;

/// 调用系统编辑器编辑文本
///
/// # Arguments
/// * `initial_content` - 初始内容
///
/// # Returns
/// * `Ok(String)` - 编辑后的内容
/// * `Err(GcopError::UserCancelled)` - 用户清空了内容或取消编辑
/// * `Err(_)` - 其他错误
pub fn edit_text(initial_content: &str) -> Result<String> {
    let edited = Editor::new()
        .edit(initial_content)?
        .ok_or(GcopError::UserCancelled)?;

    // 移除前后空白，检查是否为空
    let trimmed = edited.trim();

    if trimmed.is_empty() {
        return Err(GcopError::UserCancelled);
    }

    // 返回编辑后的内容（保留用户的格式）
    Ok(edited)
}
