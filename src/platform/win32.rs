//! Windows 平台实现喵~
//!
//! 通过 windows-sys 直接调用 Win32 API 喵。
//! 提供: 异形透明窗口(多窗口)、每像素透明呈现、全局消息泵、系统托盘、
//! 全局热键、图标提取、应用启动喵。
//!
//! 注意: windows-sys 0.59 的 HWND 是 `*mut c_void` 类型别名,
//! 不是 tuple struct,所以直接用指针、跨线程存储时转 usize 喵。

use super::{
    IconPixels, Key, Modifier, ModifierSet, PlatformWindow, SystemCommandKind, TrayEvent,
    TrayHandle, TrayHandler, TrayMenuItem, WindowEvent, WindowHandler, WindowSpec, env,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::sync::{Arc, LazyLock, Mutex, Once, OnceLock};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::HICON;

/// 注册热键的 id 喵(呼出搜索框)喵
const HOTKEY_ID_LAUNCHER: i32 = 0x4D4F; // "MO" 喵
/// 注册热键的 id 喵(后台扫描应用)喵
const HOTKEY_ID_SCAN: i32 = 0x4D50; // "MP" 喵
/// 热键线程 → 主窗口的自定义消息(WM_APP + 1)喵
const WM_MEOW_HOTKEY: u32 = 0x8000 + 1;
/// 扫描热键线程 → 主窗口的自定义消息(WM_APP + 2)喵
const WM_MEOW_SCAN: u32 = 0x8000 + 2;
/// IPC 服务端 → 主窗口的自定义消息(WM_APP + 3)喵
const WM_MEOW_IPC: u32 = 0x8000 + 3;
/// IPC 命令队列喵(named-pipe 服务端线程投递,主线程在 wnd_proc 里取走)喵
static IPC_QUEUE: Mutex<std::collections::VecDeque<super::IpcCommand>> =
    Mutex::new(std::collections::VecDeque::new());
/// 动画定时器 id 喵
const TIMER_ID: usize = 1;
/// 托盘回调消息(WM_USER + 1)喵
const WM_TRAY_MSG: u32 = 0x0400 + 1;
/// 托盘图标被选中(Win2000+,左键单击)喵
const NIN_SELECT: u32 = 0x0400;
/// IME 组字消息喵
const WM_IME_COMPOSITION: u32 = 0x010F;
/// 拖入文件消息喵
const WM_DROPFILES: u32 = 0x0233;
/// 窗口类名(UTF-16 编码,含 null 终止)喵
const CLASS_NAME: [u16; 19] = [
    0x4D, 0x65, 0x6F, 0x77, // "Meow"
    0x4C, 0x61, 0x75, 0x6E, 0x63, 0x68, 0x65, 0x72, // "Launcher"
    0x57, 0x69, 0x6E, 0x64, 0x6F, 0x77, // "Window"
    0x00, // null 终止喵
];
/// 托盘窗口类名(UTF-16,含 null)喵
const TRAY_CLASS_NAME: [u16; 15] = [
    0x4D, 0x65, 0x6F, 0x77, // "Meow"
    0x54, 0x72, 0x61, 0x79, // "Tray"
    0x57, 0x69, 0x6E, 0x64, 0x6F, 0x77, // "Window"
    0x00, // null 终止喵
];

/// 全局窗口类注册锁喵(整个进程只注册一次)喵
static CLASS_ONCE: Once = Once::new();
/// 托盘窗口类注册锁喵
static TRAY_CLASS_ONCE: Once = Once::new();

/// 转成 Win32 需要的宽字符串(NUL 结尾)喵
///
/// `*W` 系列 API 全都吃这个形态,包一层省得处处手敲喵。
fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

// 各窗口的事件处理器喵(单线程,按 hwnd 索引)喵
thread_local! {
    static HANDLERS: RefCell<HashMap<usize, Box<dyn WindowHandler>>> =
        RefCell::new(HashMap::new());
}

// 托盘事件处理器喵(单托盘)喵
thread_local! {
    static TRAY_HANDLER: RefCell<Option<Box<dyn TrayHandler>>> = RefCell::new(None);
}

// 托盘运行时状态喵(图标句柄 + 菜单项)喵
thread_local! {
    static TRAY_STATE: RefCell<Option<TrayState>> = const { RefCell::new(None) };
}

/// 托盘运行时状态喵
struct TrayState {
    /// 隐藏窗口句柄喵
    hwnd: usize,
    /// 当前图标喵
    icon: HICON,
    /// 当前菜单项喵
    menu: Vec<TrayMenuItem>,
}

/// Windows 平台句柄喵
pub struct Win32Platform {
    /// 呼出热键隐藏窗口句柄(usize 形式,保证 Send)喵
    hotkey_hwnd: Arc<Mutex<Option<usize>>>,
    /// 扫描热键隐藏窗口句柄喵(与呼出热键各占一条线程,互不干扰)喵
    scan_hotkey_hwnd: Arc<Mutex<Option<usize>>>,
}

impl Win32Platform {
    pub fn new() -> Self {
        enable_dpi_awareness();
        Self {
            hotkey_hwnd: Arc::new(Mutex::new(None)),
            scan_hotkey_hwnd: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for Win32Platform {
    fn default() -> Self {
        Self::new()
    }
}

impl Win32Platform {
    pub fn extract_icon_pixels(&self, path: &str) -> Option<IconPixels> {
        extract_icon_pixels_impl(path)
    }

    pub fn create_gpu_context(&self) -> Option<WinGpuContext> {
        create_gpu_context_impl()
    }

    pub fn launch(&self, path: &str) -> bool {
        launch_impl(path)
    }

    /// 以管理员身份启动喵(Windows 走 UAC 提权,Linux 未来走 sudo)喵
    pub fn launch_elevated(&self, path: &str) -> bool {
        launch_elevated_impl(path)
    }

    /// 把进程环境变量同步到注册表最新值喵(启动子进程前 / 收到系统广播时调用)喵
    pub fn refresh_environment(&self) -> bool {
        refresh_process_environment()
    }

    /// 此刻按住了哪些修饰键喵
    ///
    /// 用异步键状态而非消息队列状态: 要的是「物理上按着没」,
    /// 这样鼠标点选时按住修饰键也能识别喵。
    pub fn held_modifiers(&self) -> ModifierSet {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
        };

        let mut set = ModifierSet::empty();
        let pressed = |vk: u16| unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 };
        if pressed(VK_CONTROL) {
            set.insert(Modifier::Ctrl);
        }
        if pressed(VK_SHIFT) {
            set.insert(Modifier::Shift);
        }
        if pressed(VK_MENU) {
            set.insert(Modifier::Alt);
        }
        if pressed(VK_LWIN) || pressed(VK_RWIN) {
            set.insert(Modifier::Win);
        }
        set
    }

    /// 带参数启动喵(explorer /select 打开所在位置等场景)喵
    pub fn launch_args(&self, path: &str, args: &str) -> bool {
        launch_args_impl(path, args)
    }

    /// 启动 named-pipe IPC 服务端喵(CLI 联动入口,命令投递到目标窗口)喵
    ///
    /// 服务端运行在独立线程,失败不阻断主流程(只是 CLI 联动不可用)喵。
    pub fn start_ipc_server(&self, target: PlatformWindow) {
        let hwnd = target.hwnd();
        let spawned = std::thread::Builder::new()
            .name("meow-ipc-server".into())
            .spawn(move || ipc_server_loop(hwnd as HWND));
        match spawned {
            Ok(_) => log::info!("IPC 服务端已就绪喵~"),
            Err(e) => log::warn!("IPC 服务端线程创建失败({e}),CLI 联动不可用喵~"),
        }
    }

    /// 连接运行中的实例并发送一条 IPC 命令喵(CLI 客户端用)喵
    ///
    /// 返回服务端回执;实例未运行或管道不可达时返回 Err 喵。
    pub fn send_ipc_command(&self, line: &str) -> Result<String, String> {
        send_ipc_command_impl(line)
    }

    pub fn open_url(&self, url: &str, browser: &str) -> bool {
        open_url_impl(url, browser)
    }

    pub fn copy_to_clipboard(&self, text: &str) -> bool {
        copy_to_clipboard_impl(text)
    }

    pub fn execute_system_command(&self, command: SystemCommandKind) -> bool {
        execute_system_command_impl(command)
    }

    /// 弹一条系统通知喵(委托 [`super::NotificationSink`] 特质的平台实现)喵
    pub fn show_notification(&self, title: &str, body: &str) {
        super::NotificationSink::show_notification(self, title, body);
    }

    /// 在指定 shell 里执行指令喵(委托 [`super::ShellRunner`] 特质的平台实现)喵
    ///
    /// 阻塞调用,务必放在后台线程喵。
    pub fn run_shell(&self, shell: super::ShellKind, command: &str) -> Result<String, String> {
        super::ShellRunner::run_shell(self, shell, command)
    }

    pub fn register_global_hotkey(
        &self,
        modifiers: &str,
        key: &str,
        target: PlatformWindow,
    ) -> bool {
        register_hotkey_impl(
            HOTKEY_ID_LAUNCHER,
            WM_MEOW_HOTKEY,
            true,
            "呼出",
            modifiers,
            key,
            target,
            &self.hotkey_hwnd,
        )
    }

    /// 注册「扫描应用」全局热键喵(触发后台扫描,不抢前台不呼出岛)喵
    pub fn register_scan_hotkey(
        &self,
        modifiers: &str,
        key: &str,
        target: PlatformWindow,
    ) -> bool {
        register_hotkey_impl(
            HOTKEY_ID_SCAN,
            WM_MEOW_SCAN,
            false,
            "扫描",
            modifiers,
            key,
            target,
            &self.scan_hotkey_hwnd,
        )
    }

    pub fn unregister_global_hotkey(&self) {
        unregister_hotkey_impl(&self.hotkey_hwnd, "呼出");
    }

    /// 取消「扫描应用」全局热键喵
    pub fn unregister_scan_hotkey(&self) {
        unregister_hotkey_impl(&self.scan_hotkey_hwnd, "扫描");
    }

    pub fn set_auto_start(&self, enabled: bool) -> bool {
        set_auto_start_impl(enabled)
    }

    pub fn install_cli_command(&self) -> bool {
        install_cli_command_impl()
    }

    pub fn create_window(&self, spec: &WindowSpec) -> Option<PlatformWindow> {
        // 确保窗口类已注册喵
        CLASS_ONCE.call_once(register_class);

        unsafe {
            let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::CreateWindowExW(
                // 分层(per-pixel alpha) + 置顶 + 不进任务栏喵
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                CLASS_NAME.as_ptr(),
                CLASS_NAME.as_ptr(),
                WS_POPUP,
                spec.x,
                spec.y,
                spec.width,
                spec.height,
                null_mut(),
                null_mut(),
                windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(null_mut()),
                null_mut(),
            );
            if hwnd.is_null() {
                log::error!("创建窗口失败喵");
                None
            } else {
                associate_ime(hwnd);
                log::debug!("窗口创建成功: hwnd={} 喵", hwnd as usize);
                Some(PlatformWindow::from_hwnd(hwnd as usize))
            }
        }
    }

    pub fn set_window_handler(&self, window: &PlatformWindow, handler: Box<dyn WindowHandler>) {
        HANDLERS.with(|handlers| {
            handlers.borrow_mut().insert(window.hwnd(), handler);
        });
    }

    pub fn destroy_window(&self, window: &PlatformWindow) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(window.hwnd() as HWND);
        }
    }

    pub fn show_window(&self, window: &PlatformWindow, show: bool) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOW};
        unsafe {
            // SW_SHOW: 显示并允许激活;键盘焦点随后由 focus_window 统一钉住喵
            ShowWindow(window.hwnd() as HWND, if show { SW_SHOW } else { SW_HIDE });
        }
    }

    /// 把窗口顶到前台并收回键盘焦点喵~
    ///
    /// 分层置顶窗有个阴魂不散的坑: 窗口可能已经是「前台窗口」,键盘焦点却留在
    /// 别的线程 —— 此时按键会被系统判定为无效输入直接丢弃,还附赠一声提示音喵。
    /// 所以这里不做「已是前台就返回」的省事早退,而是前台 + 焦点双重确认喵。
    pub fn focus_window(&self, window: &PlatformWindow) {
        use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetFocus, SetFocus};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
        };

        let hwnd = window.hwnd() as HWND;
        unsafe {
            // 前台与键盘焦点双双到位,才算真的「就绪」喵
            if GetForegroundWindow() == hwnd && GetFocus() == hwnd {
                return;
            }

            // 前台锁自救: Windows 会拒绝后台进程抢前台(启动外部应用后必被拒)。
            // 附加到前台线程的输入队列,即可合法完成切换喵。
            let fg = GetForegroundWindow();
            let fg_thread = if fg.is_null() {
                0
            } else {
                GetWindowThreadProcessId(fg, null_mut())
            };
            let cur_thread = GetCurrentThreadId();
            let attached = fg_thread != 0 && fg_thread != cur_thread;
            if attached {
                AttachThreadInput(cur_thread, fg_thread, 1);
            }
            let foreground_ok = SetForegroundWindow(hwnd) != 0;
            if attached {
                AttachThreadInput(cur_thread, fg_thread, 0);
            }

            if foreground_ok {
                // 前台到手后立刻把键盘焦点钉回本窗: 这一下才是「打得进字」的关键喵
                BringWindowToTop(hwnd);
                SetFocus(hwnd);
            } else {
                log::debug!("SetForegroundWindow 被前台锁拒绝(已尝试附加输入线程),待点击自救喵~");
            }
        }
    }

    pub fn enable_file_drop(&self, window: &PlatformWindow) {
        use windows_sys::Win32::UI::Shell::DragAcceptFiles;
        use windows_sys::Win32::UI::WindowsAndMessaging::{ChangeWindowMessageFilterEx, MSGFLT_ALLOW};
        unsafe {
            let hwnd = window.hwnd() as HWND;
            DragAcceptFiles(hwnd, 1);
            // UIPI 放行喵: 打包安装后若以更高完整性级别运行,低权限资源管理器
            // 拖入的 WM_DROPFILES 会被系统消息过滤器拦截,导致拖拽注册失效喵。
            ChangeWindowMessageFilterEx(hwnd, WM_DROPFILES, MSGFLT_ALLOW, null_mut());
        }
    }

    pub fn resize_window(&self, window: &PlatformWindow, width: i32, height: i32) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOZORDER,
        };
        unsafe {
            SetWindowPos(
                window.hwnd() as HWND,
                HWND_TOPMOST,
                0,
                0,
                width,
                height,
                SWP_NOMOVE,
            );
        }
        let _ = SWP_NOZORDER;
    }

    pub fn move_window(&self, window: &PlatformWindow, x: i32, y: i32) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOSIZE,
        };
        unsafe {
            SetWindowPos(
                window.hwnd() as HWND,
                HWND_TOPMOST,
                x,
                y,
                0,
                0,
                SWP_NOSIZE,
            );
        }
    }

    pub fn present(&self, window: &PlatformWindow, width: i32, height: i32, bgra: &[u8]) {
        present_impl(window.hwnd() as HWND, width, height, bgra);
    }

    pub fn set_timer(&self, window: &PlatformWindow, interval_ms: u32) {
        use windows_sys::Win32::UI::WindowsAndMessaging::SetTimer;
        unsafe {
            SetTimer(window.hwnd() as HWND, TIMER_ID, interval_ms, None);
        }
    }

    pub fn create_tray(&self) -> Option<TrayHandle> {
        // 确保托盘窗口类已注册喵
        TRAY_CLASS_ONCE.call_once(|| {
            register_tray_class();
        });

        // 注册 TaskbarCreated 广播消息(explorer 重启后重建托盘图标用)喵
        WM_TRAY_TASKBAR.get_or_init(|| {
            let name: Vec<u16> = "TaskbarCreated\0".encode_utf16().collect();
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW(name.as_ptr())
            }
        });

        // 从 exe 资源加载应用图标(build.rs 嵌入的 app-icon.ico,和 exe 图标同款)喵
        let (sw, sh) = small_icon_size();
        let hicon = app_icon_hicon(sw, sh);
        if hicon.is_null() {
            log::warn!("托盘应用图标加载失败,托盘将以无图标形式创建喵~");
        }

        unsafe {
            let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::CreateWindowExW(
                0,
                TRAY_CLASS_NAME.as_ptr(),
                TRAY_CLASS_NAME.as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(null_mut()),
                null_mut(),
            );
            if hwnd.is_null() {
                log::error!("托盘窗口创建失败喵");
                return None;
            }

            // 添加托盘图标喵(一次性带上 回调消息 + 图标 + 提示,避免先出现空槽)喵
            let mut nid: NOTIFYICONDATAW = zeroed();
            nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.uCallbackMessage = WM_TRAY_MSG;
            nid.hIcon = hicon;
            set_nid_tip(&mut nid, "meow app launcher");
            if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
                log::error!("添加托盘图标失败喵");
                if !hicon.is_null() {
                    windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon(hicon);
                }
                windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
                return None;
            }

            // 存储状态喵
            TRAY_STATE.with(|s| {
                *s.borrow_mut() = Some(TrayState {
                    hwnd: hwnd as usize,
                    icon: hicon,
                    menu: Vec::new(),
                });
            });

            log::info!("系统托盘创建成功喵");
            Some(TrayHandle)
        }
    }

    pub fn set_tray_handler(&self, _tray: &TrayHandle, handler: Box<dyn TrayHandler>) {
        TRAY_HANDLER.with(|h| *h.borrow_mut() = Some(handler));
    }

    pub fn destroy_tray(&self, _tray: &TrayHandle) {
        let state = TRAY_STATE.with(|s| s.borrow_mut().take());
        if let Some(state) = state {
            unsafe {
                let mut nid: NOTIFYICONDATAW = zeroed();
                nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
                nid.hWnd = state.hwnd as HWND;
                nid.uID = 1;
                Shell_NotifyIconW(NIM_DELETE, &nid);
                if !state.icon.is_null() {
                    windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon(state.icon);
                }
                windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(state.hwnd as HWND);
            }
        }
        TRAY_HANDLER.with(|h| *h.borrow_mut() = None);
        log::debug!("托盘已移除喵");
    }

    pub fn set_tray_app_icon(&self, _tray: &TrayHandle) {
        let (sw, sh) = small_icon_size();
        let hicon = app_icon_hicon(sw, sh);
        if hicon.is_null() {
            log::warn!("托盘应用图标加载失败喵~");
            return;
        }

        let mut old_icon = null_mut();
        TRAY_STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                old_icon = std::mem::replace(&mut state.icon, hicon);
                unsafe {
                    let mut nid: NOTIFYICONDATAW = zeroed();
                    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
                    nid.hWnd = state.hwnd as HWND;
                    nid.uID = 1;
                    nid.uFlags = NIF_ICON;
                    nid.hIcon = hicon;
                    Shell_NotifyIconW(NIM_MODIFY, &nid);
                }
            }
        });

        // 释放旧图标喵
        if !old_icon.is_null() {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon(old_icon);
            }
        }
    }

    pub fn try_acquire_single_instance(&self) -> bool {
        use windows_sys::Win32::System::Threading::CreateMutexW;
        use windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS;

        // 命名互斥体: 句柄故意常驻进程,退出时由系统回收喵
        let name: Vec<u16> = "Local\\MeowAppLauncher.SingleInstance\0"
            .encode_utf16()
            .collect();
        unsafe {
            let handle = CreateMutexW(null_mut(), 0, name.as_ptr());
            if handle.is_null() {
                log::warn!(
                    "单实例互斥体创建失败(rc={}),放行启动喵~",
                    windows_sys::Win32::Foundation::GetLastError()
                );
                return true;
            }
            SINGLE_INSTANCE_MUTEX.store(handle as usize, std::sync::atomic::Ordering::SeqCst);
            let already =
                windows_sys::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS;
            if already {
                log::info!("检测到已有实例在运行喵~");
            }
            !already
        }
    }

    pub fn notify_existing_instance(&self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowW, PostMessageW,
        };

        // 优先走 IPC 通道(与 CLI 联动同一套协议,支持传参)喵
        if self.send_ipc_command("show").is_ok() {
            log::info!("已通过 IPC 通知现有实例唤起搜索框喵~");
            return;
        }
        // 回退: 找到主窗口,投递热键同款消息 → 已运行实例直接呼出搜索框喵
        unsafe {
            let hwnd = FindWindowW(CLASS_NAME.as_ptr(), std::ptr::null());
            if !hwnd.is_null() {
                PostMessageW(hwnd, WM_MEOW_HOTKEY, 0, 0);
                log::info!("已通知现有实例唤起搜索框(PostMessage 回退)喵~");
            } else {
                log::warn!("未找到现有实例的主窗口喵~");
            }
        }
    }

    pub fn set_tray_tip(&self, _tray: &TrayHandle, tip: &str) {
        let mut tip_wide: Vec<u16> = tip.encode_utf16().collect();
        tip_wide.truncate(127);
        tip_wide.push(0);

        TRAY_STATE.with(|s| {
            if let Some(state) = s.borrow().as_ref() {
                unsafe {
                    let mut nid: NOTIFYICONDATAW = zeroed();
                    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
                    nid.hWnd = state.hwnd as HWND;
                    nid.uID = 1;
                    nid.uFlags = NIF_TIP;
                    nid.szTip[..tip_wide.len()].copy_from_slice(&tip_wide);
                    Shell_NotifyIconW(NIM_MODIFY, &nid);
                }
            }
        });
    }

    pub fn set_tray_menu(&self, _tray: &TrayHandle, items: Vec<TrayMenuItem>) {
        TRAY_STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                state.menu = items;
            }
        });
    }

    /// 在指定窗口上弹出上下文菜单喵(同步阻塞,返回选中项下标;取消为 None)
    ///
    /// `screen` 为菜单弹出点的**屏幕坐标**;岛体应用卡片右键菜单走这里喵。
    pub fn show_context_menu(
        &self,
        window: &PlatformWindow,
        items: &[TrayMenuItem],
        screen_x: i32,
        screen_y: i32,
    ) -> Option<usize> {
        show_popup_menu(window.hwnd() as HWND, items, screen_x, screen_y)
    }

    pub fn run(&self) {
        use windows_sys::Win32::Media::{timeBeginPeriod, timeEndPeriod};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, TranslateMessage, MSG,
        };

        // 把系统定时器分辨率提到 1ms:Windows 默认 15.6ms 量化,
        // 会让 WM_TIMER 帧间隔在 15/31ms 间抖动,动画观感「卡」喵。
        unsafe { timeBeginPeriod(1) };

        let mut msg: MSG = unsafe { zeroed() };
        // 全局消息泵: GetMessageW 返回 0 表示收到 WM_QUIT,退出喵
        while unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } > 0 {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        unsafe { timeEndPeriod(1) };
        log::debug!("消息循环退出喵");
    }

    pub fn quit(&self) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
        }
    }

    pub fn scale_factor(&self) -> f32 {
        unsafe { windows_sys::Win32::UI::HiDpi::GetDpiForSystem() as f32 / 96.0 }
    }

    pub fn screen_size(&self) -> (i32, i32) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
    }

    pub fn display_refresh_rate(&self) -> u32 {
        use windows_sys::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
        let mut dm: DEVMODEW = unsafe { zeroed() };
        let ok = unsafe {
            EnumDisplaySettingsW(std::ptr::null(), ENUM_CURRENT_SETTINGS, &mut dm)
        };
        if ok != 0 && dm.dmDisplayFrequency > 0 {
            dm.dmDisplayFrequency
        } else {
            log::debug!("无法读取显示刷新率,回退 60Hz 喵");
            60
        }
    }

    pub fn platform_name(&self) -> &'static str {
        "windows"
    }
}

