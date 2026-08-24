//! 平台抽象层喵~
//!
//! 这是整个项目**唯一**允许出现 `cfg(target_os)` 的目录喵!
//! 向上只暴露 trait,业务代码永远不直接碰平台 API 喵。
//!
//! 第一阶段提供: 图标提取、启动应用、全局热键、置顶喵。

pub mod win32;

/// 提取到的图标像素喵(BGRA 格式,自顶向下)喵
#[derive(Debug, Clone)]
pub struct IconPixels {
    /// 宽度(px)喵
    pub width: u32,
    /// 高度(px)喵
    pub height: u32,
    /// BGRA 像素数据喵
    pub bgra: Vec<u8>,
}

/// 平台能力接口喵
pub trait PlatformCapabilities: Send + Sync {
    /// 从文件路径提取图标像素喵,失败返回 None(渲染层兜底字符图标)喵
    fn extract_icon_pixels(&self, path: &str) -> Option<IconPixels>;

    /// 启动一个应用(路径可以是 exe/lnk/url/任意可执行类型)喵,返回是否成功喵
    fn launch(&self, path: &str) -> bool;

    /// 注册全局热键喵,触发时调用回调喵
    ///
    /// * `modifiers` - 修饰键,如 "ctrl+alt" 喵
    /// * `key` - 主键,如 "space" 喵
    /// * `on_trigger` - 热键被按下时回调(可能在其他线程)喵
    /// 返回是否注册成功喵
    fn register_global_hotkey(
        &self,
        modifiers: &str,
        key: &str,
        on_trigger: Box<dyn Fn() + Send + 'static>,
    ) -> bool;

    /// 取消全局热键喵
    fn unregister_global_hotkey(&self);

    /// 设置窗口是否置顶喵(运行时切换,第二阶段实现)喵
    #[allow(dead_code)]
    fn set_always_on_top(&self, on: bool);

    /// 从窗口提取原生句柄(usize 形式,纯读取,不触发平台事件)喵
    fn window_hwnd(&self, window: &gpui::Window) -> Option<usize>;

    /// 通过原生句柄设置可见性喵(纯 Win32 调用)
    ///
    /// * `activate` - 显示时是否激活窗口(抢键盘焦点): 搜索框 `true`,选择窗 `false` 喵。
    ///   搜索框呼出要立刻可键入,需激活;选择窗只是辅助展示,激活会抢走搜索框焦点,
    ///   导致无法继续输入,所以选择窗显示用不激活方式(SW_SHOWNOACTIVATE)喵。
    ///
    /// 必须在 App 借用之外调用!在窗口 update 回调里直接 ShowWindow 会同步触发
    /// 窗口事件回调,回调里再 update 窗口就嵌套借用了喵。
    fn set_visible_hwnd(&self, hwnd: usize, visible: bool, activate: bool) -> bool;

    /// 通过原生句柄移动窗口喵(纯 Win32 调用,同理须在借用外调用)喵
    fn move_window_hwnd(&self, hwnd: usize, x: f32, y: f32) -> bool;

    /// 平台名,用于日志喵
    fn platform_name(&self) -> &'static str;
}

/// 获取当前平台的实现喵(按编译目标自动选择)喵
pub fn platform() -> std::sync::Arc<dyn PlatformCapabilities> {
    #[cfg(target_os = "windows")]
    {
        std::sync::Arc::new(win32::Win32Platform::new())
    }
    #[cfg(not(target_os = "windows"))]
    {
        // 非 Windows 平台第一阶段不实现,后续补喵
        compile_error!("第一阶段仅支持 Windows,后续再支持 Linux/macOS 喵~");
    }
}
