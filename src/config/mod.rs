// 配置模块
//
// 此模块负责应用配置的加载、管理和访问。

// 子模块声明
mod global;
mod loader;
mod structs;

#[cfg(test)]
mod tests;

// 公开 API
pub use global::{get_config, init_config};
pub use loader::{get_config_dir, load_config};
pub use structs::{
    ApiStyle, AppConfig, CommitConfig, FileConfig, LLMConfig, NetworkConfig, ProviderConfig,
    ReviewConfig, UIConfig,
};
