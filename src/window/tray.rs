//! 系统托盘喵~
//!
//! 托盘图标(exe 内嵌应用图标) + 右键菜单(显示/隐藏、设置、重启、退出)喵。
//! 单击托盘图标 = 显示/隐藏启动器切换喵。
//!
//! 托盘不直接操作窗口,而是把意图投递到应用命令队列,由启动器(应用控制器)统一执行喵。

use crate::app::{Command, SharedState};
use crate::platform::{TrayEvent, TrayHandle, TrayHandler, TrayMenuItem, Win32Platform};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

/// 系统托盘喵
pub struct Tray {
    /// 共享应用状态喵(判断启动器可见性)喵
    state: SharedState,
    /// 应用命令队列喵
    commands: Rc<RefCell<VecDeque<Command>>>,
    /// 平台句柄喵
    platform: Arc<Win32Platform>,
    /// 托盘句柄喵
    tray: TrayHandle,
}

impl Tray {
    /// 装配系统托盘喵: 创建托盘、设置图标/提示/菜单、绑定 handler 喵
    ///
    /// 返回托盘句柄,供调用方在退出时移除喵。
    pub fn spawn(
        platform: Arc<Win32Platform>,
        state: SharedState,
        commands: Rc<RefCell<VecDeque<Command>>>,
    ) -> TrayHandle {
        let tray = platform.create_tray().unwrap_or_else(|| {
            log::error!("托盘创建失败喵~");
            std::process::exit(1);
        });

        // 主题预设均为浅色,托盘直接用应用同款 ico(任何任务栏底色都醒目)喵
        let mut handler = Tray {
            state: state.clone(),
            commands,
            platform: platform.clone(),
            tray,
        };

        // 图标(exe 内嵌资源) + 提示 + 菜单喵
        handler.apply_icon();
        platform.set_tray_tip(&tray, "meow app launcher");
        handler.refresh_menu();

        platform.set_tray_handler(&tray, Box::new(handler));

        tray
    }

    /// 重新应用托盘图标喵(与应用图标保持一致)喵
    fn apply_icon(&mut self) {
        self.platform.set_tray_app_icon(&self.tray);
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
                label: "扫描应用".into(),
                enabled: true,
            },
            TrayMenuItem {
                label: "设置".into(),
                enabled: true,
            },
            TrayMenuItem {
                label: "应用管理".into(),
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
        match event {
            TrayEvent::LeftClick => {
                log::info!("托盘单击,切换启动器喵");
                self.commands.borrow_mut().push_back(Command::ToggleLauncher);
            }
            TrayEvent::Menu(i) => {
                log::info!("托盘菜单项 {i} 喵");
                let cmd = match i {
                    0 => Some(Command::ToggleLauncher),
                    1 => Some(Command::Rescan),
                    2 => Some(Command::OpenSettings),
                    3 => Some(Command::OpenAppManager),
                    4 => Some(Command::Restart),
                    5 => Some(Command::Quit),
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
