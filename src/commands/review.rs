use super::options::ReviewOptions;
use super::truncate_diff;
use crate::cli::ReviewTarget;
use crate::commands::json::{self, JsonOutput};
use crate::config::AppConfig;
use crate::error::{GcopError, Result};
use crate::git::{GitOperations, repository::GitRepository};
use crate::llm::{
    IssueSeverity, LLMProvider, ProgressReporter, ReviewResult, ReviewType,
    provider::create_provider,
};
use crate::ui;

/// 执行 review 命令（公开接口）
pub async fn run(options: &ReviewOptions<'_>, config: &AppConfig) -> Result<()> {
    let repo = GitRepository::open(Some(&config.file))?;
    let provider = create_provider(config, options.provider_override)?;
    let result = run_internal(options, config, &repo, provider.as_ref()).await;
    if let Err(ref e) = result
        && options.format.is_json()
    {
        let _ = json::output_json_error::<ReviewResult>(e);
    }
    result
}

/// 内部实现，接受依赖注入（用于测试）
#[cfg_attr(not(feature = "test-utils"), allow(dead_code))]
pub async fn run_internal(
    options: &ReviewOptions<'_>,
    config: &AppConfig,
    git: &dyn GitOperations,
    llm: &dyn LLMProvider,
) -> Result<()> {
    let skip_ui = options.format.is_machine_readable();
    let colored = options.effective_colored(config);

    // 根据目标类型路由
    let (diff, description) = match options.target {
        ReviewTarget::Changes => {
            if !skip_ui {
                ui::step(
                    &rust_i18n::t!("review.step1"),
                    &rust_i18n::t!("review.analyzing_changes"),
                    colored,
                );
            }
            let diff = git.get_uncommitted_diff()?;
            if diff.trim().is_empty() {
                if !skip_ui {
                    ui::error(&rust_i18n::t!("review.no_changes"), colored);
                }
                return Err(GcopError::InvalidInput(
                    rust_i18n::t!("review.no_uncommitted_changes_to_review").to_string(),
                ));
            }
            (
                diff,
                rust_i18n::t!("review.description.uncommitted").to_string(),
            )
        }
        ReviewTarget::Commit { hash } => {
            if !skip_ui {
                ui::step(
                    &rust_i18n::t!("review.step1"),
                    &rust_i18n::t!("review.analyzing_commit", hash = hash),
                    colored,
                );
            }
            let diff = git.get_commit_diff(hash)?;
            (
                diff,
                rust_i18n::t!("review.description.commit", hash = hash).to_string(),
            )
        }
        ReviewTarget::Range { range } => {
            if !skip_ui {
                ui::step(
                    &rust_i18n::t!("review.step1"),
                    &rust_i18n::t!("review.analyzing_range", range = range),
                    colored,
                );
            }
            let diff = git.get_range_diff(range)?;
            (
                diff,
                rust_i18n::t!("review.description.range", range = range).to_string(),
            )
        }
        ReviewTarget::File { path } => {
            if !skip_ui {
                ui::step(
                    &rust_i18n::t!("review.step1"),
                    &rust_i18n::t!("review.analyzing_file", path = path),
                    colored,
                );
            }
            let content = git.get_file_content(path)?;
            // 文件审查需要特殊处理，将内容包装成 diff 格式
            let diff = format!("--- {}\n+++ {}\n{}", path, path, content);
            (
                diff,
                rust_i18n::t!("review.description.file", path = path).to_string(),
            )
        }
    };

    // 调用 LLM 进行审查（截断过大的 diff）
    let (diff, truncated) = truncate_diff(&diff, config.llm.max_diff_size);
    if truncated && !skip_ui {
        ui::warning(&rust_i18n::t!("diff.truncated"), colored);
    }
    let review_type = match options.target {
        ReviewTarget::Changes => ReviewType::UncommittedChanges,
        ReviewTarget::Commit { hash } => ReviewType::SingleCommit(hash.clone()),
        ReviewTarget::Range { range } => ReviewType::CommitRange(range.clone()),
        ReviewTarget::File { path } => ReviewType::FileOrDir(path.clone()),
    };

    // 机器可读格式不显示 spinner
    let spinner = if skip_ui {
        None
    } else {
        Some(ui::Spinner::new(
            &rust_i18n::t!("spinner.reviewing"),
            colored,
        ))
    };

    let result = llm
        .review_code(
            &diff,
            review_type,
            config.review.custom_prompt.as_deref(),
            spinner.as_ref().map(|s| s as &dyn ProgressReporter),
        )
        .await?;

    if let Some(s) = spinner {
        s.finish_and_clear();
    }

    // 格式化输出
    if !skip_ui {
        ui::step(
            &rust_i18n::t!("review.step3"),
            &rust_i18n::t!("review.formatting"),
            colored,
        );
        println!();
    }

    match options.format {
        super::format::OutputFormat::Json => print_json(&result)?,
        super::format::OutputFormat::Markdown => print_markdown(&result, &description, colored),
        super::format::OutputFormat::Text => print_text(&result, &description, config),
    }

    Ok(())
}