// ---------------------------------------------------------------------------
// 窗口过程: 把 WM_* 消息翻译成平台无关事件喵
// ---------------------------------------------------------------------------

/// 主窗口消息处理喵
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    use windows_sys::Win32::Graphics::Gdi::ValidateRect;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, SetFocus, VK_BACK};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, IDC_ARROW, LoadCursorW, SetCursor, WM_ACTIVATE, WM_CHAR, WM_CLOSE,
        WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCDESTROY,
        WM_PAINT, WM_RBUTTONUP, WM_SETCURSOR, WM_SETTINGCHANGE, WM_SYSKEYDOWN, WM_TIMER,
    };

    match msg {
        // 热键触发(热键线程 PostMessage 过来)喵
        WM_MEOW_HOTKEY => {
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::Hotkey));
            0
        }
        // 扫描热键触发(后台扫描应用,不呼出岛)喵
        WM_MEOW_SCAN => {
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::ScanHotkey));
            0
        }
        // IPC 命令到达(named-pipe 服务端线程投递,逐条派发给启动器)喵
        WM_MEOW_IPC => {
            while let Some(cmd) = drain_ipc_queue() {
                with_window_handler(hwnd, |h| h.on_event(WindowEvent::IpcCommand(cmd)));
            }
            0
        }
        // 按键按下(含 Alt 组合的 WM_SYSKEYDOWN): 导航键 + 热键组合录制喵
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let vk = wparam as u16;
            if msg == WM_KEYDOWN {
                if let Some(key) = map_key(vk) {
                    with_window_handler(hwnd, |h| h.on_event(WindowEvent::KeyDown(key)));
                }
                // 特殊: 退格键不会产生 WM_CHAR,单独处理喵
                if vk == VK_BACK {
                    with_window_handler(hwnd, |h| h.on_event(WindowEvent::KeyDown(Key::Backspace)));
                }
            }
            // 热键录制: 修饰键 + 主键组合喵(各窗口 handler 按需消费)喵
            if let Some(name) = vk_name(vk) {
                with_window_handler(hwnd, |h| h.on_event(WindowEvent::HotkeyChord {
                    modifiers: current_modifiers(),
                    key: name,
                }));
            }
            0
        }
        // 字符输入喵(IME 提交字也会走这里,GCS_RESULTSTR 是兜底)喵
        WM_CHAR => {
            let code = wparam as u32;
            // 只收 ASCII 可打印字符; 中文由 GCS_RESULTSTR 提交,避免重复喵
            if (0x20..0x7F).contains(&code)
                && let Some(ch) = char::from_u32(code) {
                    with_window_handler(hwnd, |h| h.on_event(WindowEvent::Char(ch)));
                }
            0
        }
        // IME 组字: 预览串 + 提交结果喵
        WM_IME_COMPOSITION => {
            handle_ime_composition(hwnd, lparam);
            0
        }
        // 鼠标左键按下喵(捕获鼠标: 拖选文本/拖滑块可在窗口外持续跟随)喵
        WM_LBUTTONDOWN => {
            let (x, y) = unpack_lparam(lparam);
            unsafe { SetCapture(hwnd) };
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::MouseDown(x, y)));
            0
        }
        WM_MOUSEMOVE => {
            let (x, y) = unpack_lparam(lparam);
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::MouseMove(x, y)));
            0
        }
        WM_LBUTTONUP => {
            unsafe { ReleaseCapture() };
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::MouseUp));
            0
        }
        // 右键松开: 请求上下文菜单(坐标随事件下发,由业务层决定弹什么)喵
        WM_RBUTTONUP => {
            let (x, y) = unpack_lparam(lparam);
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::ContextMenu(x, y)));
            0
        }
        // 光标由应用显式指定,避免系统误切换到「忙/加载」指针喵
        WM_SETCURSOR => {
            unsafe {
                SetCursor(LoadCursorW(null_mut(), IDC_ARROW));
            }
            1
        }
        // 鼠标滚轮喵
        WM_MOUSEWHEEL => {
            let delta = ((wparam >> 16) & 0xFFFF) as u16 as i16 as f32;
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::MouseWheel(delta)));
            0
        }
        // 拖入文件喵
        WM_DROPFILES => {
            let paths = collect_dropped_files(wparam);
            if !paths.is_empty() {
                with_window_handler(hwnd, |h| h.on_event(WindowEvent::FilesDropped(paths)));
            }
            0
        }
        // 激活状态变化喵
        WM_ACTIVATE => {
            let active = (wparam as u32 & 0xFFFF) != 0; // WA_INACTIVE = 0 喵
            if active {
                // 分层置顶窗被激活时键盘焦点偶有漂移,这里补钉一次;
                // 少这一下,输入会被系统当成无效输入丢弃并响一声提示音喵。
                unsafe { SetFocus(hwnd) };
            } else {
                with_window_handler(hwnd, |h| h.on_event(WindowEvent::LostFocus));
            }
            0
        }
        // 系统设置变更: 只关心环境变量那一条,立刻刷新本进程环境块喵
        // (用户在系统属性里改完变量就会广播到这里,不用等下次启动子进程)喵
        WM_SETTINGCHANGE => {
            if is_environment_change(lparam) {
                refresh_process_environment();
            }
            0
        }
        // 动画帧定时器喵
        WM_TIMER => {
            if wparam == TIMER_ID {
                with_window_handler(hwnd, |h| h.on_event(WindowEvent::Timer));
            }
            0
        }
        // 忽略系统重绘请求(我们主动渲染喵)
        WM_PAINT => {
            unsafe { ValidateRect(hwnd, null_mut()) };
            0
        }
        // 请求关闭喵(业务层决定隐藏还是退出)喵
        WM_CLOSE => {
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::Close));
            0
        }
        // 窗口销毁,回收 handler 喵(同样防重入: 销毁也可能同步派发消息)喵
        WM_NCDESTROY => {
            HANDLERS.with(|handlers| {
                if let Ok(mut handlers) = handlers.try_borrow_mut() {
                    handlers.remove(&(hwnd as usize));
                }
            });
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 取指定窗口的 handler 并执行回调喵
///
/// 用 try_borrow_mut 防重入: ShowWindow/SetWindowPos 等 API 会同步派发消息,
/// 在 handler 回调内再次触发 wnd_proc 时,这里返回 None 跳过本次消息,
/// 避免 RefCell 双重可变借用 panic 喵。
fn with_window_handler<R>(hwnd: HWND, f: impl FnOnce(&mut dyn WindowHandler) -> R) -> Option<R> {
    HANDLERS.with(|handlers| {
        let mut handlers = handlers.try_borrow_mut().ok()?;
        handlers.get_mut(&(hwnd as usize)).map(|h| f(h.as_mut()))
    })
}

/// 从 IPC 队列取走一条命令喵(主线程 wnd_proc 里逐条取)喵
fn drain_ipc_queue() -> Option<super::IpcCommand> {
    IPC_QUEUE.lock().ok()?.pop_front()
}

/// named-pipe IPC 服务端主循环喵
///
/// 每轮: 建管道 → 等连接 → 读一行命令 → 入队 + 通知主线程 → 回执 → 断开喵。
/// 命令执行发生在主线程(窗口操作只能在那里做),本线程只负责收发喵。
fn ipc_server_loop(hwnd: HWND) {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_PIPE_CONNECTED, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        FlushFileBuffers, ReadFile, WriteFile, PIPE_ACCESS_DUPLEX,
    };
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE,
        PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

    let name: Vec<u16> = super::IPC_PIPE_NAME.encode_utf16().chain(Some(0)).collect();
    loop {
        let handle = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                4096,
                4096,
                0,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            let rc = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            log::warn!("IPC 管道创建失败(rc={rc}),3 秒后重试喵~");
            std::thread::sleep(std::time::Duration::from_secs(3));
            continue;
        }

        // 等客户端连接喵(客户端可能在 ConnectNamedPipe 前就 CreateFile,补判已连接)喵
        let connected =
            unsafe { ConnectNamedPipe(handle, null_mut()) != 0 } || unsafe { windows_sys::Win32::Foundation::GetLastError() } == ERROR_PIPE_CONNECTED;
        if !connected {
            log::debug!("IPC 客户端连接中止喵");
            unsafe { CloseHandle(handle) };
            continue;
        }

        // 读一行命令(以 \n 结尾;客户端写完即等响应)喵
        let mut acc: Vec<u8> = Vec::new();
        unsafe {
            let mut chunk = [0u8; 1024];
            while acc.len() < 4096 {
                let mut read = 0u32;
                if ReadFile(handle, chunk.as_mut_ptr(), chunk.len() as u32, &mut read, null_mut()) == 0
                    || read == 0
                {
                    break;
                }
                acc.extend_from_slice(&chunk[..read as usize]);
                if acc.contains(&b'\n') {
                    break;
                }
            }
        }
        let line = String::from_utf8_lossy(&acc);
        let line = line.lines().next().unwrap_or("");

        // 入队 + 通知主线程喵
        let reply = match super::parse_ipc_command(line) {
            Some(cmd) => {
                if let Ok(mut q) = IPC_QUEUE.lock() {
                    q.push_back(cmd);
                }
                unsafe { PostMessageW(hwnd, WM_MEOW_IPC, 0, 0) };
                "ok 已投递喵"
            }
            None => super::IPC_USAGE_HINT,
        };

        // 回执 + 断开,准备下一个客户端喵
        unsafe {
            let mut written = 0u32;
            WriteFile(handle, reply.as_ptr(), reply.len() as u32, &mut written, null_mut());
            FlushFileBuffers(handle);
            DisconnectNamedPipe(handle);
            CloseHandle(handle);
        }
        log::debug!("IPC 命令已处理: {line:?} → {reply} 喵");
    }
}

