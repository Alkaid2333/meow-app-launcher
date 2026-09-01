//! Windows 平台实现喵~
//!
//! 通过 windows-sys 直接调用 Win32 API 喵。
//! 提供: 异形透明窗口(多窗口)、每像素透明呈现、全局消息泵、系统托盘、
//! 全局热键、图标提取、应用启动喵。
//!
//! 注意: windows-sys 0.59 的 HWND 是 `*mut c_void` 类型别名,
//! 不是 tuple struct,所以直接用指针、跨线程存储时转 usize 喵。

use super::{
    IconPixels, Key, Platform, PlatformWindow, TrayEvent, TrayHandle, TrayHandler, TrayMenuItem,
    WindowEvent, WindowHandler, WindowSpec,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::sync::{Arc, Mutex, Once};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::HICON;

/// 注册热键的 id 喵
const HOTKEY_ID: i32 = 0x4D4F; // "MO" 喵
/// 热键线程 → 主窗口的自定义消息(WM_APP + 1)喵
const WM_MEOW_HOTKEY: u32 = 0x8000 + 1;
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
    /// 热键隐藏窗口句柄(usize 形式,保证 Send)喵
    hotkey_hwnd: Arc<Mutex<Option<usize>>>,
}

impl Win32Platform {
    pub fn new() -> Self {
        // 声明进程 DPI 感知,窗口尺寸/鼠标坐标都按物理像素处理喵
        enable_dpi_awareness();
        Self {
            hotkey_hwnd: Arc::new(Mutex::new(None)),
        }
    }
}

impl Platform for Win32Platform {
    fn extract_icon_pixels(&self, path: &str) -> Option<IconPixels> {
        extract_icon_pixels_impl(path)
    }

    fn launch(&self, path: &str) -> bool {
        launch_impl(path)
    }

    fn register_global_hotkey(
        &self,
        modifiers: &str,
        key: &str,
        target: PlatformWindow,
    ) -> bool {
        let (mods, vk) = match parse_hotkey(modifiers, key) {
            Some(v) => v,
            None => {
                log::warn!("无法解析热键: modifiers={modifiers:?} key={key:?}");
                return false;
            }
        };

        // 先取消旧的,再注册新的喵
        self.unregister_global_hotkey();

        let target_hwnd = target.hwnd();
        let hwnd_holder = self.hotkey_hwnd.clone();
        let spawned = std::thread::Builder::new()
            .name("meow-hotkey".into())
            .spawn(move || {
                hotkey_message_loop(mods, vk, target_hwnd, hwnd_holder);
            });

        match spawned {
            Ok(_) => {
                log::info!("全局热键注册成功: {modifiers}+{key} 喵");
                true
            }
            Err(e) => {
                log::error!("热键线程创建失败: {e}");
                false
            }
        }
    }

