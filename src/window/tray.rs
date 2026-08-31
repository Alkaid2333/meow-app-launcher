//! 系统托盘喵~
//!
//! 托盘图标(主题自适应) + 右键菜单(显示/隐藏、设置、重启、退出)喵。
//! 单击托盘图标 = 显示/隐藏启动器切换喵。
//!
//! 托盘不直接操作窗口,而是把意图投递到应用命令队列,由启动器(应用控制器)统一执行喵。

use crate::app::config::ThemeMode;
use crate::app::{Command, SharedState};
use crate::platform::{Platform, TrayEvent, TrayHandle, TrayHandler, TrayMenuItem};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

/// 托盘图标尺寸喵
const ICON_SIZE: u32 = 32;

/// 系统托盘喵
pub struct Tray {
    /// 共享应用状态喵(判断启动器可见性、主题)喵
    state: SharedState,
    /// 应用命令队列喵
    commands: Rc<RefCell<VecDeque<Command>>>,
    /// 平台句柄喵
    platform: Arc<dyn Platform>,
    /// 托盘句柄喵
    tray: TrayHandle,
    /// 当前是否深色主题喵(用于图标自适应)喵
    dark: bool,
}

impl Tray {
    /// 装配系统托盘喵: 创建托盘、设置图标/提示/菜单、绑定 handler 喵
    ///
    /// 返回托盘句柄,供调用方在退出时移除喵。
    pub fn spawn(
        platform: Arc<dyn Platform>,
        state: SharedState,
        commands: Rc<RefCell<VecDeque<Command>>>,
    ) -> TrayHandle {
        let tray = platform.create_tray().unwrap_or_else(|| {
            log::error!("托盘创建失败喵~");
            std::process::exit(1);
        });

        let dark = state.borrow().config.theme.mode == ThemeMode::Dark;
        let mut handler = Tray {
            state: state.clone(),
            commands,
            platform: platform.clone(),
            tray,
            dark,
        };

        // 初始图标 + 提示 + 菜单喵
        handler.apply_icon();
        platform.set_tray_tip(&tray, "喵喵应用启动器");
        handler.refresh_menu();

        platform.set_tray_handler(&tray, Box::new(handler));

        tray
    }

    /// 判断当前是否深色主题喵
    fn is_dark(&self) -> bool {
        self.state.borrow().config.theme.mode == ThemeMode::Dark
    }

    /// 按当前主题更新托盘图标喵
    fn apply_icon(&mut self) {
        let (w, h, pixels) = tray_icon_pixels(self.dark);
        self.platform.set_tray_icon(&self.tray, w, h, &pixels);
    }

    /// 刷新托盘菜单(「显示/隐藏」文本随启动器可见性动态变化)喵
    fn refresh_menu(&mut self) {
        let visible = self.state.borrow().launcher_visible;
        let items = vec![
            TrayMenuItem {
                label: if visible { "隐藏".into() } else { "显示".into() },
                enabled: true,
            },
            TrayMenuItem {
                label: "设置".into(),
                enabled: true,
            },
            TrayMenuItem {
                label: "重启".into(),
                enabled: true,
            },
            TrayMenuItem {
                label: "退出".into(),
                enabled: true,
            },
        ];
        self.platform.set_tray_menu(&self.tray, items);
    }
}

impl TrayHandler for Tray {
    fn on_event(&mut self, event: TrayEvent) {
        // 主题可能已变化,同步图标喵
        let dark = self.is_dark();
        if dark != self.dark {
            self.dark = dark;
            self.apply_icon();
        }

        match event {
            TrayEvent::LeftClick => {
                self.commands.borrow_mut().push_back(Command::ToggleLauncher);
            }
            TrayEvent::Menu(i) => {
                let cmd = match i {
                    0 => Some(Command::ToggleLauncher),
                    1 => Some(Command::OpenSettings),
                    2 => Some(Command::Restart),
                    3 => Some(Command::Quit),
                    _ => None,
                };
                if let Some(cmd) = cmd {
                    self.commands.borrow_mut().push_back(cmd);
                }
            }
        }

        // 刷新菜单文本(可见性可能已变化)喵
        self.refresh_menu();
    }
}

/// 生成托盘图标像素喵(一个圆点,深色主题用浅色,浅色主题用深色)喵
fn tray_icon_pixels(dark: bool) -> (u32, u32, Vec<u8>) {
    let size = ICON_SIZE;
    let mut bgra = vec![0u8; (size * size * 4) as usize];
    // 深色主题 → 白色圆点;浅色主题 → 深灰圆点喵
    let (r, g, b) = if dark {
        (0xE8, 0xE8, 0xE8)
    } else {
        (0x3A, 0x3A, 0x3E)
    };

    let center = size as f32 / 2.0;
    let radius = size as f32 / 2.0 - 5.0;
    let radius_sq = radius * radius;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            if dx * dx + dy * dy <= radius_sq {
                let idx = ((y * size + x) * 4) as usize;
                bgra[idx] = b;
                bgra[idx + 1] = g;
                bgra[idx + 2] = r;
                bgra[idx + 3] = 255;
            }
        }
    }
    (size, size, bgra)
}
