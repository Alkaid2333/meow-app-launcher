//! 命令行工具喵~
//!
//! `meowal register <名称> <路径> [-ico 图标]` 喵。
//! 独立成模块,便于放在 `tests/` 里做单元测试喵。

use crate::app::config::DATA_DIR_NAME;
use crate::apps::{AppInfo, AppRegistry};

/// `meowal register <名称> <路径> [-ico 图标]` 喵
pub fn cli_register(args: &[String]) {
    let data_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(DATA_DIR_NAME);
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