/// IPC 客户端喵: 连接管道发送一行命令,等服务端回执后返回喵
fn send_ipc_command_impl(line: &str) -> Result<String, String> {
    use windows_sys::Win32::Foundation::{
        CloseHandle, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING,
    };

    let name: Vec<u16> = super::IPC_PIPE_NAME.encode_utf16().chain(Some(0)).collect();
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err("实例未运行,管道不可达喵".into());
    }

    let bytes = format!("{line}\n");
    let mut acc: Vec<u8> = Vec::new();
    let sent;
    unsafe {
        let mut written = 0u32;
        sent = WriteFile(handle, bytes.as_ptr(), bytes.len() as u32, &mut written, null_mut()) != 0;
        // 服务端回执完即断开,读到 EOF 为止喵
        let mut chunk = [0u8; 512];
        loop {
            let mut read = 0u32;
            if ReadFile(handle, chunk.as_mut_ptr(), chunk.len() as u32, &mut read, null_mut()) == 0
                || read == 0
            {
                break;
            }
            acc.extend_from_slice(&chunk[..read as usize]);
            if acc.contains(&b'\n') {
                break;
            }
        }
        CloseHandle(handle);
    }
    if !sent {
        return Err("发送命令失败喵".into());
    }
    let reply = String::from_utf8_lossy(&acc).trim().to_string();
    if reply.is_empty() {
        return Err("服务端无回执喵".into());
    }
    Ok(reply)
}

