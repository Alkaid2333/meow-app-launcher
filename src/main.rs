//! 喵喵应用启动器喵~ (meowal)
//!
//! 类 macOS 聚焦搜索的跨平台高性能应用启动器喵!
//! 技术栈: Rust + Skia(自绘图形引擎) + Win32(平台层)喵。
//!
//! 架构: 多窗口 + 托盘——
//! * 启动器窗口(灵动岛搜索框 + 结果面板)喵
//! * 配置窗口(侧边栏 + 分组卡片设置界面)喵
//! * 系统托盘(显示/隐藏、设置、重启、退出)喵
//!
//! 共享状态用 `Rc<RefCell<AppState>>`,组件间通过命令队列协调喵。

mod app;
mod animation;
mod apps;
mod platform;
mod render;
mod search;
mod utils;
mod window;

use app::{AppState, SharedState};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use window::{Launcher, SettingsWindow, Tray};

fn main() {
    // 数据目录: ./.datas 喵
    let data_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(app::config::DATA_DIR_NAME);

    // 日志喵(MEOWAL_VERBOSE=1 开启 debug 日志)喵
    utils::logger::init(&data_dir, std::env::var("MEOWAL_VERBOSE").is_ok());
    log::info!("喵喵启动器启动喵! 版本 {}", env!("CARGO_PKG_VERSION"));

    let platform = platform::platform();
    let state: SharedState = Rc::new(RefCell::new(AppState::new(platform.clone(), data_dir)));
    let commands = Rc::new(RefCell::new(VecDeque::new()));

    // 1. 创建配置窗口(初始隐藏)喵
    let settings_window = SettingsWindow::spawn(platform.clone(), state.clone(), commands.clone());

    // 2. 创建启动器窗口(兼任应用控制器)喵
    let launcher_window =
        Launcher::spawn(platform.clone(), state.clone(), commands.clone(), settings_window);

    // 3. 创建系统托盘喵
    let tray = Tray::spawn(platform.clone(), state.clone(), commands.clone());

    // 4. 注册全局热键喵
    let hotkey = state.borrow().config.hotkey.clone();
    if hotkey.enabled {
        let ok = platform.register_global_hotkey(&hotkey.modifiers, &hotkey.key, launcher_window);
        if !ok {
            log::warn!("全局热键注册失败喵~");
        }
    } else {
        log::info!("全局热键已禁用喵~");
    }

    // 5. 运行全局消息循环喵
    platform.run();

    // 6. 清理资源喵
    log::info!("应用退出,清理资源喵~");
    platform.unregister_global_hotkey();
    platform.destroy_tray(&tray);
    platform.destroy_window(&launcher_window);
    platform.destroy_window(&settings_window);
}
