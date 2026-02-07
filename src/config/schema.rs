//! 配置结构定义（兼容性别名）
//!
//! **已弃用**: 此文件仅作为向后兼容的别名。
//! 请直接使用 `config::structs` 模块或顶层重导出。
//!
//! 此别名将在 1.0.0 版本中移除。

#[deprecated(
    since = "0.3.0",
    note = "Use config::structs or config::<Type> directly. This module will be removed in 1.0.0."
)]
pub use super::structs::*;