/// 托盘隐藏窗口消息处理喵
unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, WM_COMMAND, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP,
    };

    match msg {
        // explorer 重启/启动时会广播 TaskbarCreated,托盘图标会丢,这里重建喵
        msg if Some(msg) == WM_TRAY_TASKBAR.get().copied() => {
            log::debug!("收到 TaskbarCreated 广播,重建托盘图标喵~");
            let (sw, sh) = small_icon_size();
            let hicon = app_icon_hicon(sw, sh);
            if !hicon.is_null() {
                TRAY_STATE.with(|s| {
                    if let Some(state) = s.borrow_mut().as_mut() {
                        let old = std::mem::replace(&mut state.icon, hicon);
                        unsafe {
                            let mut nid: NOTIFYICONDATAW = zeroed();
                            nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
                            nid.hWnd = state.hwnd as HWND;
                            nid.uID = 1;
                            nid.uFlags = NIF_ICON;
                            nid.hIcon = hicon;
                            // 旧图标槽位已随 explorer 消失,用 NIM_ADD 重建喵
                            if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
                                Shell_NotifyIconW(NIM_MODIFY, &nid);
                            }
                        }
                        if !old.is_null() {
                            unsafe {
                                windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon(old);
                            }
                        }
                    }
                });
            }
            0
        }
        // 托盘回调消息喵
        WM_TRAY_MSG => {
            let event = (lparam & 0xFFFF) as u32;
            if event == WM_LBUTTONUP || event == NIN_SELECT {
                log::debug!("托盘左键点击喵");
                with_tray_handler(|h| h.on_event(TrayEvent::LeftClick));
            } else if event == WM_RBUTTONUP {
                log::debug!("托盘右键菜单喵");
                show_tray_menu_impl(hwnd);
            }
            0
        }
        // 菜单项点击喵
        WM_COMMAND => {
            let id = wparam & 0xFFFF;
            if id > 0 {
                with_tray_handler(|h| h.on_event(TrayEvent::Menu(id - 1)));
            }
            0
        }
        WM_DESTROY => {
            TRAY_HANDLER.with(|h| *h.borrow_mut() = None);
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 取托盘 handler 并执行回调喵(同样用 try_borrow_mut 防重入)喵
fn with_tray_handler<R>(f: impl FnOnce(&mut dyn TrayHandler) -> R) -> Option<R> {
    TRAY_HANDLER.with(|h| {
        let mut h = h.try_borrow_mut().ok()?;
        h.as_mut().map(|handler| f(handler.as_mut()))
    })
}

/// 弹出托盘菜单喵(用当前存储的菜单项)喵
fn show_tray_menu_impl(hwnd: HWND) {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let items = TRAY_STATE.with(|s| {
        s.borrow().as_ref().map(|state| state.menu.clone()).unwrap_or_default()
    });
    if items.is_empty() {
        return;
    }

    // 在光标处弹出喵
    let mut pt: POINT = unsafe { zeroed() };
    unsafe { GetCursorPos(&mut pt) };
    if let Some(index) = show_popup_menu(hwnd, &items, pt.x, pt.y) {
        with_tray_handler(|h| h.on_event(TrayEvent::Menu(index)));
    }
}

/// 弹出系统上下文菜单喵(同步阻塞,返回选中项下标;取消/点外部为 None)喵
///
/// `x`/`y` 为**屏幕坐标**;弹菜单前把宿主窗口设为前台,点击外部才能正常关闭喵。
/// 托盘菜单与岛体应用卡片的右键菜单共用喵。
fn show_popup_menu(hwnd: HWND, items: &[TrayMenuItem], x: i32, y: i32) -> Option<usize> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, PostMessageW, SetForegroundWindow,
        TrackPopupMenu, MF_ENABLED, MF_GRAYED, MF_STRING, TPM_LEFTALIGN, TPM_RETURNCMD,
        TPM_RIGHTBUTTON, WM_NULL,
    };

    if items.is_empty() {
        return None;
    }
    unsafe {
        let hmenu = CreatePopupMenu();
        if hmenu.is_null() {
            return None;
        }

        // 逐个追加菜单项喵(菜单项 id = 索引 + 1,0 保留给「取消」)喵
        for (i, item) in items.iter().enumerate() {
            let label: Vec<u16> = item.label.encode_utf16().chain(Some(0)).collect();
            let flags = MF_STRING | if item.enabled { MF_ENABLED } else { MF_GRAYED };
            AppendMenuW(hmenu, flags, i + 1, label.as_ptr());
        }

        // TPM_RETURNCMD: 选中项同步返回,不用再经 WM_COMMAND 绕一圈喵
        SetForegroundWindow(hwnd);
        let chosen = TrackPopupMenu(
            hmenu,
            TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_RETURNCMD,
            x,
            y,
            0,
            hwnd,
            null_mut(),
        );
        // 菜单点完后必须再丢一条空消息,否则宿主窗口下次弹不出菜单喵
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(hmenu);
        (chosen > 0).then(|| chosen as usize - 1)
    }
}

/// 虚拟键码 → 导航键喵
fn map_key(vk: u16) -> Option<Key> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_NEXT, VK_PRIOR, VK_RETURN,
        VK_RIGHT, VK_TAB, VK_UP,
    };
    match vk {
        VK_UP => Some(Key::Up),
        VK_DOWN => Some(Key::Down),
        VK_LEFT => Some(Key::Left),
        VK_RIGHT => Some(Key::Right),
        VK_RETURN => Some(Key::Enter),
        VK_ESCAPE => Some(Key::Escape),
        VK_TAB => Some(Key::Tab),
        VK_HOME => Some(Key::Home),
        VK_END => Some(Key::End),
        VK_PRIOR => Some(Key::PageUp),
        VK_NEXT => Some(Key::PageDown),
        VK_DELETE => Some(Key::Delete),
        _ => None,
    }
}

/// 虚拟键码 → 热键主键名喵(与 parse_hotkey 词表一致)喵
fn vk_name(vk: u16) -> Option<String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        VK_BACK, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB,
        VK_UP,
    };
    let name = match vk {
        0x30..=0x39 => char::from_u32(vk as u32)?.to_string(),                        // 0-9
        0x41..=0x5A => char::from_u32(vk as u32)?.to_ascii_lowercase().to_string(),  // a-z
        0x70..=0x87 => format!("f{}", vk - 0x70 + 1),                              // f1-f24
        VK_SPACE => "space".into(),
        VK_RETURN => "enter".into(),
        VK_ESCAPE => "esc".into(),
        VK_TAB => "tab".into(),
        VK_BACK => "backspace".into(),
        VK_DELETE => "delete".into(),
        VK_UP => "up".into(),
        VK_DOWN => "down".into(),
        VK_LEFT => "left".into(),
        VK_RIGHT => "right".into(),
        0xC0 => "tilde".into(),
        _ => return None,
    };
    Some(name)
}

/// 当前按住的修饰键组合("ctrl+alt" 风格,与 parse_hotkey 一致)喵
fn current_modifiers() -> String {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
    let mut parts = Vec::new();
    unsafe {
        // 高位置 1 表示键处于按下状态喵
        if (GetKeyState(0x11) & 0x80) != 0 {
            parts.push("ctrl");
        }
        if (GetKeyState(0x12) & 0x80) != 0 {
            parts.push("alt");
        }
        if (GetKeyState(0x10) & 0x80) != 0 {
            parts.push("shift");
        }
        if (GetKeyState(0x5B) & 0x80) != 0 || (GetKeyState(0x5C) & 0x80) != 0 {
            parts.push("win");
        }
    }
    parts.join("+")
}

/// 给分层窗挂上默认 IME 上下文,否则中文输入法常常出不来喵
fn associate_ime(hwnd: HWND) {
    use windows_sys::Win32::UI::Input::Ime::{ImmAssociateContextEx, IACE_DEFAULT};
    unsafe {
        ImmAssociateContextEx(hwnd, null_mut(), IACE_DEFAULT);
    }
}

/// 处理 IME 组字消息喵: 预览串走 ImePreedit,提交字走 Char喵
fn handle_ime_composition(hwnd: HWND, lparam: LPARAM) {
    use windows_sys::Win32::UI::Input::Ime::{
        ImmGetContext, ImmReleaseContext, GCS_COMPSTR, GCS_RESULTSTR,
    };

    let himc = unsafe { ImmGetContext(hwnd) };
    if himc.is_null() {
        return;
    }

    let flag = lparam as u32;
    if flag & GCS_RESULTSTR != 0 {
        if let Some(text) = ime_string(himc, GCS_RESULTSTR) {
            for ch in text.chars().filter(|c| !c.is_control()) {
                with_window_handler(hwnd, |h| h.on_event(WindowEvent::Char(ch)));
            }
        }
        with_window_handler(hwnd, |h| h.on_event(WindowEvent::ImePreedit(String::new())));
    } else if flag & GCS_COMPSTR != 0 {
        let text = ime_string(himc, GCS_COMPSTR).unwrap_or_default();
        with_window_handler(hwnd, |h| h.on_event(WindowEvent::ImePreedit(text)));
    }

    unsafe {
        ImmReleaseContext(hwnd, himc);
    }
}

/// 从 IME 上下文读宽字符串喵
fn ime_string(himc: windows_sys::Win32::UI::Input::Ime::HIMC, flag: u32) -> Option<String> {
    use windows_sys::Win32::UI::Input::Ime::ImmGetCompositionStringW;
    unsafe {
        let bytes = ImmGetCompositionStringW(himc, flag, null_mut(), 0);
        if bytes <= 0 {
            return Some(String::new());
        }
        let units = bytes as usize / 2;
        let mut buf = vec![0u16; units];
        let written = ImmGetCompositionStringW(
            himc,
            flag,
            buf.as_mut_ptr() as *mut _,
            bytes as u32,
        );
        if written <= 0 {
            return None;
        }
        let n = (written as usize / 2).min(buf.len());
        Some(String::from_utf16_lossy(&buf[..n]))
    }
}

/// 收集拖入的文件路径喵
fn collect_dropped_files(wparam: WPARAM) -> Vec<String> {
    use windows_sys::Win32::UI::Shell::{DragFinish, DragQueryFileW, HDROP};
    let hdrop = wparam as HDROP;
    let mut paths = Vec::new();
    unsafe {
        // 查文件数必须用哨兵值 0xFFFFFFFF(= -1),写成 0xFFFF(65535) 会被当成文件索引,
        // 返回 0 → 拖入永远没反应喵。
        let count = DragQueryFileW(hdrop, u32::MAX, null_mut(), 0);
        for i in 0..count {
            let len = DragQueryFileW(hdrop, i, null_mut(), 0) as usize;
            if len == 0 {
                continue;
            }
            let mut buf = vec![0u16; len + 1];
            DragQueryFileW(hdrop, i, buf.as_mut_ptr(), buf.len() as u32);
            if let Some(end) = buf.iter().position(|&c| c == 0) {
                buf.truncate(end);
            }
            paths.push(String::from_utf16_lossy(&buf));
        }
        DragFinish(hdrop);
    }
    paths
}

/// 从 lparam 提取客户区坐标(物理像素)喵
fn unpack_lparam(lparam: LPARAM) -> (f32, f32) {
    let x = (lparam & 0xFFFF) as u16 as i16 as f32;
    let y = ((lparam >> 16) & 0xFFFF) as u16 as i16 as f32;
    (x, y)
}

/// 注册主窗口类喵
fn register_class() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{RegisterClassW, WNDCLASSW};
    let wc = WNDCLASSW {
        lpfnWndProc: Some(wnd_proc),
        lpszClassName: CLASS_NAME.as_ptr(),
        hInstance: unsafe {
            windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(null_mut())
        },
        hbrBackground: null_mut(),
        ..unsafe { zeroed() }
    };
    if unsafe { RegisterClassW(&wc) } == 0 {
        log::error!("主窗口类注册失败喵");
    } else {
        log::debug!("主窗口类注册成功喵");
    }
}

/// 注册托盘窗口类喵
fn register_tray_class() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{RegisterClassW, WNDCLASSW};
    let wc = WNDCLASSW {
        lpfnWndProc: Some(tray_wnd_proc),
        lpszClassName: TRAY_CLASS_NAME.as_ptr(),
        hInstance: unsafe {
            windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(null_mut())
        },
        hbrBackground: null_mut(),
        ..unsafe { zeroed() }
    };
    if unsafe { RegisterClassW(&wc) } == 0 {
        log::error!("托盘窗口类注册失败喵");
    }
}

