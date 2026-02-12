use crate::error::{GcopError, Result};

/// 调用系统编辑器编辑文本
///
/// 使用 `edit` crate，自动按优先级选择编辑器：
/// $VISUAL > $EDITOR > 平台预设列表（nano/vim/vi/emacs/...）
/// 若环境变量指向的编辑器不存在，会自动回退到下一个可用编辑器。
///
/// # Arguments
/// * `initial_content` - 初始内容
///
/// # Returns
/// * `Ok(String)` - 编辑后的内容
/// * `Err(GcopError::UserCancelled)` - 用户清空了内容
/// * `Err(_)` - 其他错误
pub fn edit_text(initial_content: &str) -> Result<String> {
    let edited = edit::edit(initial_content)?;

    // 移除前后空白，检查是否为空
    let trimmed = edited.trim();

    if trimmed.is_empty() {
        return Err(GcopError::UserCancelled);
    }

    // 返回编辑后的内容（保留用户的格式）
    Ok(edited)
}