    fn unregister_global_hotkey(&self) {
        let hwnd = self.hotkey_hwnd.lock().unwrap().take();
        if let Some(hwnd) = hwnd {
            log::debug!("取消全局热键喵");
            unsafe {
                // 发 WM_CLOSE 让消息循环优雅退出喵
                windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                    hwnd as HWND,
                    windows_sys::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                    0,
                    0,
                );
            }
        }
    }

    fn create_window(&self, spec: &WindowSpec) -> Option<PlatformWindow> {
        // 确保窗口类已注册喵
        CLASS_ONCE.call_once(|| {
            register_class();
        });

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

    fn set_window_handler(&self, window: &PlatformWindow, handler: Box<dyn WindowHandler>) {
        HANDLERS.with(|handlers| {
            handlers.borrow_mut().insert(window.hwnd(), handler);
        });
    }

    fn destroy_window(&self, window: &PlatformWindow) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(window.hwnd() as HWND);
        }
    }

    fn show_window(&self, window: &PlatformWindow, show: bool) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOW};
        unsafe {
            // SW_SHOWNA: 显示但不抢焦点,避免分层窗闪一下又失焦喵
            ShowWindow(window.hwnd() as HWND, if show { SW_SHOW } else { SW_HIDE });
        }
    }

    fn focus_window(&self, window: &PlatformWindow) {
        use windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
        unsafe {
            SetForegroundWindow(window.hwnd() as HWND);
        }
    }

    fn enable_file_drop(&self, window: &PlatformWindow) {
        use windows_sys::Win32::UI::Shell::DragAcceptFiles;
        unsafe {
            DragAcceptFiles(window.hwnd() as HWND, 1);
        }
    }

    fn resize_window(&self, window: &PlatformWindow, width: i32, height: i32) {
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

    fn move_window(&self, window: &PlatformWindow, x: i32, y: i32) {
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

    fn present(&self, window: &PlatformWindow, width: i32, height: i32, bgra: &[u8]) {
        present_impl(window.hwnd() as HWND, width, height, bgra);
    }

    fn set_timer(&self, window: &PlatformWindow, interval_ms: u32) {
        use windows_sys::Win32::UI::WindowsAndMessaging::SetTimer;
        unsafe {
            SetTimer(window.hwnd() as HWND, TIMER_ID, interval_ms, None);
        }
    }

    fn create_tray(&self) -> Option<TrayHandle> {
        // 确保托盘窗口类已注册喵
        TRAY_CLASS_ONCE.call_once(|| {
            register_tray_class();
        });

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

            // 添加托盘图标喵
            let mut nid: NOTIFYICONDATAW = zeroed();
            nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            nid.uFlags = NIF_MESSAGE;
            nid.uCallbackMessage = WM_TRAY_MSG;
            if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
                log::error!("添加托盘图标失败喵");
                windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
                return None;
            }

            // 存储状态喵
            TRAY_STATE.with(|s| {
                *s.borrow_mut() = Some(TrayState {
                    hwnd: hwnd as usize,
                    icon: null_mut(),
                    menu: Vec::new(),
                });
            });

            log::info!("系统托盘创建成功喵");
            Some(TrayHandle)
        }
    }

    fn set_tray_handler(&self, _tray: &TrayHandle, handler: Box<dyn TrayHandler>) {
        TRAY_HANDLER.with(|h| *h.borrow_mut() = Some(handler));
    }

    fn destroy_tray(&self, _tray: &TrayHandle) {
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

    fn set_tray_icon(&self, _tray: &TrayHandle, width: u32, height: u32, bgra: &[u8]) {
        let hicon = pixels_to_hicon(width, height, bgra);
        if hicon.is_null() {
            log::warn!("托盘图标创建失败喵");
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

    fn set_tray_tip(&self, _tray: &TrayHandle, tip: &str) {
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

    fn set_tray_menu(&self, _tray: &TrayHandle, items: Vec<TrayMenuItem>) {
        TRAY_STATE.with(|s| {
            if let Some(state) = s.borrow_mut().as_mut() {
                state.menu = items;
            }
        });
    }

    fn run(&self) {
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

    fn quit(&self) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
        }
    }

    fn scale_factor(&self) -> f32 {
        unsafe { windows_sys::Win32::UI::HiDpi::GetDpiForSystem() as f32 / 96.0 }
    }

    fn screen_size(&self) -> (i32, i32) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
    }

    fn display_refresh_rate(&self) -> u32 {
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

    fn platform_name(&self) -> &'static str {
        "windows"
    }
}

// ---------------------------------------------------------------------------
// 窗口过程: 把 WM_* 消息翻译成平台无关事件喵
// ---------------------------------------------------------------------------

/// 主窗口消息处理喵
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    use windows_sys::Win32::Graphics::Gdi::ValidateRect;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_BACK;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, WM_ACTIVATE, WM_CHAR, WM_CLOSE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP,
        WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCDESTROY, WM_PAINT, WM_SYSKEYDOWN, WM_TIMER,
    };

    match msg {
        // 热键触发(热键线程 PostMessage 过来)喵
        WM_MEOW_HOTKEY => {
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::Hotkey));
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
        // 鼠标左键按下喵
        WM_LBUTTONDOWN => {
            let (x, y) = unpack_lparam(lparam);
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::MouseDown(x, y)));
            0
        }
        WM_MOUSEMOVE => {
            let (x, y) = unpack_lparam(lparam);
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::MouseMove(x, y)));
            0
        }
        WM_LBUTTONUP => {
            with_window_handler(hwnd, |h| h.on_event(WindowEvent::MouseUp));
            0
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
        // 失焦(前台切走)喵
        WM_ACTIVATE => {
            let active = (wparam as u32 & 0xFFFF) != 0; // WA_INACTIVE = 0 喵
            if !active {
                with_window_handler(hwnd, |h| h.on_event(WindowEvent::LostFocus));
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
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow,
        TrackPopupMenu, MF_ENABLED, MF_GRAYED, MF_STRING, TPM_BOTTOMALIGN, TPM_RIGHTALIGN,
        TPM_RIGHTBUTTON, WM_NULL,
    };

    let items = TRAY_STATE.with(|s| {
        s.borrow().as_ref().map(|state| state.menu.clone()).unwrap_or_default()
    });
    if items.is_empty() {
        return;
    }

    unsafe {
        let hmenu = CreatePopupMenu();
        if hmenu.is_null() {
            return;
        }

        // 逐个追加菜单项喵(菜单项 id = 索引 + 1,0 保留)喵
        for (i, item) in items.iter().enumerate() {
            let label: Vec<u16> = item.label.encode_utf16().chain(Some(0)).collect();
            let flags = MF_STRING | if item.enabled { MF_ENABLED } else { MF_GRAYED };
            AppendMenuW(hmenu, flags, i + 1, label.as_ptr());
        }

        // 在光标处弹出菜单喵
        let mut pt: POINT = zeroed();
        GetCursorPos(&mut pt);
        // 设置前台窗口,确保菜单点击外部能正确关闭喵
        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            hmenu,
            TPM_RIGHTBUTTON | TPM_RIGHTALIGN | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            0,
            hwnd,
            null_mut(),
        );
        // 托盘菜单点完后必须再丢一条空消息,否则下次点不开喵
        windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(hmenu);
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
        let count = DragQueryFileW(hdrop, 0xFFFF, null_mut(), 0);
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
// 托盘图标: BGRA 像素 → HICON
// ---------------------------------------------------------------------------

/// 把 BGRA 像素转成 HICON 喵(带 alpha)喵
fn pixels_to_hicon(width: u32, height: u32, bgra: &[u8]) -> HICON {
    use windows_sys::Win32::Graphics::Gdi::{
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CreateDIBSection, DeleteObject, DIB_RGB_COLORS,
        GetDC, ReleaseDC,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, ICONINFO};

    if width == 0 || height == 0 {
        return null_mut();
    }

    unsafe {
        let screen_dc = GetDC(null_mut());

        // 32bpp 彩色 DIB(含 alpha,自顶向下)喵
        let mut color_bmi: BITMAPINFO = zeroed();
        color_bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..zeroed()
        };
        let mut color_bits: *mut std::ffi::c_void = null_mut();
        let hbm_color = CreateDIBSection(
            screen_dc,
            &color_bmi,
            DIB_RGB_COLORS,
            &mut color_bits,
            null_mut(),
            0,
        );
        if !color_bits.is_null() {
            let len = (width * height * 4) as usize;
            std::ptr::copy_nonoverlapping(bgra.as_ptr(), color_bits as *mut u8, len.min(bgra.len()));
        }

        // 1bpp AND mask(全 0 = 不透明)喵
        let mask_stride = (width.div_ceil(16) * 2) as usize;
        let and_mask = vec![0u8; mask_stride * height as usize];
        let mut mask_bmi: BITMAPINFO = zeroed();
        mask_bmi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 1,
            biCompression: BI_RGB,
            ..zeroed()
        };
        let mut mask_bits: *mut std::ffi::c_void = null_mut();
        let hbm_mask = CreateDIBSection(
            screen_dc,
            &mask_bmi,
            DIB_RGB_COLORS,
            &mut mask_bits,
            null_mut(),
            0,
        );
        if !mask_bits.is_null() {
            std::ptr::copy_nonoverlapping(and_mask.as_ptr(), mask_bits as *mut u8, and_mask.len());
        }

        let icon_info = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hbm_mask,
            hbmColor: hbm_color,
        };
        let hicon = CreateIconIndirect(&icon_info);

        DeleteObject(hbm_color);
        DeleteObject(hbm_mask);
        ReleaseDC(null_mut(), screen_dc);

        hicon
    }
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