/// 声明进程 DPI 感知喵(窗口尺寸/坐标都按物理像素处理)喵
fn enable_dpi_awareness() {
    use windows_sys::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

// ---------------------------------------------------------------------------
// 每像素透明呈现: UpdateLayeredWindow
// ---------------------------------------------------------------------------

/// 开机自启键名喵(注册表 Run 值)
const AUTO_START_VALUE: &str = "MeowAppLauncher";

/// 设置开机自启喵: 写入/删除 `HKCU\...\CurrentVersion\Run` 喵
///
/// 开启时写入 `"当前 exe 路径"`(带引号,路径含空格安全);
/// 关闭时删除对应值(值不存在也视为成功,保证幂等)喵。
fn set_auto_start_impl(enabled: bool) -> bool {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };
    use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const REG_NONE: *const u16 = std::ptr::null();
    let key_wide: Vec<u16> = RUN_KEY.encode_utf16().chain(Some(0)).collect();
    let value_wide: Vec<u16> = AUTO_START_VALUE.encode_utf16().chain(Some(0)).collect();

    unsafe {
        let mut key: HKEY = null_mut();
        // 先尝试打开(通常已存在)喵
        let mut opened = windows_sys::Win32::System::Registry::RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_wide.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut key,
        ) == 0;
        if !opened {
            // 不存在则创建喵
            let mut disp = 0u32;
            opened = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                key_wide.as_ptr(),
                0,
                REG_NONE,
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                null_mut(),
                &mut key,
                &mut disp,
            ) == 0;
        }
        if !opened {
            log::error!("开机自启: 无法打开/创建 Run 注册表键喵");
            return false;
        }

        let ok = if enabled {
            match std::env::current_exe() {
                Ok(exe) => {
                    let cmd = format!("\"{}\"", exe.to_string_lossy());
                    let data: Vec<u16> = cmd.encode_utf16().chain(Some(0)).collect();
                    let rc = RegSetValueExW(
                        key,
                        value_wide.as_ptr(),
                        0,
                        REG_SZ,
                        data.as_ptr() as *const u8,
                        (data.len() * 2) as u32,
                    );
                    if rc == 0 {
                        log::info!("开机自启已开启: {cmd} 喵");
                    } else {
                        log::error!("开机自启: 写注册表失败(rc={rc})喵");
                    }
                    rc == 0
                }
                Err(e) => {
                    log::error!("开机自启: 无法获取当前可执行路径({e})喵");
                    false
                }
            }
        } else {
            let rc = RegDeleteValueW(key, value_wide.as_ptr());
            if rc == 0 {
                log::info!("开机自启已关闭喵");
                true
            } else if rc == ERROR_FILE_NOT_FOUND {
                // 本来就没开,幂等成功喵
                log::debug!("开机自启本就未开启,跳过删除喵");
                true
            } else {
                log::error!("开机自启: 删除注册表值失败(rc={rc})喵");
                false
            }
        };

        let _ = RegCloseKey(key);
        ok
    }
}

/// 把可执行文件目录加入用户 PATH 喵(让 `meowal` 在终端可用)喵
///
/// 写入 `HKCU\Environment\Path`(保留原 REG_EXPAND_SZ 类型与既有条目),
/// 已存在则幂等跳过;成功后广播 WM_SETTINGCHANGE 让已开终端生效喵。
fn install_cli_command_impl() -> bool {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_READ, KEY_SET_VALUE, REG_EXPAND_SZ,
    };

    const ENV_KEY: &str = "Environment";
    const VALUE: &str = "Path";

    let Ok(exe) = std::env::current_exe() else {
        log::error!("CLI 注册: 无法获取当前可执行路径喵");
        return false;
    };
    let Some(exe_dir) = exe.parent().map(|d| d.to_string_lossy().to_string()) else {
        return false;
    };

    let key_wide: Vec<u16> = ENV_KEY.encode_utf16().chain(Some(0)).collect();
    let value_wide: Vec<u16> = VALUE.encode_utf16().chain(Some(0)).collect();

    unsafe {
        let mut key: HKEY = null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, key_wide.as_ptr(), 0, KEY_READ | KEY_SET_VALUE, &mut key) != 0 {
            log::error!("CLI 注册: 无法打开 HKCU\\{ENV_KEY} 喵");
            return false;
        }

        // 读现有 Path(记录类型,保留 REG_EXPAND_SZ 语义)喵
        let mut path_type: u32 = REG_EXPAND_SZ;
        let mut size: u32 = 0;
        let mut buf: Vec<u16> = Vec::new();
        let exists = RegQueryValueExW(key, value_wide.as_ptr(), null_mut(), &mut path_type, null_mut(), &mut size) == 0
            && size > 0;
        if exists {
            buf.resize((size as usize) / 2 + 1, 0);
            let mut tmp = size;
            if RegQueryValueExW(
                key,
                value_wide.as_ptr(),
                null_mut(),
                &mut path_type,
                buf.as_mut_ptr() as *mut u8,
                &mut tmp,
            ) != 0
            {
                buf.clear();
            }
            while buf.last() == Some(&0) {
                buf.pop();
            }
        }

        // 已包含则幂等成功喵(条目以 ; 分隔,大小写不敏感)喵
        let existing = String::from_utf16_lossy(&buf);
        if existing.split(';').any(|e| e.eq_ignore_ascii_case(&exe_dir)) {
            let _ = RegCloseKey(key);
            log::debug!("CLI 已在用户 PATH 中喵");
            return true;
        }

        // 追加可执行目录喵
        let mut new_path = existing;
        if !new_path.is_empty() && !new_path.ends_with(';') {
            new_path.push(';');
        }
        new_path.push_str(&exe_dir);
        let data: Vec<u16> = new_path.encode_utf16().chain(Some(0)).collect();
        let rc = RegSetValueExW(
            key,
            value_wide.as_ptr(),
            0,
            if exists { path_type } else { REG_EXPAND_SZ },
            data.as_ptr() as *const u8,
            (data.len() * 2) as u32,
        );
        let _ = RegCloseKey(key);
        if rc != 0 {
            log::error!("CLI 注册: 写入 PATH 失败(rc={rc})喵");
            return false;
        }

        broadcast_environment_change();
        log::info!("CLI 已加入用户 PATH: {exe_dir} 喵");
        true
    }
}

/// 广播环境变量变更,让已打开的终端/资源管理器识别新 PATH 喵
///
/// HWND_BROADCAST 会遍历所有顶层窗口,可能耗时,故放独立线程 + 短超时,
/// 绝不阻塞应用启动喵。
fn broadcast_environment_change() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };
    std::thread::Builder::new()
        .name("meow-env-broadcast".into())
        .spawn(move || unsafe {
            let env: Vec<u16> = "Environment\0".encode_utf16().collect();
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                env.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                200,
                null_mut(),
            );
        })
        .ok();
}

// ---------------------------------------------------------------------------
// 进程环境变量同步喵
// ---------------------------------------------------------------------------

/// 增量同步状态喵(记住上一轮同步过哪些变量)喵
static ENV_SYNC: LazyLock<Mutex<env::EnvSync>> =
    LazyLock::new(|| Mutex::new(env::EnvSync::default()));

/// 把本进程的环境块对齐到注册表里的最新值喵
///
/// 为什么要这么麻烦: 进程环境块是启动瞬间的快照,而我们**常驻不重启**,
/// 用户在「系统属性 → 环境变量」里改完东西后,从我们这儿拉起的子进程
/// (终端/编辑器…)拿到的还是旧值,只有重启应用才刷新——这显然不合理喵。
///
/// 于是改成: 读注册表两级环境键 → 合并展开 → 与本进程做差量 → 写回。
/// 写的是**本进程自己**的环境块,子进程照旧继承,不必特殊照顾喵。
pub fn refresh_process_environment() -> bool {
    use windows_sys::Win32::System::Environment::SetEnvironmentVariableW;

    let system = read_registry_env(MachineScope::System);
    if system.is_empty() {
        log::debug!("环境变量同步跳过: 系统环境键读不到内容喵");
        return false;
    }
    let user = read_registry_env(MachineScope::User);

    let merged = env::merge(&system, &user);
    let fresh = env::expand_all(&merged, |name| std::env::var(name).ok());

    let plan = match ENV_SYNC.lock() {
        Ok(mut sync) => sync.plan(&fresh),
        Err(_) => {
            log::warn!("环境变量同步跳过: 状态锁已中毒喵");
            return false;
        }
    };

    // 先算出「真正变了的」数量,日志才有信息量(首次同步几乎每条都算写入)喵
    let changed = plan
        .set
        .iter()
        .filter(|(name, value)| std::env::var(name).ok().as_ref() != Some(value))
        .count();

    unsafe {
        for (name, value) in &plan.set {
            let name_wide = to_wide(name);
            let value_wide = to_wide(value);
            SetEnvironmentVariableW(name_wide.as_ptr(), value_wide.as_ptr());
        }
        for name in &plan.remove {
            // 传 NULL 即删除,和用户手动删掉变量的语义一致喵
            let name_wide = to_wide(name);
            SetEnvironmentVariableW(name_wide.as_ptr(), std::ptr::null());
        }
    }

    if changed > 0 || !plan.remove.is_empty() {
        log::info!(
            "环境变量已同步: 更新 {changed} 项, 移除 {} 项喵",
            plan.remove.len()
        );
    } else {
        log::debug!("环境变量无需变更(共 {} 项)喵", plan.set.len());
    }
    true
}

/// 注册表环境键的归属喵
#[derive(Clone, Copy)]
enum MachineScope {
    /// `HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment` 喵
    System,
    /// `HKCU\Environment` 喵
    User,
}

/// 读一个注册表环境键下的所有字符串值喵
///
/// 只收 `REG_SZ` / `REG_EXPAND_SZ`(环境键里也只会有这两种),
/// 展开与否交给 [`env::expand_all`],这里原样搬运喵。
fn read_registry_env(scope: MachineScope) -> Vec<env::EnvEntry> {
    use windows_sys::Win32::Foundation::ERROR_MORE_DATA;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_EXPAND_SZ, REG_SZ, RegCloseKey,
        RegEnumValueW, RegOpenKeyExW,
    };

    const SYSTEM_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
    const USER_KEY: &str = "Environment";

    let (root, sub): (HKEY, &str) = match scope {
        MachineScope::System => (HKEY_LOCAL_MACHINE, SYSTEM_KEY),
        MachineScope::User => (HKEY_CURRENT_USER, USER_KEY),
    };
    let sub_wide = to_wide(sub);

    let mut out = Vec::new();
    unsafe {
        let mut key: HKEY = null_mut();
        if RegOpenKeyExW(root, sub_wide.as_ptr(), 0, KEY_READ, &mut key) != 0 {
            log::debug!("环境变量同步: 读不到注册表键 {sub} 喵");
            return out;
        }

        let mut name_buf = vec![0u16; 512];
        let mut data_buf = vec![0u16; 16 * 1024];
        let mut index = 0u32;
        loop {
            let mut name_len = name_buf.len() as u32;
            let mut data_len = (data_buf.len() * 2) as u32;
            let mut kind = 0u32;
            let rc = RegEnumValueW(
                key,
                index,
                name_buf.as_mut_ptr(),
                &mut name_len,
                std::ptr::null(),
                &mut kind,
                data_buf.as_mut_ptr() as *mut u8,
                &mut data_len,
            );
            if rc == ERROR_MORE_DATA {
                // 值比缓冲区还长(极端 PATH 才会有),扩容后重试同一个索引喵
                data_buf.resize(data_buf.len() * 2, 0);
                continue;
            }
            if rc != 0 {
                break; // 枚举结束或出错,统一收尾喵
            }
            index += 1;
            if kind != REG_SZ && kind != REG_EXPAND_SZ {
                continue;
            }
            let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            let chars = (data_len as usize / 2).min(data_buf.len());
            let value = String::from_utf16_lossy(&data_buf[..chars]);
            out.push(env::EnvEntry {
                name,
                value: value.trim_end_matches('\0').to_string(),
                expand: kind == REG_EXPAND_SZ,
            });
        }
        let _ = RegCloseKey(key);
    }
    out
}

