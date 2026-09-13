//! 命令行工具喵~
//!
//! `meowal register <名称> <路径> [-ico 图标]` 喵。
//! `meowal show|hide|toggle|query <文本>` 喵: 经 named-pipe 与运行中的实例联动喵。
//! 独立成模块,便于放在 `tests/` 里做单元测试喵。

use crate::app::config;
use crate::apps::{AppInfo, AppRegistry};

/// `meowal register <名称> <路径> [-ico 图标]` 喵
pub fn cli_register(args: &[String]) {
    // release 是 Windows 子系统(无控制台),从终端调用时先附加父控制台,
    // 让 println/eprintln 输出可见喵。
    #[cfg(target_os = "windows")]
    attach_parent_console();

    // 数据目录固定为 exe 同目录的 .datas(与 GUI 一致,不随终端 CWD 漂移)喵
    let data_dir = config::data_dir();
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
    let mut registry = AppRegistry::load(&data_dir);
    let mut app_info = AppInfo::manual(&name, &path);
    app_info.icon_path = icon;
    registry.upsert(app_info);
    registry.save(&data_dir);
    println!("已注册: {name} → {path}");
}

/// 附加到父进程控制台并重绑 stdout/stderr 喵(仅 Windows,release 无控制台时生效)喵
///
/// GUI 双击启动时没有父控制台,AttachConsole 失败即静默返回,不打扰喵。
#[cfg(target_os = "windows")]
fn attach_parent_console() {
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        AttachConsole, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
    };
    use std::ptr::null_mut;

    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            // 无父控制台(如双击启动),无需处理喵
            return;
        }
        let con = CreateFileW(
            "CONOUT$\0".encode_utf16().collect::<Vec<_>>().as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        );
        if con != INVALID_HANDLE_VALUE {
            // Rust 的 stdout/stderr 每次访问都会重新 GetStdHandle,重绑后输出即可见喵
            SetStdHandle(STD_OUTPUT_HANDLE, con);
            SetStdHandle(STD_ERROR_HANDLE, con);
        }
    }
}

/// `meowal show|hide|toggle|query <文本>` 喵: 经 named-pipe 联动运行中的实例喵
///
/// 实例未运行时打印提示并以退出码 1 结束喵。
pub fn cli_ipc(args: &[String]) {
    #[cfg(target_os = "windows")]
    attach_parent_console();

    let command = match args.first().map(|s| s.as_str()) {
        Some("show") => "show".to_string(),
        Some("hide") => "hide".to_string(),
        Some("toggle") => "toggle".to_string(),
        Some("query") => {
            let text = args[1..].join(" ");
            if text.trim().is_empty() {
                eprintln!("用法: meowal query <搜索文本>");
                std::process::exit(1);
            }
            format!("query {text}")
        }
        _ => {
            eprintln!("用法: meowal show | hide | toggle | query <文本> | register <名称> <路径>");
            std::process::exit(1);
        }
    };

    let platform = crate::platform::platform();
    match platform.send_ipc_command(&command) {
        Ok(reply) => println!("{reply}"),
        Err(e) => {
            eprintln!("meowal: {e}");
            std::process::exit(1);
        }
    }
}

/// 解析注册命令参数喵:`<名称> <路径> [-ico 图标]` 喵
pub fn parse_register_args(args: &[String]) -> Option<(String, String, Option<String>)> {
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
