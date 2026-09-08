//! 构建脚本喵: 把应用图标嵌入 exe 资源 + release 用 Windows 子系统(不弹控制台)喵。

fn main() {
    // release 产物切到 Windows 子系统: 双击启动不再弹出控制台刷日志喵。
    // debug 保留控制台子系统,方便开发时看日志喵。仅作用于 meowal 二进制,
    // 不影响测试可执行文件(test 仍走控制台输出)喵。
    // 注: 子系统切 WINDOWS 后 CRT 默认找 WinMain,必须同时 /ENTRY:mainCRTStartup
    // 让入口仍是 main(与 Rust 的 #![windows_subsystem] 行为一致)喵。
    if std::env::var("PROFILE").map(|p| p == "release").unwrap_or(false) {
        println!("cargo:rustc-link-arg-bin=meowal=/SUBSYSTEM:WINDOWS");
        println!("cargo:rustc-link-arg-bin=meowal=/ENTRY:mainCRTStartup");
    }

    #[cfg(target_os = "windows")]
    {
        // rc.exe 由 Windows SDK 提供;显式定位以绕开 winres 的注册表查询
        // (部分安全策略会拦截 reg.exe,导致 SDK 探测失败)喵。
        // .ico 嵌入 exe 资源(id=1),exe 图标与托盘图标同款喵。
        let mut res = winres::WindowsResource::new();
        if let Some(toolkit) = locate_rc_toolkit() {
            println!("cargo:rerun-if-env-changed=MEOWAL_RC_TOOLKIT");
            println!("cargo:warning=使用显式定位的 RC 工具链: {}", toolkit.display());
            res.set_toolkit_path(toolkit.to_string_lossy().as_ref());
        }
        if let Err(e) = res.set_icon("assets/app_icons/app-icon.ico").compile() {
            eprintln!("嵌入 exe 图标失败喵: {e}");
            std::process::exit(1);
        }
    }
}

/// 定位含 rc.exe 的 SDK 工具链目录喵。
///
/// 优先级: 环境变量 `MEOWAL_RC_TOOLKIT` > 扫描 Windows Kits > None(交给 winres 默认探测)。
/// 返回的是「直接包含 rc.exe」的目录(如 `...\Windows Kits\10\bin\10.0.26100.0\x64`)喵。
fn locate_rc_toolkit() -> Option<std::path::PathBuf> {
    // 1. 手动指定喵(特殊环境兜底)喵
    if let Ok(path) = std::env::var("MEOWAL_RC_TOOLKIT") {
        let path = std::path::PathBuf::from(path);
        if path.join("rc.exe").is_file() {
            return Some(path);
        }
        eprintln!("MEOWAL_RC_TOOLKIT 指向的目录没有 rc.exe 喵: {}", path.display());
    }

    // 2. 扫描常见 SDK 安装位置,取版本号最高的那个喵
    let roots = [
        r"C:\Program Files (x86)\Windows Kits\10\bin",
        r"C:\Program Files\Windows Kits\10\bin",
    ];
    let mut best: Option<(std::ffi::OsString, std::path::PathBuf)> = None;
    for root in roots {
        let entries = match std::fs::read_dir(root) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let rc = entry.path().join(r"x64\rc.exe");
            if rc.is_file() {
                // 版本目录名是语义化版本字符串,按字典序取最大即可喵
                let ver = entry.file_name();
                let better = best
                    .as_ref()
                    .map(|(best_ver, _)| ver > *best_ver)
                    .unwrap_or(true);
                if better {
                    // winres 约定: toolkit 目录必须直接包含 rc.exe 喵
                    best = Some((ver, rc.parent().unwrap().to_path_buf()));
                }
            }
        }
    }
    best.map(|(_, dir)| dir)
}