/// 判断 `WM_SETTINGCHANGE` 的 lParam 是不是 "Environment" 喵
///
/// lParam 是系统给的字符串指针,可能为空;最多扫视 32 个字符就收工,
/// 不会读到不该读的地方喵。
fn is_environment_change(lparam: LPARAM) -> bool {
    const MAX_LEN: usize = 32;
    if lparam == 0 {
        return false;
    }
    let ptr = lparam as *const u16;
    unsafe {
        let mut len = 0usize;
        while len < MAX_LEN && *ptr.add(len) != 0 {
            len += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
        text.eq_ignore_ascii_case("Environment")
    }
}

// ---------------------------------------------------------------------------
// GPU 渲染上下文: WGL / OpenGL
// ---------------------------------------------------------------------------

/// WGL 像素格式描述符喵(windows-sys 0.59 未提供,按标准签名手写)喵
#[repr(C)]
struct PixelFormatDescriptor {
    n_size: u16,
    n_version: u16,
    dw_flags: u32,
    i_pixel_type: u8,
    c_color_bits: u8,
    c_red_bits: u8,
    c_red_shift: u8,
    c_green_bits: u8,
    c_green_shift: u8,
    c_blue_bits: u8,
    c_blue_shift: u8,
    c_alpha_bits: u8,
    c_alpha_shift: u8,
    c_accum_bits: u8,
    c_accum_red_bits: u8,
    c_accum_green_bits: u8,
    c_accum_blue_bits: u8,
    c_accum_alpha_bits: u8,
    c_depth_bits: u8,
    c_stencil_bits: u8,
    c_aux_buffers: u8,
    i_layer_type: u8,
    b_reserved: u8,
    dw_layer_mask: u32,
    dw_visible_mask: u32,
    dw_damage_mask: u32,
}

const PFD_DRAW_TO_WINDOW: u32 = 0x0000_0004;
const PFD_SUPPORT_OPENGL: u32 = 0x0000_0020;
const PFD_DOUBLEBUFFER: u32 = 0x0000_0001;
const PFD_TYPE_RGBA: u8 = 0;

#[link(name = "gdi32")]
unsafe extern "system" {
    fn ChoosePixelFormat(hdc: *mut std::ffi::c_void, ppfd: *const PixelFormatDescriptor) -> i32;
    fn SetPixelFormat(
        hdc: *mut std::ffi::c_void,
        i_pixel_format: i32,
        ppfd: *const PixelFormatDescriptor,
    ) -> i32;
}

#[link(name = "opengl32")]
unsafe extern "system" {
    fn wglCreateContext(hdc: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn wglMakeCurrent(hdc: *mut std::ffi::c_void, hglrc: *mut std::ffi::c_void) -> i32;
    fn wglDeleteContext(hglrc: *mut std::ffi::c_void) -> i32;
    fn wglGetProcAddress(lp_proc_name: *const std::os::raw::c_char) -> *const std::ffi::c_void;
}

/// WGL 渲染上下文句柄喵(隐藏窗口 + 设备上下文 + GL 上下文)喵
pub struct WinGpuContext {
    /// 隐藏窗口句柄喵
    hwnd: usize,
    /// 窗口设备上下文喵
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    /// GL 上下文句柄喵
    hglrc: *mut std::ffi::c_void,
}

impl WinGpuContext {
    pub fn get_proc(&self, name: &str) -> *const std::ffi::c_void {
        let Ok(cname) = std::ffi::CString::new(name) else {
            return null_mut();
        };
        unsafe {
            let p = wglGetProcAddress(cname.as_ptr());
            if !p.is_null() {
                return p;
            }
            // 核心 1.x 函数不在扩展导出里,走 opengl32.dll 导出喵
            let module = windows_sys::Win32::System::LibraryLoader::GetModuleHandleA(
                c"opengl32.dll".as_ptr().cast(),
            );
            if module.is_null() {
                null_mut()
            } else {
                windows_sys::Win32::System::LibraryLoader::GetProcAddress(
                    module,
                    cname.as_ptr() as *const u8,
                )
                .map(|f| f as *const std::ffi::c_void)
                .unwrap_or(null_mut())
            }
        }
    }

    pub fn make_current(&self) -> bool {
        unsafe { wglMakeCurrent(self.hdc, self.hglrc) != 0 }
    }

    pub fn valid(&self) -> bool {
        !self.hglrc.is_null()
    }
}

impl Drop for WinGpuContext {
    fn drop(&mut self) {
        unsafe {
            wglMakeCurrent(null_mut(), null_mut());
            wglDeleteContext(self.hglrc);
            windows_sys::Win32::Graphics::Gdi::ReleaseDC(self.hwnd as HWND, self.hdc);
            windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(self.hwnd as HWND);
        }
    }
}

/// 创建 WGL GPU 上下文喵(隐藏窗口 + 像素格式 + GL 上下文并 make current)喵
fn create_gpu_context_impl() -> Option<WinGpuContext> {
    use windows_sys::Win32::Graphics::Gdi::{GetDC, ReleaseDC};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_POPUP};

    CLASS_ONCE.call_once(register_class);

    unsafe {
        // 隐藏 1×1 窗口承载 GL 上下文喵(不出现在任务栏/屏幕)喵
        let hwnd = CreateWindowExW(
            0,
            CLASS_NAME.as_ptr(),
            CLASS_NAME.as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null_mut()),
            null_mut(),
        );
        if hwnd.is_null() {
            log::error!("GPU: 隐藏窗口创建失败喵");
            return None;
        }
        let hdc = GetDC(hwnd);
        if hdc.is_null() {
            DestroyWindow(hwnd);
            return None;
        }

        let mut pfd: PixelFormatDescriptor = zeroed();
        pfd.n_size = size_of::<PixelFormatDescriptor>() as u16;
        pfd.n_version = 1;
        pfd.dw_flags = PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER;
        pfd.i_pixel_type = PFD_TYPE_RGBA;
        let fmt = ChoosePixelFormat(hdc, &pfd);
        if fmt == 0 || SetPixelFormat(hdc, fmt, &pfd) == 0 {
            log::warn!("GPU: 像素格式选择/设置失败喵");
            ReleaseDC(hwnd, hdc);
            DestroyWindow(hwnd);
            return None;
        }

        let hglrc = wglCreateContext(hdc);
        if hglrc.is_null() {
            log::warn!("GPU: wglCreateContext 失败喵");
            ReleaseDC(hwnd, hdc);
            DestroyWindow(hwnd);
            return None;
        }
        if wglMakeCurrent(hdc, hglrc) == 0 {
            log::warn!("GPU: wglMakeCurrent 失败喵");
            wglDeleteContext(hglrc);
            ReleaseDC(hwnd, hdc);
            DestroyWindow(hwnd);
            return None;
        }

        log::debug!("GPU: WGL 上下文已创建喵");
        Some(WinGpuContext {
            hwnd: hwnd as usize,
            hdc,
            hglrc,
        })
    }
}

/// 把 BGRA 像素呈现到分层窗口喵(per-pixel alpha)喵
fn present_impl(hwnd: HWND, width: i32, height: i32, bgra: &[u8]) {
    use windows_sys::Win32::Graphics::Gdi::{
        AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, DIB_RGB_COLORS, GetDC,
        ReleaseDC, SelectObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA};
    use windows_sys::Win32::Foundation::SIZE;

    if width <= 0 || height <= 0 {
        return;
    }

    unsafe {
        let screen_dc = GetDC(null_mut());
        let mem_dc = CreateCompatibleDC(screen_dc);

        let mut bmi: BITMAPINFO = zeroed();
        bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..zeroed()
        };

        let mut bits: *mut std::ffi::c_void = null_mut();
        let hbitmap = CreateDIBSection(
            screen_dc,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            null_mut(),
            0,
        );

        if hbitmap.is_null() || bits.is_null() {
            log::error!("CreateDIBSection 失败喵");
            DeleteDC(mem_dc);
            ReleaseDC(null_mut(), screen_dc);
            return;
        }

        let size = (width * height * 4) as usize;
        let len = size.min(bgra.len());
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, len);

        let old = SelectObject(mem_dc, hbitmap);
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let src = POINT { x: 0, y: 0 };
        let sz = SIZE {
            cx: width,
            cy: height,
        };
        UpdateLayeredWindow(
            hwnd,
            null_mut(),
            null_mut(),
            &sz,
            mem_dc,
            &src,
            0,
            &blend,
            ULW_ALPHA,
        );

        SelectObject(mem_dc, old);
        DeleteObject(hbitmap);
        DeleteDC(mem_dc);
        ReleaseDC(null_mut(), screen_dc);
    }
}

// ---------------------------------------------------------------------------
// 托盘图标: exe 内嵌资源 → HICON
// ---------------------------------------------------------------------------

/// 单实例互斥体句柄喵(进程存活期间常驻,退出时由系统回收)喵
static SINGLE_INSTANCE_MUTEX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// TaskbarCreated 广播消息 id 喵(explorer 重启/启动时广播,懒注册)喵
static WM_TRAY_TASKBAR: OnceLock<u32> = OnceLock::new();

/// 托盘小图标的目标尺寸喵(跟随系统 DPI)喵
fn small_icon_size() -> (i32, i32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON, SM_CYSMICON};
    unsafe { (
        GetSystemMetrics(SM_CXSMICON).max(16),
        GetSystemMetrics(SM_CYSMICON).max(16),
    ) }
}

/// 从 exe 资源加载应用图标喵(winres 嵌入的 app-icon.ico,资源 id = 1)喵
#[allow(clippy::manual_dangling_ptr)] // MAKEINTRESOURCE(1),不是空悬指针喵
fn app_icon_hicon(width: i32, height: i32) -> HICON {
    use windows_sys::Win32::UI::WindowsAndMessaging::{LoadImageW, IMAGE_ICON, LR_DEFAULTCOLOR};

    unsafe {
        let hmod = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(null_mut());
        LoadImageW(hmod, 1usize as *const u16, IMAGE_ICON, width, height, LR_DEFAULTCOLOR) as HICON
    }
}

/// 往 NOTIFYICONDATAW 写提示文本喵(szTip 定长 128 wchar)喵
fn set_nid_tip(nid: &mut NOTIFYICONDATAW, tip: &str) {
    let mut wide: Vec<u16> = tip.encode_utf16().collect();
    wide.truncate(127);
    wide.push(0);
    nid.szTip[..wide.len()].copy_from_slice(&wide);
}

// ---------------------------------------------------------------------------
// 热键消息循环
// ---------------------------------------------------------------------------

/// 解析热键字符串为 (修饰键, 虚拟键码) 喵
fn parse_hotkey(modifiers: &str, key: &str) -> Option<(u32, u32)> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
    };

    let mut mods: u32 = 0;
    for part in modifiers.split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "alt" => mods |= MOD_ALT,
            "shift" => mods |= MOD_SHIFT,
            "win" | "super" | "meta" => mods |= MOD_WIN,
            "" => {}
            other => {
                log::warn!("未知修饰键: {other}");
                return None;
            }
        }
    }

    let lower = key.trim().to_ascii_lowercase();
    let vk: u32 = if lower.len() == 1 {
        let c = lower.chars().next()?;
        match c {
            'a'..='z' => c as u32 - 'a' as u32 + 0x41,
            '0'..='9' => c as u32,
            ' ' => 0x20,
            '`' => 0xC0,
            other => {
                log::warn!("未知热键字符: {other}");
                return None;
            }
        }
    } else if let Some(num) = lower.strip_prefix('f') {
        let n: u32 = num.parse().ok()?;
        if !(1..=24).contains(&n) {
            log::warn!("未知热键键名: {lower}");
            return None;
        }
        0x70 + n - 1
    } else {
        match lower.as_str() {
            "space" => 0x20,
            "enter" => 0x0D,
            "esc" | "escape" => 0x1B,
            "tab" => 0x09,
            "backspace" => 0x08,
            "delete" => 0x2E,
            "up" => 0x26,
            "down" => 0x28,
            "left" => 0x25,
            "right" => 0x27,
            "tilde" => 0xC0,
            other => {
                log::warn!("未知热键键名: {other}");
                return None;
            }
        }
    };
    Some((mods, vk))
}

