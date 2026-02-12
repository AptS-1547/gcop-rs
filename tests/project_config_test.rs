//! 项目级配置集成测试
//!
//! 测试 .gcop/config.toml 项目配置的完整流程

use gcop_rs::config::{AppConfig, CommitConvention, ConventionStyle};
use gcop_rs::llm::CommitContext;
use gcop_rs::llm::prompt::build_commit_prompt_split;

// === Convention 从配置到 Prompt 的端到端测试 ===

/// 测试 conventional 风格 convention 注入到 prompt 的完整链路
#[test]
fn test_convention_conventional_e2e() {
    // 模拟从配置加载的 convention
    let convention = CommitConvention {
        style: ConventionStyle::Conventional,
        types: Some(vec![
            "feat".to_string(),
            "fix".to_string(),
            "docs".to_string(),
            "refactor".to_string(),
        ]),
        template: None,
        extra_prompt: Some("All commit messages must be in English".to_string()),
    };

    let context = CommitContext {
        files_changed: vec!["src/lib.rs".to_string(), "src/main.rs".to_string()],
        insertions: 15,
        deletions: 3,
        branch_name: Some("feature/auth".to_string()),
        custom_prompt: None,
        user_feedback: vec![],
        convention: Some(convention),
        scope_info: None,
    };

    let diff = "diff --git a/src/lib.rs b/src/lib.rs\n+pub fn authenticate() {}";

    let (system, user) =
        build_commit_prompt_split(diff, &context, None, context.convention.as_ref());

    // system prompt 应包含默认规则 + convention 约束
    assert!(system.contains("git commit message generator"));
    assert!(system.contains("## Convention:"));
    assert!(system.contains("conventional commits format"));
    assert!(system.contains("Allowed types: feat, fix, docs, refactor"));
    assert!(system.contains("All commit messages must be in English"));

    // user message 应包含 diff 和 context
    assert!(user.contains("src/lib.rs"));
    assert!(user.contains("Branch: feature/auth"));
    assert!(user.contains("+15 -3"));
}

/// 测试 gitmoji 风格 convention 端到端
#[test]
fn test_convention_gitmoji_e2e() {
    let convention = CommitConvention {
        style: ConventionStyle::Gitmoji,
        types: None,
        template: None,
        extra_prompt: None,
    };

    let context = CommitContext {
        files_changed: vec!["README.md".to_string()],
        insertions: 5,
        deletions: 0,
        branch_name: None,
        custom_prompt: None,
        user_feedback: vec![],
        convention: Some(convention),
        scope_info: None,
    };

    let (system, _) =
        build_commit_prompt_split("diff", &context, None, context.convention.as_ref());

    assert!(system.contains("gitmoji"));
    assert!(!system.contains("Allowed types"));
}

/// 测试 custom 风格 + template 端到端
#[test]
fn test_convention_custom_with_template_e2e() {
    let convention = CommitConvention {
        style: ConventionStyle::Custom,
        types: Some(vec!["feature".to_string(), "bugfix".to_string()]),
        template: Some("[{type}] {subject}".to_string()),
        extra_prompt: Some("Use imperative mood".to_string()),
    };

    let context = CommitContext {
        files_changed: vec!["app.rs".to_string()],
        insertions: 1,
        deletions: 1,
        branch_name: None,
        custom_prompt: None,
        user_feedback: vec![],
        convention: Some(convention),
        scope_info: None,
    };

    let (system, _) =
        build_commit_prompt_split("diff", &context, None, context.convention.as_ref());

    // Custom 风格不注入 "Follow conventional commits format" 或 "gitmoji" 到 Convention 段
    let convention_section = system.split("## Convention:").nth(1).unwrap();
    assert!(!convention_section.contains("conventional commits format"));
    assert!(!convention_section.contains("gitmoji"));
    // 但应包含 types、template、extra_prompt
    assert!(system.contains("Allowed types: feature, bugfix"));
    assert!(system.contains("Commit template: [{type}] {subject}"));
    assert!(system.contains("Use imperative mood"));
}

/// 测试 convention + custom_prompt 共存
/// custom_prompt 替换默认 system prompt，convention 追加在后面
#[test]
fn test_convention_with_custom_prompt_e2e() {
    let convention = CommitConvention {
        style: ConventionStyle::Conventional,
        types: Some(vec!["feat".to_string(), "fix".to_string()]),
        ..Default::default()
    };

    let context = CommitContext {
        files_changed: vec!["a.rs".to_string()],
        insertions: 1,
        deletions: 0,
        branch_name: None,
        custom_prompt: Some("You are a minimal commit message generator.".to_string()),
        user_feedback: vec![],
        convention: Some(convention),
        scope_info: None,
    };

    let (system, _) = build_commit_prompt_split(
        "diff",
        &context,
        context.custom_prompt.as_deref(),
        context.convention.as_ref(),
    );

    // custom_prompt 替换默认 system prompt
    assert!(system.starts_with("You are a minimal commit message generator."));
    // convention 追加在后面
    assert!(system.contains("## Convention:"));
    assert!(system.contains("Allowed types: feat, fix"));
}

/// 测试 convention + user_feedback 共存
#[test]
fn test_convention_with_feedback_e2e() {
    let convention = CommitConvention {
        style: ConventionStyle::Conventional,
        ..Default::default()
    };

    let context = CommitContext {
        files_changed: vec!["a.rs".to_string()],
        insertions: 1,
        deletions: 0,
        branch_name: None,
        custom_prompt: None,
        user_feedback: vec!["请使用中文".to_string()],
        convention: Some(convention),
        scope_info: None,
    };

    let (system, user) =
        build_commit_prompt_split("diff", &context, None, context.convention.as_ref());

    // convention 在 system prompt
    assert!(system.contains("## Convention:"));
    // feedback 在 user message
    assert!(user.contains("User Requirements"));
    assert!(user.contains("1. 请使用中文"));
}

/// 测试无 convention 时 prompt 不包含 Convention 段
#[test]
fn test_no_convention_no_section_e2e() {
    let context = CommitContext {
        files_changed: vec!["a.rs".to_string()],
        insertions: 1,
        deletions: 0,
        branch_name: None,
        custom_prompt: None,
        user_feedback: vec![],
        convention: None,
        scope_info: None,
    };

    let (system, _) = build_commit_prompt_split("diff", &context, None, None);

    assert!(!system.contains("## Convention:"));
}

// === AppConfig convention 集成测试 ===

/// 测试 AppConfig 默认无 convention
#[test]
fn test_app_config_default_no_convention() {
    let config = AppConfig::default();
    assert!(config.commit.convention.is_none());
}

/// 测试 CommitConvention 默认值
#[test]
fn test_commit_convention_defaults() {
    let conv = CommitConvention::default();
    assert_eq!(conv.style, ConventionStyle::Conventional);
    assert!(conv.types.is_none());
    assert!(conv.template.is_none());
    assert!(conv.extra_prompt.is_none());
}
