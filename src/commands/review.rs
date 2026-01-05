use crate::cli::{Cli, ReviewTarget};
use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::git::{GitOperations, repository::GitRepository};
use crate::llm::{IssueSeverity, LLMProvider, ReviewResult, ReviewType, provider::create_provider};
use crate::ui;

/// 执行 review 命令（公开接口）
pub async fn run(cli: &Cli, config: &AppConfig, target: &ReviewTarget, format: &str) -> Result<()> {
    let repo = GitRepository::open(Some(&config.file))?;
    let provider = create_provider(config, cli.provider.as_deref())?;
    run_internal(config, target, format, &repo, provider.as_ref()).await
}

/// 内部实现，接受依赖注入（用于测试）
#[cfg_attr(not(feature = "test-utils"), allow(dead_code))]
pub async fn run_internal(
    config: &AppConfig,
    target: &ReviewTarget,
    format: &str,
    git: &dyn GitOperations,
    llm: &dyn LLMProvider,
) -> Result<()> {
    let colored = config.ui.colored;

    // 根据目标类型路由
    let (diff, description) = match target {
        ReviewTarget::Changes => {
            ui::step("1/3", "Analyzing uncommitted changes...", colored);
            let diff = git.get_uncommitted_diff()?;
            if diff.trim().is_empty() {
                ui::error("No uncommitted changes found.", colored);
                return Err(GcopError::InvalidInput(
                    "No uncommitted changes to review".to_string(),
                ));
            }
            (diff, "Uncommitted changes".to_string())
        }
        ReviewTarget::Commit { hash } => {
            ui::step("1/3", &format!("Analyzing commit {}...", hash), colored);
            let diff = git.get_commit_diff(hash)?;
            (diff, format!("Commit {}", hash))
        }
        ReviewTarget::Range { range } => {
            ui::step("1/3", &format!("Analyzing range {}...", range), colored);
            let diff = git.get_range_diff(range)?;
            (diff, format!("Commit range {}", range))
        }
        ReviewTarget::File { path } => {
            ui::step("1/3", &format!("Analyzing file {}...", path), colored);
            let content = git.get_file_content(path)?;
            // 文件审查需要特殊处理，将内容包装成 diff 格式
            let diff = format!("--- {}\n+++ {}\n{}", path, path, content);
            (diff, format!("File {}", path))
        }
    };

    // 调用 LLM 进行审查
    let spinner = ui::Spinner::new("Reviewing code with AI...", colored);

    let review_type = match target {
        ReviewTarget::Changes => ReviewType::UncommittedChanges,
        ReviewTarget::Commit { hash } => ReviewType::SingleCommit(hash.clone()),
        ReviewTarget::Range { range } => ReviewType::CommitRange(range.clone()),
        ReviewTarget::File { path } => ReviewType::FileOrDir(path.clone()),
    };

    let result = llm
        .review_code(
            &diff,
            review_type,
            config.review.custom_prompt.as_deref(),
            Some(&spinner),
        )
        .await?;

    spinner.finish_and_clear();

    // 格式化输出
    ui::step("3/3", "Formatting results...", colored);
    println!();

    match format {
        "json" => print_json(&result)?,
        "markdown" => print_markdown(&result, &description, colored),
        _ => print_text(&result, &description, config),
    }

    Ok(())
}

/// 以文本格式输出审查结果
fn print_text(result: &ReviewResult, description: &str, config: &AppConfig) {
    let colored = config.ui.colored;

    println!("{}", ui::info(&format!("Review: {}", description), colored));
    println!();

    // 输出摘要
    println!("📝 Summary:");
    println!("{}", result.summary);
    println!();

    // 输出问题
    if !result.issues.is_empty() {
        println!("🔍 Issues found:");
        println!();

        for (i, issue) in result.issues.iter().enumerate() {
            // 根据配置过滤严重性
            let min_severity = match config.review.min_severity.as_str() {
                "critical" => IssueSeverity::Critical,
                "warning" => IssueSeverity::Warning,
                _ => IssueSeverity::Info,
            };

            // 跳过低于最小严重性的问题
            let issue_level = match issue.severity {
                IssueSeverity::Critical => 0,
                IssueSeverity::Warning => 1,
                IssueSeverity::Info => 2,
            };

            let min_level = match min_severity {
                IssueSeverity::Critical => 0,
                IssueSeverity::Warning => 1,
                IssueSeverity::Info => 2,
            };

            if issue_level > min_level {
                continue;
            }

            // 格式化严重性标签
            let severity_label = match issue.severity {
                IssueSeverity::Critical => {
                    if colored {
                        "CRITICAL".to_string()
                    } else {
                        "[CRITICAL]".to_string()
                    }
                }
                IssueSeverity::Warning => {
                    if colored {
                        "WARNING".to_string()
                    } else {
                        "[WARNING]".to_string()
                    }
                }
                IssueSeverity::Info => {
                    if colored {
                        "INFO".to_string()
                    } else {
                        "[INFO]".to_string()
                    }
                }
            };

            // 输出问题
            print!("  {}. ", i + 1);

            if colored {
                use colored::Colorize;
                match issue.severity {
                    IssueSeverity::Critical => print!("{}", severity_label.red().bold()),
                    IssueSeverity::Warning => print!("{}", severity_label.yellow().bold()),
                    IssueSeverity::Info => print!("{}", severity_label.blue().bold()),
                }
            } else {
                print!("{}", severity_label);
            }

            println!(" {}", issue.description);

            // 输出位置信息
            if let Some(file) = &issue.file {
                if let Some(line) = issue.line {
                    println!("     Location: {}:{}", file, line);
                } else {
                    println!("     Location: {}", file);
                }
            }
            println!();
        }
    } else {
        println!("✨ No issues found!");
        println!();
    }

    // 输出建议
    if !result.suggestions.is_empty() {
        println!("💡 Suggestions:");
        println!();
        for suggestion in &result.suggestions {
            println!("  • {}", suggestion);
        }
        println!();
    }
}

/// 以 JSON 格式输出审查结果
fn print_json(result: &ReviewResult) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    println!("{}", json);
    Ok(())
}

/// 以 Markdown 格式输出审查结果
fn print_markdown(result: &ReviewResult, description: &str, _colored: bool) {
    println!("# Code Review: {}", description);
    println!();

    // 摘要
    println!("## Summary");
    println!();
    println!("{}", result.summary);
    println!();

    // 问题
    if !result.issues.is_empty() {
        println!("## Issues");
        println!();

        for issue in &result.issues {
            let severity_emoji = match issue.severity {
                IssueSeverity::Critical => "🔴",
                IssueSeverity::Warning => "🟡",
                IssueSeverity::Info => "🔵",
            };

            let severity_text = match issue.severity {
                IssueSeverity::Critical => "**CRITICAL**",
                IssueSeverity::Warning => "**WARNING**",
                IssueSeverity::Info => "**INFO**",
            };

            println!("### {} {}", severity_emoji, severity_text);
            println!();
            println!("{}", issue.description);
            println!();

            if let Some(file) = &issue.file {
                if let Some(line) = issue.line {
                    println!("**Location:** `{}:{}`", file, line);
                } else {
                    println!("**Location:** `{}`", file);
                }
                println!();
            }
        }
    } else {
        println!("## Issues");
        println!();
        println!("✨ No issues found!");
        println!();
    }

    // 建议
    if !result.suggestions.is_empty() {
        println!("## Suggestions");
        println!();
        for suggestion in &result.suggestions {
            println!("- {}", suggestion);
        }
        println!();
    }
}
