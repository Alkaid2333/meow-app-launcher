//! Windows 平台实现喵~
//!
//! 通过 windows-sys 直接调用 Win32 API 喵。
//! 提供: 图标提取、应用启动、全局热键、窗口显示/隐藏喵。
//!
//! 注意: windows-sys 0.59 的 HWND 是 `*mut c_void` 类型别名,
//! 不是 tuple struct,所以直接用指针、跨线程存储时转 usize 喵。

use super::{IconPixels, PlatformCapabilities};
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::HWND;

/// 注册热键的 id 喵
const HOTKEY_ID: i32 = 0x4D4F; // "MO" 喵

/// Windows 平台句柄喵
pub struct Win32Platform {
    /// 热键隐藏窗口句柄(usize 形式,保证 Send)喵
    hotkey_hwnd: Arc<Mutex<Option<usize>>>,
}

impl Win32Platform {
    pub fn new() -> Self {
        Self {
            hotkey_hwnd: Arc::new(Mutex::new(None)),
        }
    }
}

impl PlatformCapabilities for Win32Platform {
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
        on_trigger: Box<dyn Fn() + Send + 'static>,
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

        let hwnd_holder = self.hotkey_hwnd.clone();
        let spawned = std::thread::Builder::new()
            .name("meow-hotkey".into())
            .spawn(move || {
                hotkey_message_loop(mods, vk, on_trigger, hwnd_holder);
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

    fn set_always_on_top(&self, on: bool) {
        // 窗口置顶由 GPUI 窗口创建时配置,运行时切换待第二阶段喵
        log::debug!("set_always_on_top({on}) 由窗口创建时配置喵");
    }

    fn platform_name(&self) -> &'static str {
        "windows"
    }

    fn window_hwnd(&self, window: &gpui::Window) -> Option<usize> {
        use raw_window_handle::HasWindowHandle;
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() else {
            return None;
        };
        Some(win32.hwnd.get() as usize)
    }

    fn set_visible_hwnd(&self, hwnd: usize, visible: bool, activate: bool) -> bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            HWND_TOPMOST, SW_HIDE, SW_SHOW, SW_SHOWNOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
            SWP_SHOWWINDOW, SetWindowPos, ShowWindow,
        };
        let hwnd = hwnd as HWND;
        unsafe {
            if visible {
                // 自绘窗口关键: 关掉 DWM 给这个无边框窗口画的那一圈非客户区边框 + 阴影
                // (1) DwmExtendFrameIntoClientArea(-1) 把 frame 扩到整个客户区,配合
                //     ACCENT_ENABLE_TRANSPARENTGRADIENT 把最外 1px 也一起透明化
                // (2) DwmSetWindowAttribute(NCRENDERING_DISABLED) 彻底关掉 DWM 非客户区渲染
                // 两者合体,窗口就完全等同于"自己就是形状",不会再有矩形载体边框喵
                remove_dwm_frame(hwnd);

                // 置顶显示喵(是否激活由调用方决定: 搜索框要抢焦点,选择窗不能抢)喵
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                );
                // activate=false 用 SW_SHOWNOACTIVATE,不抢搜索框的键盘焦点喵
                ShowWindow(hwnd, if activate { SW_SHOW } else { SW_SHOWNOACTIVATE });
            } else {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
        log::debug!("窗口可见性: {visible} (激活: {activate}) 喵");
        true
    }

    fn move_window_hwnd(&self, hwnd: usize, x: f32, y: f32) -> bool {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SWP_NOZORDER, SWP_NOSIZE, SetWindowPos};
        unsafe {
            SetWindowPos(
                hwnd as HWND,
                null_mut(),
                x as i32,
                y as i32,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER,
            );
        }
        log::debug!("窗口移动到 ({x}, {y}) 喵");
        true
    }
}

// ---------------------------------------------------------------------------
// DWM 异形窗口辅助: 关掉 DWM 给无边框窗口画的那一圈非客户区边框/阴影
// ---------------------------------------------------------------------------

/// 关掉 DWM 给无边框 PopUp 窗口画的非客户区边框 + 阴影,实现"窗口自己就是形状"喵
///
/// 背景: Windows 即便给了 `WS_POPUP`(dwstyle=0) + `WS_EX_NOREDIRECTIONBITMAP` + ACCENT 透明渐变,
/// DWM 仍然会围绕这个矩形窗口画一圈非客户区边框/阴影(就是肉眼看到的"明显的边框")。
/// 这里两步走把它彻底消掉:
///   1. `DwmExtendFrameIntoClientArea(MARGINS{-1,-1,-1,-1})`: 把 DWM frame 扩到整个客户区,
///      配合 ACCENT_ENABLE_TRANSPARENTGRADIENT,连最外 1px 都被透明化;
///   2. `DwmSetWindowAttribute(DWMWA_NCRENDERING_POLICY, DWMNCRP_DISABLED)`: 彻底关掉 DWM 的
///      非客户区渲染管线,不再有阴影/边框被画出来。
fn remove_dwm_frame(hwnd: HWND) {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMNCRP_DISABLED,
        DWMWA_NCRENDERING_POLICY,
    };
    use windows_sys::Win32::UI::Controls::MARGINS;

    unsafe {
        // 1) frame 扩到整个客户区(-1 = extend to entire client area)
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

        // 2) 关掉 DWM 非客户区渲染(同时去掉阴影 + 边框)
        let policy: i32 = DWMNCRP_DISABLED;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY as u32,
            &policy as *const i32 as *const std::ffi::c_void,
            std::mem::size_of::<i32>() as u32,
        );
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

/// 隐藏窗口消息循环: 注册热键、派发回调、响应注销喵
fn hotkey_message_loop(
    mods: u32,
    vk: u32,
    on_trigger: Box<dyn Fn() + Send + 'static>,
    hwnd_holder: Arc<Mutex<Option<usize>>>,
) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, DispatchMessageW, GetMessageW, RegisterClassW,
        TranslateMessage, MSG, WNDCLASSW, WM_HOTKEY, WS_OVERLAPPED,
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
            WS_OVERLAPPED as u32,
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
            on_trigger();
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
    wparam: usize,
    lparam: isize,
) -> isize {
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
        DeleteObject, GetDIBits, GetDC, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::UI::Shell::{
        SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW,
    };
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
