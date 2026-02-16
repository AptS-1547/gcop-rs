use crate::config;
use crate::error::{GcopError, Result};
use crate::git::find_git_root;
use crate::ui;
use std::fs;

/// Initialization configuration file
pub fn run(force: bool, project: bool, colored: bool) -> Result<()> {
    if project {
        run_project_init(force, colored)
    } else {
        run_user_init(force, colored)
    }
}

/// Initialize user-level configuration file
fn run_user_init(force: bool, colored: bool) -> Result<()> {
    // 1. Get the configuration directory and file path
    let config_dir = config::get_config_dir().ok_or_else(|| {
        GcopError::Config(rust_i18n::t!("config.failed_determine_dir").to_string())
    })?;

    let config_file = config_dir.join("config.toml");

    // 2. Check if the configuration file already exists
    if config_file.exists() && !force {
        ui::warning(
            &rust_i18n::t!("init.exists", path = config_file.display()),
            colored,
        );
        println!();
        println!("{}", rust_i18n::t!("init.use_force"));
        println!("{}", rust_i18n::t!("init.config_edit"));
        return Ok(());
    }

    // 3. Create configuration directory
    fs::create_dir_all(&config_dir)?;
    ui::success(
        &rust_i18n::t!("init.dir_created", path = config_dir.display()),
        colored,
    );

    // 4. Copy the sample configuration
    let example_config = include_str!("../../examples/config.toml.example");
    fs::write(&config_file, example_config)?;
    ui::success(
        &rust_i18n::t!("init.file_created", path = config_file.display()),
        colored,
    );

    // 5. Set file permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&config_file)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&config_file, perms)?;
        ui::success(&rust_i18n::t!("init.permissions"), colored);
    }

    // 6. Display next step prompt
    println!();
    println!("{}", ui::info(&rust_i18n::t!("init.next_steps"), colored));
    println!("{}", rust_i18n::t!("init.step1"));
    println!("{}", rust_i18n::t!("init.step1_cmd"));
    println!();
    println!("{}", rust_i18n::t!("init.step2"));
    println!("{}", rust_i18n::t!("init.step2_url"));
    println!();

    // 7. Ask whether to install git aliases
    let install_aliases = ui::confirm(&rust_i18n::t!("init.install_aliases"), true)?;

    if install_aliases {
        println!();
        match crate::commands::alias::install_all(force, colored) {
            Ok(_) => {}
            Err(e) => {
                ui::warning(
                    &rust_i18n::t!("init.alias_failed", error = e.to_string()),
                    colored,
                );
                println!();
                println!("{}", rust_i18n::t!("init.alias_later"));
                println!("{}", rust_i18n::t!("init.alias_cmd"));
            }
        }
    } else {
        println!();
        println!("{}", rust_i18n::t!("init.alias_skipped"));
        println!("{}", rust_i18n::t!("init.alias_run_later"));
    }

    println!();
    println!("{}", rust_i18n::t!("init.docs"));

    Ok(())
}

/// Initialize the project-level configuration file (.gcop/config.toml)
///
/// If the current directory is not in the Git repository, return to the current working directory to create `.gcop/config.toml` and give a prompt.
fn run_project_init(force: bool, colored: bool) -> Result<()> {
    // 1. Find the git repo root directory
    let repo_root = match find_git_root() {
        Some(root) => root,
        None => {
            ui::warning(&rust_i18n::t!("init.project_not_git_repo"), colored);
            std::env::current_dir()?
        }
    };

    let gcop_dir = repo_root.join(".gcop");
    let config_file = gcop_dir.join("config.toml");

    // 2. Check if it already exists
    if config_file.exists() && !force {
        ui::warning(
            &rust_i18n::t!("init.project_exists", path = config_file.display()),
            colored,
        );
        println!();
        println!("{}", rust_i18n::t!("init.use_force"));
        return Ok(());
    }

    // 3. Create the .gcop/ directory
    fs::create_dir_all(&gcop_dir)?;
    ui::success(
        &rust_i18n::t!("init.project_dir_created", path = gcop_dir.display()),
        colored,
    );

    // 4. Write project-level templates
    let project_config = include_str!("../../examples/project-config.toml.example");
    fs::write(&config_file, project_config)?;
    ui::success(
        &rust_i18n::t!("init.project_created", path = config_file.display()),
        colored,
    );

    // 5. Prompt for next step
    println!();
    println!(
        "{}",
        ui::info(&rust_i18n::t!("init.project_next_steps"), colored)
    );
    println!("{}", rust_i18n::t!("init.project_step1"));
    println!("{}", rust_i18n::t!("init.project_step2"));

    Ok(())
}
