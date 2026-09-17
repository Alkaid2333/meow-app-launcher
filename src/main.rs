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
//! 核心模块在库 `meow_app_launcher` 中,本文件仅负责装配与入口喵。

use meow_app_launcher::app::config;
use meow_app_launcher::app::{AppState, SharedState};
use meow_app_launcher::cli;
use meow_app_launcher::platform;
use meow_app_launcher::utils;
use meow_app_launcher::window::{AppManagerWindow, Launcher, SettingsWindow, Tray};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // CLI 子命令喵(register 走离线注册,其余经 IPC 与运行中的实例联动)喵
    if let Some(first) = args.first() {
        match first.as_str() {
            "register" => {
                cli::cli_register(&args[1..]);
                return;
            }
            "show" | "hide" | "toggle" | "query" => {
                cli::cli_ipc(&args);
                return;
            }
            _ => {}
        }
    }

    // 数据目录喵(便携式,固定在 exe 同目录的 .datas,与 CLI register 保持一致)喵
    let data_dir = config::data_dir();

    // 日志喵(MEOWAL_VERBOSE=1 开启 debug 日志)喵
    utils::logger::init(&data_dir, std::env::var("MEOWAL_VERBOSE").is_ok());
    log::info!("喵喵启动器启动喵! 版本 {}", env!("CARGO_PKG_VERSION"));

    let platform = platform::platform();

    // 常驻进程的环境块是启动时的快照: 开机自启的场景下这块可能已经存放很久了,
    // 开机先对齐一次注册表,之后拉起的子进程就不会再用旧环境变量喵。
    platform.refresh_environment();

    // 单实例保护喵: 已有实例在跑时,把「唤起」意图转交给它后退出
    // (再次启动 meowal = 呼出已有实例的搜索框,不再出现双岛互踩)喵
    if !platform.try_acquire_single_instance() {
        platform.notify_existing_instance();
        return;
    }

    let state: SharedState = Rc::new(RefCell::new(AppState::new(platform.clone(), data_dir)));
    let commands = Rc::new(RefCell::new(VecDeque::new()));

    // 0. 让 `meowal` 命令在终端可用(各平台自行实现;失败不阻断主流程)喵
    if platform.install_cli_command() {
        log::info!("meowal 命令已就绪(可作终端命令使用)喵");
    } else {
        log::warn!("meowal 命令注册失败,不影响应用运行喵~");
    }

    // 1. 同步开机自启状态喵(配置开启时确保注册表一致,路径变更也能自动修复)喵
    if state.borrow().config.auto_start {
        let ok = platform.set_auto_start(true);
        log::info!("启动时同步开机自启: 成功={ok} 喵");
    }

    // 1. 创建配置窗口与应用管理中心(初始隐藏)喵
    let settings_window = SettingsWindow::spawn(platform.clone(), state.clone(), commands.clone());
    let manager_window = AppManagerWindow::spawn(platform.clone(), state.clone());

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

    // 5. 启动 named-pipe IPC 服务端(`meowal show|hide|toggle|query` 的联动入口)喵
    platform.start_ipc_server(launcher_window);

    // 6. 运行全局消息循环喵
    platform.run();

    // 7. 清理资源喵
    log::info!("应用退出,清理资源喵~");
    platform.unregister_global_hotkey();
    platform.destroy_tray(&tray);
    platform.destroy_window(&launcher_window);
    platform.destroy_window(&settings_window);
    platform.destroy_window(&manager_window);
}
