// 测试 Git 操作模块
use gcop_rs::git::{GitOperations, repository::GitRepository};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 测试 Git 操作模块 ===\n");

    // 打开仓库
    let repo = GitRepository::open()?;
    println!("✓ 成功打开 Git 仓库");

    // 获取当前分支
    if let Some(branch) = repo.get_current_branch()? {
        println!("✓ 当前分支: {}", branch);
    } else {
        println!("⚠ HEAD 处于 detached 状态");
    }

    // 检查是否有 staged changes
    let has_staged = repo.has_staged_changes()?;
    println!("✓ Staged changes: {}", if has_staged { "是" } else { "否" });

    if has_staged {
        // 获取 staged diff
        let diff = repo.get_staged_diff()?;
        println!("\n--- Staged Diff (前 500 字符) ---");
        println!("{}", &diff[..diff.len().min(500)]);

        // 获取统计信息
        let stats = repo.get_diff_stats(&diff)?;
        println!("\n--- Diff 统计 ---");
        println!("文件变更: {:?}", stats.files_changed);
        println!("插入行数: {}", stats.insertions);
        println!("删除行数: {}", stats.deletions);
    }

    println!("\n✓ 所有测试通过！");
    Ok(())
}
