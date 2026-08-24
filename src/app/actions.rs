//! 动作定义喵~ 可跨平台分发的命令喵。
//!
//! 第一阶段只有几个动作: 呼出/隐藏搜索浮窗、打开配置、退出喵。
//! 后续绑定到全局热键、托盘菜单喵。

/// 呼出/隐藏聚焦搜索浮窗喵
#[derive(Clone, Debug, PartialEq, Eq, Hash, gpui::Action)]
pub struct ToggleLauncher;

/// 打开配置界面喵(第二阶段实现,先占位喵)
#[derive(Clone, Debug, PartialEq, Eq, Hash, gpui::Action)]
pub struct OpenSettings;

/// 退出应用喵
#[derive(Clone, Debug, PartialEq, Eq, Hash, gpui::Action)]
pub struct QuitApp;