/// 以文本格式输出审查结果
fn print_text(result: &ReviewResult, description: &str, config: &AppConfig) {
    let colored = config.ui.colored;

    println!(
        "{}",
        ui::info(
            &rust_i18n::t!("review.title", description = description),
            colored
        )
    );
    println!();

    // 输出摘要
    println!("{}", rust_i18n::t!("review.summary_title"));
    println!("{}", result.summary);
    println!();

    // 输出问题
    if !result.issues.is_empty() {
        println!("{}", rust_i18n::t!("review.issues_found"));
        println!();

        for (i, issue) in result.issues.iter().enumerate() {
            // 根据配置过滤严重性
            let min_severity = IssueSeverity::from_config_str(&config.review.min_severity);

            // 跳过低于最小严重性的问题
            if issue.severity.level() > min_severity.level() {
                continue;
            }

            // 输出问题
            print!("  {}. ", i + 1);

            if colored {
                print!("{}", issue.severity.colored_label());
            } else {
                print!("{}", issue.severity.label(false));
            }

            println!(" {}", issue.description);

            // 输出位置信息
            if let Some(file) = &issue.file {
                if let Some(line) = issue.line {
                    println!(
                        "     {}",
                        rust_i18n::t!("review.location.with_line", file = file, line = line)
                    );
                } else {
                    println!(
                        "     {}",
                        rust_i18n::t!("review.location.file_only", file = file)
                    );
                }
            }
            println!();
        }
    } else {
        println!("{}", rust_i18n::t!("review.no_issues"));
        println!();
    }

    // 输出建议
    if !result.suggestions.is_empty() {
        println!("{}", rust_i18n::t!("review.suggestions_title"));
        println!();
        for suggestion in &result.suggestions {
            println!("  • {}", suggestion);
        }
        println!();
    }
}

/// 以 JSON 格式输出审查结果
fn print_json(result: &ReviewResult) -> Result<()> {
    let output = JsonOutput {
        success: true,
        data: Some(result.clone()),
        error: None,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// 以 Markdown 格式输出审查结果
fn print_markdown(result: &ReviewResult, description: &str, _colored: bool) {
    println!(
        "{}",
        rust_i18n::t!("review.md.title", description = description)
    );
    println!();

    // 摘要
    println!("{}", rust_i18n::t!("review.md.summary"));
    println!();
    println!("{}", result.summary);
    println!();

    // 问题
    if !result.issues.is_empty() {
        println!("{}", rust_i18n::t!("review.md.issues"));
        println!();

        for issue in &result.issues {
            let severity_emoji = match issue.severity {
                IssueSeverity::Critical => "🔴",
                IssueSeverity::Warning => "🟡",
                IssueSeverity::Info => "🔵",
            };

            let severity_text = match issue.severity {
                IssueSeverity::Critical => rust_i18n::t!("review.md.severity_critical"),
                IssueSeverity::Warning => rust_i18n::t!("review.md.severity_warning"),
                IssueSeverity::Info => rust_i18n::t!("review.md.severity_info"),
            };

            println!("### {} {}", severity_emoji, severity_text);
            println!();
            println!("{}", issue.description);
            println!();

            if let Some(file) = &issue.file {
                if let Some(line) = issue.line {
                    println!(
                        "{}",
                        rust_i18n::t!(
                            "review.md.location",
                            location = format!("{}:{}", file, line)
                        )
                    );
                } else {
                    println!("{}", rust_i18n::t!("review.md.location", location = file));
                }
                println!();
            }
        }
    } else {
        println!("{}", rust_i18n::t!("review.md.no_issues_title"));
        println!();
        println!("{}", rust_i18n::t!("review.md.no_issues"));
        println!();
    }

    // 建议
    if !result.suggestions.is_empty() {
        println!("{}", rust_i18n::t!("review.md.suggestions"));
        println!();
        for suggestion in &result.suggestions {
            println!("- {}", suggestion);
        }
        println!();
    }
}
