// 测试 Git 操作模块
use gcop_rs::git::{GitOperations, repository::GitRepository};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing Git Operations Module ===\n");

    // 打开仓库
    let repo = GitRepository::open()?;
    println!("✓ Successfully opened Git repository");

    // 获取当前分支
    if let Some(branch) = repo.get_current_branch()? {
        println!("✓ Current branch: {}", branch);
    } else {
        println!("⚠ HEAD is in detached state");
    }

    // 检查是否有 staged changes
    let has_staged = repo.has_staged_changes()?;
    println!(
        "✓ Staged changes: {}",
        if has_staged { "yes" } else { "no" }
    );

    if has_staged {
        // 获取 staged diff
        let diff = repo.get_staged_diff()?;
        println!("\n--- Staged Diff (first 500 characters) ---");
        println!("{}", &diff[..diff.len().min(500)]);

        // 获取统计信息
        let stats = repo.get_diff_stats(&diff)?;
        println!("\n--- Diff Statistics ---");
        println!("Files changed: {:?}", stats.files_changed);
        println!("Insertions: {}", stats.insertions);
        println!("Deletions: {}", stats.deletions);
    }

    println!("\n✓ All tests passed!");
    Ok(())
}
