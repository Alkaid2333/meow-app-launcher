//! 构建脚本喵: 把应用图标嵌入 Windows exe 资源喵。

fn main() {
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
