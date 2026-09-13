//! 平台抽象层喵~
//!
//! 这是整个项目**唯一**允许出现 `cfg(target_os)` 的目录喵!
//! 向上只暴露 trait 与平台无关类型,业务代码永远不直接碰平台 API 喵。
//!
//! 职责边界:
//! * 窗口生命周期(创建/显示/移动/缩放/销毁),支持多窗口喵
//! * 每像素透明呈现(per-pixel alpha)喵
//! * 全局消息泵(把 WM_* 翻译成平台无关事件,分发给各窗口 handler)喵
//! * 系统托盘(图标/菜单/事件)喵
//! * 全局热键、图标提取、应用启动喵

pub mod win32;

pub use win32::{Win32Platform, WinGpuContext};

use serde::{Deserialize, Serialize};

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

/// 系统托盘句柄喵(不透明)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// 系统托盘句柄喵(单托盘,仅作令牌标识)喵
pub struct TrayHandle;

/// 托盘菜单项喵
#[derive(Debug, Clone)]
pub struct TrayMenuItem {
    /// 菜单项文本喵
    pub label: String,
    /// 是否可点击喵
    pub enabled: bool,
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
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// 全局热键触发喵
    Hotkey,
    /// 导航键按下喵
    KeyDown(Key),
    /// 字符输入喵
    Char(char),
    /// IME 预编辑串(未提交)喵
    ImePreedit(String),
    /// 鼠标按下(客户区物理坐标)喵
    MouseDown(f32, f32),
    /// 鼠标移动(客户区物理坐标)喵
    MouseMove(f32, f32),
    /// 鼠标松开喵
    MouseUp,
    /// 右键松开(客户区物理坐标,请求上下文菜单)喵
    ContextMenu(f32, f32),
    /// 鼠标滚轮(正=向上)喵
    MouseWheel(f32),
    /// 窗口失去激活(前台切走)喵
    LostFocus,
    /// 定时器触发(动画帧驱动)喵
    Timer,
    /// 按键组合喵(修饰键 + 主键,供配置 GUI 录制热键使用)喵
    HotkeyChord {
        /// 修饰键,如 "ctrl+alt" 喵
        modifiers: String,
        /// 主键,如 "space" / "b" / "f5" 喵
        key: String,
    },
    /// 请求关闭喵
    Close,
    /// 拖入文件(路径列表)喵
    FilesDropped(Vec<String>),
    /// IPC 命令到达喵(named-pipe 服务端投递,由启动器执行)喵
    IpcCommand(IpcCommand),
}

/// 窗口事件处理器(由业务层实现)喵
pub trait WindowHandler {
    /// 处理一个窗口事件喵
    fn on_event(&mut self, event: WindowEvent);
}

/// 托盘事件喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    /// 单击托盘图标(显示/隐藏切换)喵
    LeftClick,
    /// 托盘菜单项点击(索引)喵
    Menu(usize),
}

/// 托盘事件处理器(由业务层实现)喵
pub trait TrayHandler {
    /// 处理一个托盘事件喵
    fn on_event(&mut self, event: TrayEvent);
}

/// 系统命令喵(仅内置预定义集合,不接受任意输入,安全边界清晰)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemCommandKind {
    /// 锁屏喵
    Lock,
    /// 睡眠喵
    Sleep,
    /// 关机喵
    Shutdown,
    /// 重启喵
    Restart,
}

impl SystemCommandKind {
    /// 给搜索结果看的名字喵(唯一出处,配置 GUI 也用它)喵
    pub fn title(self) -> &'static str {
        match self {
            Self::Lock => "锁屏",
            Self::Sleep => "睡眠",
            Self::Shutdown => "关机",
            Self::Restart => "重启",
        }
    }
}

/// 获取当前平台的实现喵(按编译目标自动选择)喵
pub fn platform() -> std::sync::Arc<Win32Platform> {
    #[cfg(target_os = "windows")]
    {
        std::sync::Arc::new(Win32Platform::new())
    }
    #[cfg(not(target_os = "windows"))]
    {
        compile_error!("暂仅支持 Windows,后续再支持 Linux/macOS 喵~");
    }
}

// ---------------------------------------------------------------------------
// named-pipe IPC 喵(CLI 与运行中实例的联动通道)喵
// ---------------------------------------------------------------------------

/// 命名管道路径喵(服务端 = 运行中的 GUI 实例,客户端 = meowal CLI)喵
pub const IPC_PIPE_NAME: &str = "\\\\.\\pipe\\meow-app-launcher";

/// IPC 命令喵(文本协议,一行一命令,UTF-8)喵
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcCommand {
    /// 呼出搜索框喵
    Show,
    /// 隐藏搜索框喵
    Hide,
    /// 切换呼出/隐藏喵
    Toggle,
    /// 呼出并填入查询文本喵
    Query(String),
}

/// 解析一行 IPC 命令文本喵(`show` / `hide` / `toggle` / `query <文本>`)喵
///
/// 纯函数,便于单元测试喵。
pub fn parse_ipc_command(line: &str) -> Option<IpcCommand> {
    let line = line.trim();
    let (verb, rest) = match line.find(' ') {
        Some(i) => (&line[..i], line[i + 1..].trim()),
        None => (line, ""),
    };
    match verb {
        "show" => Some(IpcCommand::Show),
        "hide" => Some(IpcCommand::Hide),
        "toggle" => Some(IpcCommand::Toggle),
        "query" if !rest.is_empty() => Some(IpcCommand::Query(rest.to_string())),
        _ => None,
    }
}

/// IPC 服务端对未知命令的标准回执喵(CLI 帮助文案共用,保持一致)喵
pub const IPC_USAGE_HINT: &str =
    "err 未知命令喵 (可用: show / hide / toggle / query <文本>)";