/// 隐藏窗口消息循环: 注册热键、触发时激活目标窗口并派发事件喵
fn hotkey_message_loop(
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
    if unsafe { RegisterClassW(&wc) } == 0 {
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

    if unsafe { RegisterHotKey(hwnd, HOTKEY_ID, mods, vk) } == 0 {
        log::error!("RegisterHotKey 失败(可能被其他程序占用)喵");
        unsafe { DestroyWindow(hwnd) };
        return;
    }

    log::debug!("热键消息循环启动喵");

    let mut msg: MSG = unsafe { zeroed() };
    while unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } > 0 {
        if msg.message == WM_HOTKEY && msg.wParam as i32 == HOTKEY_ID {
            log::debug!("收到全局热键喵!");
            unsafe { SetForegroundWindow(target_hwnd as HWND) };
            unsafe { PostMessageW(target_hwnd as HWND, WM_MEOW_HOTKEY, 0, 0) };
        }
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    unsafe {
        UnregisterHotKey(hwnd, HOTKEY_ID);
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
fn launch_impl(path: &str) -> bool {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

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

// ---------------------------------------------------------------------------
// 常量与类型引入喵
// ---------------------------------------------------------------------------

use windows_sys::Win32::UI::Shell::{NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, Shell_NotifyIconW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
