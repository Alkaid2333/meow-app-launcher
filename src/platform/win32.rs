//! Windows 平台实现喵~
//!
//! 通过 windows-sys 直接调用 Win32 API 喵。
//! 提供: 异形透明窗口、每像素透明呈现、消息泵、全局热键、图标提取、应用启动喵。
//!
//! 注意: windows-sys 0.59 的 HWND 是 `*mut c_void` 类型别名,
//! 不是 tuple struct,所以直接用指针、跨线程存储时转 usize 喵。

use super::{IconPixels, Key, Platform, PlatformWindow, WindowEvent, WindowHandler, WindowSpec};
use std::cell::RefCell;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::sync::{Arc, Mutex, Once};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

/// 注册热键的 id 喵
const HOTKEY_ID: i32 = 0x4D4F; // "MO" 喵
/// 热键线程 → 主窗口的自定义消息(WM_APP + 1)喵
const WM_MEOW_HOTKEY: u32 = 0x8000 + 1;
/// 动画定时器 id 喵
const TIMER_ID: usize = 1;
/// 窗口类名(UTF-16 编码,含 null 终止)喵
const CLASS_NAME: [u16; 19] = [
    0x4D, 0x65, 0x6F, 0x77, // "Meow"
    0x4C, 0x61, 0x75, 0x6E, 0x63, 0x68, 0x65, 0x72, // "Launcher"
    0x57, 0x69, 0x6E, 0x64, 0x6F, 0x77, // "Window"
    0x00, // null 终止喵
];

/// 全局窗口类注册锁喵(整个进程只注册一次)喵
static CLASS_ONCE: Once = Once::new();

