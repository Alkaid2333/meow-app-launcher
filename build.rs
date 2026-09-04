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
        // rc.exe 由 Windows SDK 提供(MSVC 工具链自带),winres 自动定位喵。
        // .ico 仅用于 exe 打包;运行时的 GUI 图标走 assets/app_icons 里的 png 喵。
        if let Err(e) = winres::WindowsResource::new()
            .set_icon("assets/app_icons/app-icon.ico")
            .compile()
        {
            eprintln!("嵌入 exe 图标失败喵: {e}");
            std::process::exit(1);
        }
    }
}
