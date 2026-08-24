//! 平台抽象层喵~
//!
//! 这是整个项目**唯一**允许出现 `cfg(target_os)` 的目录喵!
//! 向上只暴露 trait 与平台无关类型,业务代码永远不直接碰平台 API 喵。
//!
//! 职责边界:
//! * 窗口生命周期(创建/显示/移动/缩放/销毁)喵
//! * 每像素透明呈现(per-pixel alpha)喵
//! * 消息泵(把 WM_* 翻译成平台无关事件)喵
//! * 全局热键、图标提取、应用启动喵

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

/// 窗口创建规格喵(物理像素)喵
#[derive(Debug, Clone, Copy)]
pub struct WindowSpec {
    /// 左上角 x 坐标喵
    pub x: i32,
    /// 左上角 y 坐标喵
    pub y: i32,
    /// 宽度喵
    pub width: i32,
    /// 高度喵
    pub height: i32,
}

/// 平台窗口句柄喵(不透明,仅平台层可解引用)喵
///
/// 内部是原生句柄的 `usize` 表示,保证 `Send + Sync + Copy`,
/// 可跨线程(如热键线程)安全传递喵。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlatformWindow {
    hwnd: usize,
}

impl PlatformWindow {
    /// 仅供平台层构造喵
    pub(crate) fn from_hwnd(hwnd: usize) -> Self {
        Self { hwnd }
    }

    /// 仅供平台层读取原生句柄喵
    pub(crate) fn hwnd(&self) -> usize {
        self.hwnd
    }
}

/// 导航键喵(平台无关)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Backspace,
    Tab,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
}

/// 平台无关窗口事件喵
#[derive(Debug, Clone, Copy)]
pub enum WindowEvent {
    /// 全局热键触发喵
    Hotkey,
    /// 导航键按下喵
    KeyDown(Key),
    /// 字符输入喵
    Char(char),
    /// 鼠标按下(客户区物理坐标)喵
    MouseDown(f32, f32),
    /// 窗口失去激活(前台切走)喵
    LostFocus,
    /// 定时器触发(动画帧驱动)喵
    Timer,
    /// 请求关闭喵
    Close,
}

/// 窗口事件处理器(由业务层实现)喵
pub trait WindowHandler {
    /// 处理一个窗口事件喵
    fn on_event(&mut self, event: WindowEvent);
}

/// 平台能力接口喵
pub trait Platform: Send + Sync {
    /// 从文件路径提取图标像素喵,失败返回 None(渲染层兜底字符图标)喵
    fn extract_icon_pixels(&self, path: &str) -> Option<IconPixels>;

    /// 启动一个应用(路径可以是 exe/lnk/url/任意可执行类型)喵,返回是否成功喵
    fn launch(&self, path: &str) -> bool;

    /// 注册全局热键喵,触发时向 `target` 窗口投递 Hotkey 事件喵
    ///
    /// * `modifiers` - 修饰键,如 "ctrl+alt" 喵
    /// * `key` - 主键,如 "space" 喵
    /// * `target` - 热键触发的目标窗口喵
    ///
    /// 返回是否注册成功喵
    fn register_global_hotkey(
        &self,
        modifiers: &str,
        key: &str,
        target: PlatformWindow,
    ) -> bool;

    /// 取消全局热键喵
    fn unregister_global_hotkey(&self);

    /// 创建异形透明置顶窗口喵
    fn create_window(&self, spec: &WindowSpec) -> Option<PlatformWindow>;

    /// 销毁窗口喵
    fn destroy_window(&self, window: &PlatformWindow);

    /// 显示/隐藏窗口喵
    fn show_window(&self, window: &PlatformWindow, show: bool);

    /// 调整窗口尺寸(物理像素)喵
    fn resize_window(&self, window: &PlatformWindow, width: i32, height: i32);

    /// 用 BGRA 像素呈现窗口内容喵(每像素透明)喵
    fn present(&self, window: &PlatformWindow, width: i32, height: i32, bgra: &[u8]);

    /// 设置定时器喵(驱动动画帧,间隔毫秒)喵
    fn set_timer(&self, window: &PlatformWindow, interval_ms: u32);

    /// 停止定时器喵
    fn kill_timer(&self, window: &PlatformWindow);

    /// 进入消息循环喵,阻塞直到窗口关闭喵
    fn run_message_loop(&self, window: &PlatformWindow, handler: &mut dyn WindowHandler);

    /// 系统 DPI 缩放系数喵(1.0 = 100%)喵
    fn scale_factor(&self) -> f32;

    /// 主屏尺寸(物理像素)喵
    fn screen_size(&self) -> (i32, i32);

    /// 平台名,用于日志喵
    fn platform_name(&self) -> &'static str;
}

/// 获取当前平台的实现喵(按编译目标自动选择)喵
pub fn platform() -> std::sync::Arc<dyn Platform> {
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