// 当前消息循环的事件处理器喵(单线程,用 thread_local 传给 WndProc)喵
thread_local! {
    static HANDLER: RefCell<Option<*mut dyn WindowHandler>> = const { RefCell::new(None) };
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
                log::debug!("窗口创建成功: hwnd={} 喵", hwnd as usize);
                Some(PlatformWindow::from_hwnd(hwnd as usize))
            }
        }
    }

    fn destroy_window(&self, window: &PlatformWindow) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow(window.hwnd() as HWND);
        }
    }

    fn show_window(&self, window: &PlatformWindow, show: bool) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOW};
        unsafe {
            ShowWindow(window.hwnd() as HWND, if show { SW_SHOW } else { SW_HIDE });
        }
    }

    fn resize_window(&self, window: &PlatformWindow, width: i32, height: i32) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOMOVE,
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

    fn kill_timer(&self, window: &PlatformWindow) {
        use windows_sys::Win32::UI::WindowsAndMessaging::KillTimer;
        unsafe {
            KillTimer(window.hwnd() as HWND, TIMER_ID);
        }
    }

    fn run_message_loop(&self, _window: &PlatformWindow, handler: &mut dyn WindowHandler) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            DispatchMessageW, GetMessageW, TranslateMessage, MSG,
        };

        // 安全性: 消息循环阻塞运行直至退出,handler 在整个循环期间始终有效;
        // 且单线程运行,无数据竞争。此处把引用生命周期延长为 'static 仅供
        // WndProc 回调使用,循环结束立即清空喵。
        let handler: &'static mut dyn WindowHandler =
            unsafe { std::mem::transmute(handler) };

        // 把 handler 指针存到 thread_local,供 WndProc 回调喵
        HANDLER.with(|slot| {
            *slot.borrow_mut() = Some(handler as *mut dyn WindowHandler);
        });

        let mut msg: MSG = unsafe { zeroed() };
        // 消息泵: GetMessageW 返回 0 表示收到 WM_QUIT,退出喵
        while unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } > 0 {
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // 清空 handler 指针喵
        HANDLER.with(|slot| {
            *slot.borrow_mut() = None;
        });
        log::debug!("消息循环退出喵");
    }

    fn scale_factor(&self) -> f32 {
        unsafe { windows_sys::Win32::UI::HiDpi::GetDpiForSystem() as f32 / 96.0 }
    }

    fn screen_size(&self) -> (i32, i32) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
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
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, PostQuitMessage, WM_ACTIVATE, WM_CHAR, WM_CLOSE, WM_DESTROY, WM_KEYDOWN,
        WM_LBUTTONDOWN, WM_PAINT, WM_TIMER,
    };
    use windows_sys::Win32::Graphics::Gdi::ValidateRect;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_BACK;

    match msg {
        // 热键触发(热键线程 PostMessage 过来)喵
        WM_MEOW_HOTKEY => {
            with_handler(|h| h.on_event(WindowEvent::Hotkey));
            0
        }
        // 导航键按下喵
        WM_KEYDOWN => {
            let vk = wparam as u16;
            if let Some(key) = map_key(vk) {
                with_handler(|h| h.on_event(WindowEvent::KeyDown(key)));
            }
            // 特殊: 退格键不会产生 WM_CHAR,单独处理喵
            if vk == VK_BACK {
                with_handler(|h| h.on_event(WindowEvent::KeyDown(Key::Backspace)));
            }
            0
        }
        // 字符输入喵
        WM_CHAR => {
            let code = wparam as u32;
            // 过滤控制字符,只保留可打印字符喵
            if code >= 0x20 && code != 0x7f
                && let Some(ch) = char::from_u32(code) {
                    with_handler(|h| h.on_event(WindowEvent::Char(ch)));
                }
            0
        }
        // 鼠标左键按下喵
        WM_LBUTTONDOWN => {
            let (x, y) = unpack_lparam(lparam);
            with_handler(|h| h.on_event(WindowEvent::MouseDown(x, y)));
            0
        }
        // 失焦(前台切走)喵
        WM_ACTIVATE => {
            let active = (wparam as u32 & 0xFFFF) != 0; // WA_INACTIVE = 0 喵
            if !active {
                with_handler(|h| h.on_event(WindowEvent::LostFocus));
            }
            0
        }
        // 动画帧定时器喵
        WM_TIMER => {
            if wparam == TIMER_ID {
                with_handler(|h| h.on_event(WindowEvent::Timer));
            }
            0
        }
        // 忽略系统重绘请求(我们主动渲染喵)
        WM_PAINT => {
            unsafe { ValidateRect(hwnd, null_mut()) };
            0
        }
        // 请求关闭喵
        WM_CLOSE => {
            with_handler(|h| h.on_event(WindowEvent::Close));
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// 取当前 handler 并执行回调喵
fn with_handler<R>(f: impl FnOnce(&mut dyn WindowHandler) -> R) -> Option<R> {
    HANDLER.with(|slot| {
        let ptr = *slot.borrow();
        // 安全性: 消息循环单线程运行,handler 在循环存续期间有效喵
        ptr.map(|p| f(unsafe { &mut *p }))
    })
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

/// 从 lparam 提取客户区坐标(物理像素)喵
///
/// 低 16 位为 x,高 16 位为 y(有符号)喵。
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
        // 背景刷为空,layered 窗口不依赖 WM_ERASEBKGND 喵
        hbrBackground: null_mut(),
        ..unsafe { zeroed() }
    };
    if unsafe { RegisterClassW(&wc) } == 0 {
        log::error!("主窗口类注册失败喵");
    } else {
        log::debug!("主窗口类注册成功喵");
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
    use windows_sys::Win32::Foundation::{POINT, SIZE};

    if width <= 0 || height <= 0 {
        return;
    }

    unsafe {
        // 屏幕 DC + 兼容内存 DC 喵
        let screen_dc = GetDC(null_mut());
        let mem_dc = CreateCompatibleDC(screen_dc);

        // 32bpp、负高度(自顶向下)位图信息喵
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

        // 创建 DIB section,拿到可直接写入的像素指针喵
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

        // 拷贝 BGRA 像素到 DIB 喵
        let size = (width * height * 4) as usize;
        let len = size.min(bgra.len());
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, len);

        // 选入内存 DC 后呈现喵
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

        // 清理喵
        SelectObject(mem_dc, old);
        DeleteObject(hbitmap);
        DeleteDC(mem_dc);
        ReleaseDC(null_mut(), screen_dc);
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
        // 单个字符: 字母/数字/符号喵
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
        // F1-F24 功能键喵
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

    // 注册隐藏窗口类喵
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

    // 创建隐藏窗口喵
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
    // 把句柄共享出去,方便外部注销喵
    *hwnd_holder.lock().unwrap() = Some(hwnd as usize);

    // 注册全局热键喵
    if unsafe { RegisterHotKey(hwnd, HOTKEY_ID, mods, vk) } == 0 {
        log::error!("RegisterHotKey 失败(可能被其他程序占用)喵");
        unsafe { DestroyWindow(hwnd) };
        return;
    }

    log::debug!("热键消息循环启动喵");

    // 消息泵喵
    let mut msg: MSG = unsafe { zeroed() };
    while unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) } > 0 {
        if msg.message == WM_HOTKEY && msg.wParam as i32 == HOTKEY_ID {
            log::debug!("收到全局热键喵!");
            // 先激活主窗口(用户主动按热键,允许抢前台)再派发事件喵
            unsafe { SetForegroundWindow(target_hwnd as HWND) };
            unsafe { PostMessageW(target_hwnd as HWND, WM_MEOW_HOTKEY, 0, 0) };
        }
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    // 收到 WM_QUIT 后清理喵
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
            // 关闭窗口 → 触发 WM_DESTROY → 退出消息循环喵
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

    // 拿到图标对应的位图喵
    let mut icon_info: ICONINFO = unsafe { zeroed() };
    if unsafe { GetIconInfo(sfi.hIcon, &mut icon_info) } == 0 {
        unsafe { DestroyIcon(sfi.hIcon) };
        return None;
    }
    let hbm = icon_info.hbmColor;

    // 读取位图尺寸喵
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

    // 用 GetDIBits 读出 BGRA 像素喵
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
        // 负高度 = 自顶向下,省去翻转喵
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
    // ShellExecute 返回值 > 32 表示成功喵
    let code = result as isize;
    if code <= 32 {
        log::error!("启动应用失败: {path} (code={code})");
        false
    } else {
        log::info!("已启动应用: {path} 喵");
        true
    }
}

// 引入必要的 Win32 常量喵
use windows_sys::Win32::UI::WindowsAndMessaging::{
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