/// 注册一路全局热键的通用实现喵(id 与转发消息参数化,呼出/扫描各占一路)喵
///
/// `foreground` 决定触发时要不要抢前台 —— 呼出热键要(岛要接收输入),
/// 扫描热键不要(后台默默干活,抢前台反而打断用户)喵。
#[allow(clippy::too_many_arguments)]
fn register_hotkey_impl(
    hotkey_id: i32,
    forward_msg: u32,
    foreground: bool,
    label: &str,
    modifiers: &str,
    key: &str,
    target: PlatformWindow,
    hwnd_holder: &Arc<Mutex<Option<usize>>>,
) -> bool {
    let (mods, vk) = match parse_hotkey(modifiers, key) {
        Some(v) => v,
        None => {
            log::warn!("无法解析热键: modifiers={modifiers:?} key={key:?}");
            return false;
        }
    };

    // 先取消旧的,再注册新的喵
    unregister_hotkey_impl(hwnd_holder, label);

    let target_hwnd = target.hwnd();
    let hwnd_holder = hwnd_holder.clone();
    let spawned = std::thread::Builder::new()
        .name(format!("meow-hotkey-{label}"))
        .spawn(move || {
            hotkey_message_loop(hotkey_id, forward_msg, foreground, mods, vk, target_hwnd, hwnd_holder);
        });

    match spawned {
        Ok(_) => {
            log::info!("{label}热键注册成功: {modifiers}+{key} 喵");
            true
        }
        Err(e) => {
            log::error!("{label}热键线程创建失败: {e}");
            false
        }
    }
}

/// 取消一路全局热键的通用实现喵(发 WM_CLOSE 让消息循环优雅退出)喵
fn unregister_hotkey_impl(hwnd_holder: &Mutex<Option<usize>>, label: &str) {
    let hwnd = hwnd_holder.lock().unwrap().take();
    if let Some(hwnd) = hwnd {
        log::debug!("取消{label}热键喵");
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                hwnd as HWND,
                windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                0,
                0,
            );
        }
    }
}

/// 隐藏窗口消息循环: 注册热键、触发时激活目标窗口并派发事件喵
#[allow(clippy::too_many_arguments)]
fn hotkey_message_loop(
    hotkey_id: i32,
    forward_msg: u32,
    foreground: bool,
    mods: u32,
    vk: u32,
    target_hwnd: usize,
    hwnd_holder: Arc<Mutex<Option<usize>>>,
) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, PostMessageW,
        RegisterClassW, SetForegroundWindow, TranslateMessage, MSG, WNDCLASSW, WM_HOTKEY,
        WS_OVERLAPPED,
    };

    let class_name: Vec<u16> = "MeowHotkeyWindow\0".encode_utf16().collect();
    let wc = WNDCLASSW {
        lpfnWndProc: Some(hotkey_wnd_proc),
        lpszClassName: class_name.as_ptr(),
        ..unsafe { zeroed() }
    };
    // 窗口类是进程级注册: 重新录制热键会再起一个线程,类已存在时属正常,忽略喵
    let rc = unsafe { RegisterClassW(&wc) };
    let already_exists = rc == 0
        && unsafe { windows_sys::Win32::Foundation::GetLastError() }
            == windows_sys::Win32::Foundation::ERROR_CLASS_ALREADY_EXISTS;
    if rc == 0 && !already_exists {
        log::error!("隐藏窗口类注册失败喵");
        return;
    }

    let hwnd: HWND = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
        )
    };
    if hwnd.is_null() {
        log::error!("隐藏窗口创建失败喵");
        return;
    }
    *hwnd_holder.lock().unwrap() = Some(hwnd as usize);

    // 旧热键线程可能还没释放同组合,轻微重试(上限 ~200ms),避免「改完不生效」喵
    let mut registered = false;
    for _ in 0..20 {
        if unsafe { RegisterHotKey(hwnd, hotkey_id, mods, vk) } != 0 {
            registered = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    if !registered {
        log::error!("RegisterHotKey 失败(可能被其他程序占用)喵");
        unsafe { DestroyWindow(hwnd) };
        *hwnd_holder.lock().unwrap() = None;
        return;
    }

    log::debug!("热键消息循环启动喵");

    let mut msg: MSG = unsafe { zeroed() };
    while unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } > 0 {
        if msg.message == WM_HOTKEY && msg.wParam as i32 == hotkey_id {
            log::debug!("收到全局热键喵!");
            // 呼出热键要抢前台: 热键按下即系统认定的「最后一次输入事件」,
            // 此刻本线程握有前台特权,错过这个窗口期主线程只能走前台锁自救喵。
            // 扫描热键是后台动作,不打断用户手头的事喵。
            if foreground {
                unsafe { SetForegroundWindow(target_hwnd as HWND) };
            }
            unsafe { PostMessageW(target_hwnd as HWND, forward_msg, 0, 0) };
        }
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    unsafe {
        UnregisterHotKey(hwnd, hotkey_id);
        DestroyWindow(hwnd);
    }
    log::debug!("热键消息循环退出喵");
}

/// 隐藏窗口的消息处理喵
unsafe extern "system" fn hotkey_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, DestroyWindow, PostQuitMessage, WM_CLOSE, WM_DESTROY,
    };
    match msg {
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

// ---------------------------------------------------------------------------
// 图标提取
// ---------------------------------------------------------------------------

/// 用 SHGetFileInfoW 提取文件图标为 BGRA 像素喵
fn extract_icon_pixels_impl(path: &str) -> Option<IconPixels> {
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC,
        GetDIBits, GetObjectW, ReleaseDC,
    };
    use windows_sys::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

    let path_wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();

    let mut sfi: SHFILEINFOW = unsafe { zeroed() };
    let result = unsafe {
        SHGetFileInfoW(
            path_wide.as_ptr(),
            0,
            &mut sfi,
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if result == 0 || sfi.hIcon.is_null() {
        log::debug!("无法提取图标: {path}");
        return None;
    }

    let mut icon_info: ICONINFO = unsafe { zeroed() };
    if unsafe { GetIconInfo(sfi.hIcon, &mut icon_info) } == 0 {
        unsafe { DestroyIcon(sfi.hIcon) };
        return None;
    }
    let hbm = icon_info.hbmColor;

    let mut bm: BITMAP = unsafe { zeroed() };
    if unsafe { GetObjectW(hbm as _, size_of::<BITMAP>() as i32, &mut bm as *mut _ as _) } == 0 {
        unsafe {
            DeleteObject(hbm);
            DeleteObject(icon_info.hbmMask);
            DestroyIcon(sfi.hIcon);
        }
        return None;
    }

    let width = bm.bmWidth as u32;
    let height = bm.bmHeight as u32;
    if width == 0 || height == 0 || width > 512 || height > 512 {
        unsafe {
            DeleteObject(hbm);
            DeleteObject(icon_info.hbmMask);
            DestroyIcon(sfi.hIcon);
        }
        return None;
    }

    let hdc = unsafe { GetDC(null_mut()) };
    if hdc.is_null() {
        unsafe {
            DeleteObject(hbm);
            DeleteObject(icon_info.hbmMask);
            DestroyIcon(sfi.hIcon);
        }
        return None;
    }

    let mut bmi: BITMAPINFO = unsafe { zeroed() };
    bmi.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..unsafe { zeroed() }
    };

    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let got = unsafe {
        GetDIBits(
            hdc,
            hbm,
            0,
            height,
            pixels.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        )
    };
    unsafe { ReleaseDC(null_mut(), hdc) };

    unsafe {
        DeleteObject(hbm);
        DeleteObject(icon_info.hbmMask);
        DestroyIcon(sfi.hIcon);
    }

    if got == 0 {
        log::debug!("GetDIBits 失败: {path}");
        return None;
    }

    Some(IconPixels {
        width,
        height,
        bgra: pixels,
    })
}

// ---------------------------------------------------------------------------
// 启动应用
// ---------------------------------------------------------------------------

/// 用 ShellExecuteW 打开任意文件/快捷方式/链接喵
/// 打开链接喵: 浏览器为空走系统默认;否则启动指定浏览器喵
///
/// 浏览器配置支持两种写法喵:
/// * 纯路径: `C:\...\chrome.exe`(URL 直接作为参数)喵
/// * 带占位符: `"C:\Program Files\...\firefox.exe" --new-window %1` 喵
fn open_url_impl(url: &str, browser: &str) -> bool {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let browser = browser.trim();
    if browser.is_empty() {
        return launch_impl(url);
    }

    // 拆 exe 与参数: 引号包裹的路径优先,否则按第一个空格切喵
    let (exe, mut params): (String, String) = if let Some(rest) = browser.strip_prefix('"')
        && let Some(end) = rest.find('"')
    {
        (rest[..end].to_string(), rest[end + 1..].trim().to_string())
    } else if let Some(space) = browser.find(' ') {
        (browser[..space].to_string(), browser[space + 1..].trim().to_string())
    } else {
        (browser.to_string(), String::new())
    };

    // %1 占位符 = URL 插入位;没有占位符就追加到参数尾部喵
    if params.contains("%1") {
        params = params.replace("%1", url);
    } else if params.is_empty() {
        params.push_str(url);
    } else {
        params.push(' ');
        params.push_str(url);
    }

    let exe_wide: Vec<u16> = exe.encode_utf16().chain(Some(0)).collect();
    let params_wide: Vec<u16> = params.encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            null_mut(),
            exe_wide.as_ptr(),
            params_wide.as_ptr(),
            null_mut(),
            SW_SHOWNORMAL,
        )
    };
    if (result as isize) > 32 {
        log::info!("已用自定义浏览器打开链接: {browser} ← {url} 喵");
        true
    } else {
        log::warn!("自定义浏览器打开失败(code={}),回退系统默认喵~", result as isize);
        launch_impl(url)
    }
}

/// 把文本放入系统剪贴板喵(CF_UNICODETEXT,UTF-16)喵
fn copy_to_clipboard_impl(text: &str) -> bool {
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    // CF 常量族在 Ole 模块里喵(历史包袱,别问喵)
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
    use windows_sys::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};

    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    let bytes = wide.len() * 2;

    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            log::warn!("剪贴板打开失败,复制取消喵~");
            return false;
        }
        EmptyClipboard();
        let handle: HANDLE = GlobalAlloc(GMEM_MOVEABLE, bytes);
        let ok = if handle.is_null() {
            log::warn!("剪贴板内存分配失败喵~");
            false
        } else {
            let dst = GlobalLock(handle) as *mut u16;
            if dst.is_null() {
                log::warn!("剪贴板内存锁定失败喵~");
                false
            } else {
                std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
                GlobalUnlock(handle);
                // 写入成功后剪贴板接管句柄,不许再 GlobalFree 喵
                !SetClipboardData(CF_UNICODETEXT as u32, handle).is_null()
            }
        };
        CloseClipboard();
        if ok {
            log::debug!("已复制 {} 个字符到剪贴板喵~", text.chars().count());
        } else {
            log::warn!("剪贴板写入失败喵~");
        }
        ok
    }
}

