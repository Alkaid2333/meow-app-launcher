//! 喵喵应用启动器核心库喵~
//!
//! 供二进制入口(`main.rs`)与集成测试(`tests/`)共享的模块集合喵。
//! 业务模块一律通过 `meow_app_launcher::...` 访问,测试代码与主代码分离喵。

pub mod app;
pub mod animation;
pub mod apps;
pub mod cli;
pub mod platform;
pub mod render;
pub mod search;
pub mod utils;
pub mod window;
