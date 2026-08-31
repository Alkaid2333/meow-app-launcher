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
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("register") {
        cli_register(&args[1..]);
        return;
    }

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

/// `meowal register <名称> <路径> [-ico 图标]` 喵
fn cli_register(args: &[String]) {
    let data_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(app::config::DATA_DIR_NAME);
    let _ = std::fs::create_dir_all(&data_dir);

    let parsed = parse_register_args(args);
    let Some((name, path, icon)) = parsed else {
        eprintln!("用法: meowal register <应用名称> <可执行路径> [-ico 图标路径]");
        std::process::exit(1);
    };
    if !std::path::Path::new(&path).exists() {
        eprintln!("路径不存在: {path}");
        std::process::exit(1);
    }
    let mut registry = apps::AppRegistry::load(&data_dir);
    let mut app_info = apps::AppInfo::manual(&name, &path);
    app_info.icon_path = icon;
    registry.upsert(app_info);
    registry.save(&data_dir);
    println!("已注册: {name} → {path}");
}

fn parse_register_args(args: &[String]) -> Option<(String, String, Option<String>)> {
    if args.len() < 2 {
        return None;
    }
    let name = args[0].clone();
    let path = args[1].clone();
    let mut icon = None;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "-ico" {
            icon = args.get(i + 1).cloned();
            i += 2;
        } else {
            i += 1;
        }
    }
    Some((name, path, icon))
}

#[cfg(test)]
mod tests {
    use super::parse_register_args;

    #[test]
    fn parse_register_basic() {
        let args = ["记事本".into(), "C:/Windows/notepad.exe".into()];
        let (name, path, ico) = parse_register_args(&args).unwrap();
        assert_eq!(name, "记事本");
        assert_eq!(path, "C:/Windows/notepad.exe");
        assert!(ico.is_none());
    }

    #[test]
    fn parse_register_with_ico() {
        let args = [
            "Chrome".into(),
            "C:/chrome.exe".into(),
            "-ico".into(),
            "C:/icon.png".into(),
        ];
        let (_, _, ico) = parse_register_args(&args).unwrap();
        assert_eq!(ico.as_deref(), Some("C:/icon.png"));
    }
}
