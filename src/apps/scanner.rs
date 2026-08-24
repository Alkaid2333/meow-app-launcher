//! 应用扫描器喵~
//!
//! 自动扫描系统统一存放软件的目录并注册应用喵。
//! 第一阶段支持 Windows 开始菜单快捷方式(.lnk)扫描喵,
//! 后续可扩展 Linux(.desktop)、macOS(.app)喵。

use super::{AppInfo, AppSource};
use std::path::{Path, PathBuf};

/// 扫描到的应用喵
pub fn scan_installed_apps() -> Vec<AppInfo> {
    let mut apps: Vec<AppInfo> = Vec::new();

    // Windows: 开始菜单 Programs 目录喵
    #[cfg(target_os = "windows")]
    {
        apps.extend(scan_windows_start_menu());
    }
    // TODO: Linux .desktop 扫描、macOS .app 扫描喵(第二阶段喵)

    log::info!("扫描完成,共发现 {} 个应用喵", apps.len());
    apps
}

/// 扫描 Windows 开始菜单下的 .lnk 快捷方式喵
#[cfg(target_os = "windows")]
fn scan_windows_start_menu() -> Vec<AppInfo> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    // 系统级: C:\ProgramData\Microsoft\Windows\Start Menu\Programs
    if let Some(pd) = std::env::var_os("ProgramData") {
        dirs.push(Path::new(&pd).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }
    // 用户级: %APPDATA%\Microsoft\Windows\Start Menu\Programs
    if let Some(ad) = std::env::var_os("APPDATA") {
        dirs.push(Path::new(&ad).join("Microsoft\\Windows\\Start Menu\\Programs"));
    }

    let mut apps = Vec::new();
    for dir in dirs {
        if !dir.exists() {
            log::debug!("开始菜单目录不存在,跳过: {:?}", dir);
            continue;
        }
        log::debug!("扫描开始菜单: {:?}", dir);
        collect_links(&dir, &mut apps);
    }
    apps
}

/// 递归收集目录下的 .lnk 文件喵
#[cfg(target_os = "windows")]
fn collect_links(dir: &Path, out: &mut Vec<AppInfo>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        log::warn!("读取目录失败: {:?}", dir);
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // 跳过卸载相关的目录,免得注册一堆卸载器喵
            let lower = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();
            if lower.contains("uninstall") {
                continue;
            }
            collect_links(&path, out);
            continue;
        }
        // 只关心 .lnk 快捷方式喵
        if path.extension().and_then(|e| e.to_str()) == Some("lnk") {
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("未命名应用")
                .to_string();
            // 过滤明显没用的系统链接喵
            if name.is_empty()
                || name.eq_ignore_ascii_case("desktop.ini")
                || name.starts_with(".")
            {
                continue;
            }
            out.push(AppInfo {
                name,
                path: path.to_string_lossy().to_string(),
                icon_path: None,
                tags: Vec::new(),
                favorite: false,
                launch_count: 0,
                last_used: 0,
                source: AppSource::Scanned,
            });
        }
    }
}
