//! 喵喵应用启动器喵~ (meowal)
//!
//! 类 macOS 聚焦搜索的跨平台高性能应用启动器喵!
//! 技术栈: Rust + Skia(自绘图形引擎) + Win32(平台层)喵。
//!
//! 架构: 单窗口灵动岛——一个透明异形置顶窗口,内部用 Skia 绘制
//! 搜索框胶囊 + 结果面板,面板随结果有无而展开/折叠喵。

mod app;
mod animation;
mod apps;
mod platform;
mod render;
mod search;
mod utils;
mod window;

use std::path::PathBuf;
use window::Launcher;

fn main() {
    // 数据目录: ./.datas 喵
    let data_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(app::config::DATA_DIR_NAME);

    // 日志喵(MEOWAL_VERBOSE=1 开启 debug 日志)喵
    utils::logger::init(&data_dir, std::env::var("MEOWAL_VERBOSE").is_ok());
    log::info!("喵喵启动器启动喵! 版本 {}", env!("CARGO_PKG_VERSION"));

    // 装配并运行启动器喵
    let platform = platform::platform();
    let mut launcher = Launcher::new(platform, data_dir);
    launcher.run();
}
