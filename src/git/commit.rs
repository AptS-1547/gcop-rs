use git2::Repository;

use crate::error::Result;

/// 执行 git commit
///
/// # Arguments
/// * `repo` - Git 仓库
/// * `message` - Commit 消息
pub fn commit_changes(repo: &Repository, message: &str) -> Result<()> {
    // 1. 获取当前 index
    let mut index = repo.index()?;

    // 2. 写入 tree
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    // 3. 获取 signature（从 git config）
    let signature = repo.signature()?;

    // 4. 获取 HEAD commit 作为 parent
    let head = repo.head()?;
    let parent_commit = head.peel_to_commit()?;

    // 5. 创建 commit
    repo.commit(
        Some("HEAD"),      // 更新 HEAD
        &signature,        // author
        &signature,        // committer
        message,           // commit message
        &tree,             // tree
        &[&parent_commit], // parents
    )?;

    Ok(())
}
