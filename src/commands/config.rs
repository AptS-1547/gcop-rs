use crate::config::{self, load_config};
use crate::error::{GcopError, Result};
use crate::llm::provider::create_provider;
use crate::ui;
use colored::Colorize;
use dialoguer::Select;

/// User-optional operations after editing
enum EditAction {
    Retry,  // Re-edit
    Keep,   // Keep the original configuration (do not modify it)
    Ignore, // Ignore errors and force save
}

/// Runs the `config` command with either edit or validate behavior.
pub async fn run(action: Option<crate::cli::ConfigAction>, colored: bool) -> Result<()> {
    // Default behavior: call edit
    let action = action.unwrap_or(crate::cli::ConfigAction::Edit);

    match action {
        crate::cli::ConfigAction::Edit => edit(colored),
        crate::cli::ConfigAction::Validate => validate(colored).await,
    }
}

/// Open the editor to edit the configuration file (with verification)
fn edit(colored: bool) -> Result<()> {
    let config_dir = config::get_config_dir().ok_or_else(|| {
        GcopError::Config(rust_i18n::t!("config.failed_determine_dir").to_string())
    })?;

    let config_file = config_dir.join("config.toml");

    // If the configuration file does not exist, prompt to run init
    if !config_file.exists() {
        ui::error(&rust_i18n::t!("config.file_not_found"), colored);
        println!();
        println!("{}", rust_i18n::t!("config.run_init"));
        println!("  mkdir -p {}", config_dir.display());
        println!(
            "  cp examples/config.toml.example {}",
            config_file.display()
        );
        return Err(GcopError::Config(
            rust_i18n::t!("config.file_not_found").to_string(),
        ));
    }

    // Initial reading of configuration content
    let mut content = std::fs::read_to_string(&config_file)?;

    // Edit-Verify Loop
    loop {
        println!(
            "{}",
            ui::info(
                &rust_i18n::t!("config.editing", path = config_file.display().to_string()),
                colored
            )
        );

        // Open the editor using the edit crate (automatic fallback: $VISUAL > $EDITOR > platform preset list)
        let edited = edit::edit(&content)?;

        // Verify configuration (deserialized through config crate, path consistent with load_config)
        let validation: std::result::Result<crate::config::AppConfig, _> =
            ::config::Config::builder()
                .add_source(::config::File::from_str(
                    &edited,
                    ::config::FileFormat::Toml,
                ))
                .build()
                .and_then(|c| c.try_deserialize());
        match validation {
            Ok(_) => {
                // Verification successful, write to file
                std::fs::write(&config_file, &edited)?;
                ui::success(&rust_i18n::t!("config.file_updated"), colored);
                return Ok(());
            }
            Err(e) => {
                // Verification failed
                println!();
                ui::error(
                    &rust_i18n::t!("config.validation_failed", error = e.to_string()),
                    colored,
                );
                println!();

                match prompt_edit_action(colored)? {
                    EditAction::Retry => {
                        // Keep the edited content and continue editing
                        content = edited;
                        continue;
                    }
                    EditAction::Keep => {
                        // The original file has never been modified and is returned directly.
                        println!("{}", ui::info(&rust_i18n::t!("config.unchanged"), colored));
                        return Ok(());
                    }
                    EditAction::Ignore => {
                        // Force saving of incorrect configuration
                        std::fs::write(&config_file, &edited)?;
                        ui::warning(&rust_i18n::t!("config.saved_with_errors"), colored);
                        return Ok(());
                    }
                }
            }
        }
    }
}

/// Prompt user to select an action
fn prompt_edit_action(colored: bool) -> Result<EditAction> {
    let items: Vec<String> = if colored {
        vec![
            format!("{}", rust_i18n::t!("config.action_reedit").yellow()),
            format!("{}", rust_i18n::t!("config.action_keep").blue()),
            format!("{}", rust_i18n::t!("config.action_ignore").red()),
        ]
    } else {
        vec![
            rust_i18n::t!("config.action_reedit").to_string(),
            rust_i18n::t!("config.action_keep").to_string(),
            rust_i18n::t!("config.action_ignore").to_string(),
        ]
    };

    let prompt = if colored {
        format!("{}", rust_i18n::t!("config.action_prompt").cyan().bold())
    } else {
        rust_i18n::t!("config.action_prompt").to_string()
    };

    let selection = Select::new()
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| {
            GcopError::Other(
                rust_i18n::t!("config.input_failed", error = e.to_string()).to_string(),
            )
        })?;

    Ok(match selection {
        0 => EditAction::Retry,
        1 => EditAction::Keep,
        _ => EditAction::Ignore,
    })
}

/// Verify configuration
async fn validate(colored: bool) -> Result<()> {
    ui::step("1/2", &rust_i18n::t!("config.loading"), colored);

    // Load configuration
    let config = load_config()?;

    ui::success(&rust_i18n::t!("config.loaded"), colored);
    println!();

    // Show configured providers
    println!("{}", rust_i18n::t!("config.providers"));
    for name in config.llm.providers.keys() {
        println!("  • {}", name);
    }
    println!();

    // Verify provider chain availability (default provider + fallback providers)
    ui::step("2/2", &rust_i18n::t!("config.testing"), colored);

    let provider = create_provider(&config, None)?;

    match provider.validate().await {
        Ok(_) => {
            ui::success(
                &rust_i18n::t!("config.validated", provider = config.llm.default_provider),
                colored,
            );
        }
        Err(e) => {
            ui::error(
                &rust_i18n::t!("config.validation_failed_short", error = e.to_string()),
                colored,
            );
            if let Some(suggestion) = e.localized_suggestion() {
                println!();
                println!(
                    "{}",
                    rust_i18n::t!("config.suggestion", suggestion = suggestion)
                );
            }
            return Err(e);
        }
    }

    Ok(())
}
