//! 全局配置结构 + JSON 持久化喵~
//!
//! 配置文件生成在 `./.datas/config.json` 喵,首次启动自动创建默认配置喵。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 数据目录名: 配置、应用注册、图标快照都放这里喵
pub const DATA_DIR_NAME: &str = ".datas";
/// 配置文件路径(相对数据目录)喵
pub const CONFIG_FILE_NAME: &str = "config.json";
/// 应用注册文件路径(相对数据目录)喵
pub const APPS_FILE_NAME: &str = "apps.json";

/// 主题模式喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    /// 浅色喵
    #[default]
    Light,
    /// 深色喵
    Dark,
}

/// 应用排列样式喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppLayout {
    /// 网格显示喵
    Grid,
    /// 行显示喵
    #[default]
    Row,
}

/// 默认搜索模式喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// 按名称搜索喵(默认)
    #[default]
    Name,
    /// 按 Tag 搜索喵(`t: xxx` 前缀)
    Tag,
    /// 按首字母搜索喵(`i: a b c` 前缀)
    Initial,
}

/// 全局热键配置喵
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// 是否启用全局热键喵
    pub enabled: bool,
    /// 修饰键: "ctrl" / "alt" / "shift" / "win"(可组合 "+",如 "ctrl+alt")喵
    pub modifiers: String,
    /// 触发键: 如 "space" / "f" / "`" 喵
    pub key: String,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            modifiers: "ctrl+alt".into(),
            key: "space".into(),
        }
    }
}

/// 浮窗窗口配置喵
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowConfig {
    /// 窗口宽度(px)喵
    pub width: f32,
    /// 窗口高度(px)喵
    pub height: f32,
    /// 是否固定在屏幕顶部置顶喵
    pub always_on_top: bool,
    /// 应用排列样式喵
    pub layout: AppLayout,
    /// 图标显示大小(px)喵
    pub icon_size: f32,
    /// 是否显示最近打开的应用喵
    pub show_recent: bool,
    /// 是否显示收藏的应用喵
    pub show_favorites: bool,
    /// 是否显示最常用的应用喵
    pub show_frequent: bool,
    /// 是否始终显示所有应用喵
    pub show_all: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 640.0,
            height: 480.0,
            always_on_top: true,
            layout: AppLayout::Row,
            icon_size: 36.0,
            show_recent: false,
            show_favorites: false,
            show_frequent: false,
            show_all: true,
        }
    }
}

/// 主题配置喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// 主题模式喵
    pub mode: ThemeMode,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Light,
        }
    }
}

/// 搜索配置喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchConfig {
    /// 默认搜索模式喵
    pub default_mode: SearchMode,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_mode: SearchMode::Name,
        }
    }
}

/// 全局配置喵
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    /// 全局热键喵
    pub hotkey: HotkeyConfig,
    /// 浮窗窗口喵
    pub window: WindowConfig,
    /// 主题喵
    pub theme: ThemeConfig,
    /// 搜索喵
    pub search: SearchConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: HotkeyConfig::default(),
            window: WindowConfig::default(),
            theme: ThemeConfig::default(),
            search: SearchConfig::default(),
        }
    }
}

impl AppConfig {
    /// 加载配置喵,文件不存在或解析失败时生成默认配置喵~
    pub fn load(data_dir: &PathBuf) -> Self {
        let path = data_dir.join(CONFIG_FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<AppConfig>(&raw) {
                Ok(config) => {
                    log::debug!("配置加载成功: {:?}", path);
                    config
                }
                Err(e) => {
                    log::warn!("配置解析失败({e}),回退默认配置喵~");
                    Self::default()
                }
            },
            Err(_) => {
                log::info!("配置文件不存在,使用默认配置喵~");
                Self::default()
            }
        }
    }

    /// 保存配置到磁盘喵,保存失败只记日志不崩溃喵~
    pub fn save(&self, data_dir: &PathBuf) {
        let path = data_dir.join(CONFIG_FILE_NAME);
        match serde_json::to_string_pretty(self) {
            Ok(raw) => match std::fs::write(&path, raw) {
                Ok(_) => log::debug!("配置已保存: {:?}", path),
                Err(e) => log::error!("配置保存失败: {e}"),
            },
            Err(e) => log::error!("配置序列化失败: {e}"),
        }
    }
}