/// 执行内置系统命令喵(锁屏/睡眠/关机/重启)喵
fn execute_system_command_impl(command: SystemCommandKind) -> bool {
    match command {
        SystemCommandKind::Lock => {
            let ok = unsafe { windows_sys::Win32::System::Shutdown::LockWorkStation() != 0 };
            log::info!("执行系统命令: 锁屏,成功={ok} 喵");
            ok
        }
        SystemCommandKind::Sleep => {
            // SetSuspendState(休眠=false, 强制=false, 禁唤醒=false)喵
            let ok = unsafe { windows_sys::Win32::System::Power::SetSuspendState(0, 0, 0) != 0 };
            log::info!("执行系统命令: 睡眠,成功={ok} 喵");
            ok
        }
        SystemCommandKind::Shutdown | SystemCommandKind::Restart => {
            exit_windows_impl(matches!(command, SystemCommandKind::Restart))
        }
        // 自定义指令不走这里: 由业务层经 ShellRunner 异步执行喵
        SystemCommandKind::Custom => {
            log::warn!("Custom 指令应由 ShellRunner 执行,而非内置系统命令喵");
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Shell 指令执行喵(ShellRunner 特质的 Windows 实现)
// ---------------------------------------------------------------------------

/// 指令输出的展示上限喵(通知气泡塞不下太长的东西喵)
const SHELL_OUTPUT_LIMIT: usize = 400;

impl super::ShellRunner for Win32Platform {
    fn run_shell(&self, shell: super::ShellKind, command: &str) -> Result<String, String> {
        run_shell_command_impl(shell, command)
    }
}

/// 在指定 shell 中执行指令并捕获合并输出喵
///
/// 统一隐藏窗口(CREATE_NO_WINDOW),捕获 stdout+stderr,
/// 输出按控制台编码解码(UTF-8 优先,回落系统代码页)并截断到展示上限喵。
fn run_shell_command_impl(shell: super::ShellKind, command: &str) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    /// CREATE_NO_WINDOW: 后台执行不闪黑窗喵
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    if command.trim().is_empty() {
        return Err("指令为空喵".into());
    }

    let mut cmd = match shell {
        super::ShellKind::Powershell => {
            let mut c = Command::new("powershell.exe");
            c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", command]);
            c
        }
        super::ShellKind::Cmd => {
            let mut c = Command::new("cmd.exe");
            c.args(["/C", command]);
            c
        }
    };
    cmd.creation_flags(CREATE_NO_WINDOW);

    log::info!("执行{}指令: {command} 喵", shell.label());
    let output = cmd.output().map_err(|e| format!("指令启动失败: {e}"))?;

    // stdout + stderr 合并,按控制台编码解码喵
    // (中文系统的 ipconfig/ping 等经典工具输出 GBK,硬按 UTF-8 解会满屏替换符喵)
    let mut text = decode_console_output(&output.stdout);
    text.push_str(&decode_console_output(&output.stderr));
    let text = text.trim().to_string();

    if output.status.success() {
        Ok(truncate_output(&text))
    } else {
        let code = output.status.code().unwrap_or(-1);
        Err(if text.is_empty() {
            format!("指令退出码 {code}")
        } else {
            truncate_output(&text)
        })
    }
}

/// 按控制台编码把子进程输出字节解成文本喵(UTF-8 优先,回落系统 OEM 代码页)喵
///
/// 控制台程序的输出编码取决于程序本身与系统区域设置:
/// * 现代程序/PS 脚本可能直接输出 UTF-8 → 严格校验通过,原样直通喵
///   (中文系统的经典工具 ipconfig/ping 等输出 GBK(OEM 代码页 936) →
///   UTF-8 校验必然失败,回落用 `MultiByteToWideChar(CP_OEMCP)` 转成 UTF-16 再进 String喵)
///
/// 两条路都失败时才退回 UTF-8 宽松替换,尽力不产生乱码喵。
pub fn decode_console_output(bytes: &[u8]) -> String {
    use windows_sys::Win32::Globalization::{CP_OEMCP, MultiByteToWideChar};

    if bytes.is_empty() {
        return String::new();
    }
    // 1. 合法 UTF-8(含纯 ASCII)直接用喵
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }
    // 2. 按系统 OEM 代码页(中文系统 = 936/GBK)转宽字符喵
    unsafe {
        let len = MultiByteToWideChar(
            CP_OEMCP,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            null_mut(),
            0,
        );
        if len > 0 {
            let mut wide = vec![0u16; len as usize];
            let written = MultiByteToWideChar(
                CP_OEMCP,
                0,
                bytes.as_ptr(),
                bytes.len() as i32,
                wide.as_mut_ptr(),
                len,
            );
            if written > 0 {
                wide.truncate(written as usize);
                return String::from_utf16_lossy(&wide);
            }
        }
    }
    // 3. 兜底: UTF-8 宽松替换(非法字节变 U+FFFD,至少不 panic)喵
    String::from_utf8_lossy(bytes).into_owned()
}

/// 截断输出到展示上限喵(超长加省略号尾巴)喵
fn truncate_output(text: &str) -> String {
    if text.chars().count() <= SHELL_OUTPUT_LIMIT {
        return text.to_string();
    }
    let head: String = text.chars().take(SHELL_OUTPUT_LIMIT).collect();
    format!("{head}…")
}

// ---------------------------------------------------------------------------
// 系统通知喵(NotificationSink 特质的 Windows 实现)
// ---------------------------------------------------------------------------

impl super::NotificationSink for Win32Platform {
    fn show_notification(&self, title: &str, body: &str) {
        show_notification_impl(title, body);
    }
}

/// 托盘气泡通知喵(复用常驻托盘图标,零额外窗口零额外依赖)喵
///
/// 跨线程调用 Shell_NotifyIconW 是安全的(内部投递到托盘所有者线程)喵。
fn show_notification_impl(title: &str, body: &str) {
    /// 通知信息标志: 带信息图标喵
    const NIIF_INFO: u32 = 0x0000_0001;

    // 气泡文本上限: 标题 63 / 正文 255 字符(UTF-16 单元)喵
    let mut title_wide: Vec<u16> = title.encode_utf16().collect();
    title_wide.truncate(63);
    title_wide.push(0);
    let mut body_wide: Vec<u16> = body.encode_utf16().collect();
    body_wide.truncate(255);
    body_wide.push(0);

    let sent = TRAY_STATE.with(|s| {
        let tray = s.borrow();
        let Some(state) = tray.as_ref() else {
            return false;
        };
        unsafe {
            let mut nid: NOTIFYICONDATAW = zeroed();
            nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = state.hwnd as HWND;
            nid.uID = 1;
            nid.uFlags = NIF_INFO;
            nid.dwInfoFlags = NIIF_INFO;
            nid.szInfoTitle[..title_wide.len()].copy_from_slice(&title_wide);
            nid.szInfo[..body_wide.len()].copy_from_slice(&body_wide);
            Shell_NotifyIconW(NIM_MODIFY, &nid) != 0
        }
    });
    if sent {
        log::debug!("系统通知已弹出: {title} 喵");
    } else {
        // 托盘不在(极少见)时退化为日志,不打断业务喵
        log::info!("通知(托盘不可用,降级日志): {title} — {body} 喵");
    }
}

/// 关机/重启喵: 申请 SeShutdownPrivilege 特权后 ExitWindowsEx 喵
/// (不借道 shutdown.exe,避免控制台闪窗喵)喵
fn exit_windows_impl(restart: bool) -> bool {
    use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HANDLE, LUID};
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Shutdown::{
        EWX_POWEROFF, EWX_REBOOT, EWX_SHUTDOWN, ExitWindowsEx,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let flags = if restart { EWX_REBOOT } else { EWX_SHUTDOWN | EWX_POWEROFF };
    unsafe {
        let mut token: HANDLE = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token)
            == 0
        {
            log::warn!("关机/重启失败: 打开进程令牌不成功喵~");
            return false;
        }
        let privilege: Vec<u16> = "SeShutdownPrivilege\0".encode_utf16().collect();
        let mut luid: LUID = zeroed();
        if LookupPrivilegeValueW(std::ptr::null(), privilege.as_ptr(), &mut luid) == 0 {
            log::warn!("关机/重启失败: 查询关机特权不成功喵~");
            return false;
        }
        let tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        AdjustTokenPrivileges(token, 0, &tp, 0, null_mut(), null_mut());
        // 未持有特权时 AdjustTokenPrivileges 返回非 0 但 ERROR_NOT_ALL_ASSIGNED,
        // 不视为失败,交给 ExitWindowsEx 最终裁决喵
        if windows_sys::Win32::Foundation::GetLastError() != ERROR_SUCCESS
            && windows_sys::Win32::Foundation::GetLastError()
                != windows_sys::Win32::Foundation::ERROR_NOT_ALL_ASSIGNED
        {
            log::debug!("关机特权申请未完全成功,继续尝试执行喵~");
        }
        let ok = ExitWindowsEx(flags, 0) != 0;
        if ok {
            log::info!("执行系统命令: {},成功喵~", if restart { "重启" } else { "关机" });
        } else {
            log::warn!(
                "ExitWindowsEx 失败(rc={})喵~",
                windows_sys::Win32::Foundation::GetLastError()
            );
        }
        ok
    }
}

/// 启动应用喵(走 shell 关联;启动前先把环境变量同步到最新)喵
fn launch_impl(path: &str) -> bool {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    // 子进程继承的是**本进程**的环境块,而本进程是常驻的:
    // 开跑之前先对齐一次注册表,免得用户刚改的变量要等重启才生效喵。
    refresh_process_environment();

    let path_wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            null_mut(), // "open" 操作喵
            path_wide.as_ptr(),
            null_mut(),
            null_mut(),
            SW_SHOWNORMAL,
        )
    };
    let code = result as isize;
    if code <= 32 {
        log::error!("启动应用失败: {path} (code={code})");
        false
    } else {
        log::info!("已启动应用: {path} 喵");
        true
    }
}

/// 以管理员身份启动应用喵
///
/// Windows 上「管理员权限」就是 UAC 提权,所以走 `runas` 动词,
/// 由系统弹窗征求用户同意;未来 Linux 平台对应实现为 `sudo` 喵。
/// 提权失败(典型: 用户在 UAC 弹窗上点了「否」)只记 warn,不当故障处理喵。
fn launch_elevated_impl(path: &str) -> bool {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    /// ShellExecuteW 返回值 <=32 一律表示失败,5 是「用户拒绝了提权」喵
    const ERROR_ACCESS_DENIED: isize = 5;

    refresh_process_environment();

    let path_wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let verb: Vec<u16> = "runas".encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            verb.as_ptr(),
            path_wide.as_ptr(),
            null_mut(),
            null_mut(),
            SW_SHOWNORMAL,
        )
    };
    let code = result as isize;
    if code <= 32 {
        if code == ERROR_ACCESS_DENIED {
            log::warn!("提权启动已取消(用户拒绝了 UAC): {path} 喵");
        } else {
            log::error!("提权启动失败: {path} (code={code})");
        }
        false
    } else {
        log::info!("已以管理员身份启动: {path} 喵");
        true
    }
}

/// 带参数启动喵(explorer /select 打开所在位置等场景)喵
fn launch_args_impl(path: &str, args: &str) -> bool {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    refresh_process_environment();

    let path_wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let args_wide: Vec<u16> = args.encode_utf16().chain(Some(0)).collect();
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            null_mut(),
            path_wide.as_ptr(),
            args_wide.as_ptr(),
            null_mut(),
            SW_SHOWNORMAL,
        )
    };
    let code = result as isize;
    if code <= 32 {
        log::error!("带参启动失败: {path} {args} (code={code})");
        false
    } else {
        log::info!("已带参启动: {path} {args} 喵");
        true
    }
}

// ---------------------------------------------------------------------------
// 常量与类型引入喵
// ---------------------------------------------------------------------------

use windows_sys::Win32::UI::Shell::{NOTIFYICONDATAW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, Shell_NotifyIconW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
