//! 全局配置结构 + JSON 持久化喵~
//!
//! 配置文件生成在 `./.datas/config.json` 喵,首次启动自动创建默认配置喵。

use crate::animation::IslandConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 数据目录名: 配置、应用注册、图标快照都放这里喵
pub const DATA_DIR_NAME: &str = ".datas";
/// 配置文件路径(相对数据目录)喵
pub const CONFIG_FILE_NAME: &str = "config.json";
/// 应用注册文件路径(相对数据目录)喵
pub const APPS_FILE_NAME: &str = "apps.json";

/// 主题预设喵: 材质与配色融合成一体,三档均为浅色,
/// 文字色与底材成对出现,任何预设下对比度都有保证喵。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    /// 毛玻璃: 半透明白 + 细噪点质感,深色文字喵
    #[default]
    FrostedGlass,
    /// 云母: 不透明桌面染色,深色文字、青苔绿强调色喵
    Mica,
    /// 不透明: 实心暖纸,描边更强,深色文字喵
    Opaque,
}

impl ThemePreset {
    /// 给配置 GUI 看的名字喵
    pub fn label(self) -> &'static str {
        match self {
            Self::FrostedGlass => "毛玻璃",
            Self::Mica => "云母",
            Self::Opaque => "不透明",
        }
    }

    /// 循环切换预设喵
    pub fn cycle(self) -> Self {
        match self {
            Self::FrostedGlass => Self::Mica,
            Self::Mica => Self::Opaque,
            Self::Opaque => Self::FrostedGlass,
        }
    }
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
    /// 展开面板宽度(px, 同步到 island.expanded_width)喵
    pub width: f64,
    /// 展开面板高度(px, 同步到 island.expanded_height)喵
    pub height: f64,
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
            width: 560.0,
            height: 268.0,
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

/// 主题配置喵(材质与配色已融合,仅一档预设可调)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// 主题预设喵
    #[serde(default)]
    pub preset: ThemePreset,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            preset: ThemePreset::FrostedGlass,
        }
    }
}

/// 过滤规则喵: 按关键词排除不需要的启动项喵
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterRule {
    /// 匹配关键词喵
    #[serde(default)]
    pub keyword: String,
    /// 是否大小写敏感喵
    #[serde(default)]
    pub case_sensitive: bool,
}

impl FilterRule {
    /// 是否命中应用名喵(空关键词恒不命中)喵
    pub fn matches(&self, name: &str) -> bool {
        if self.keyword.is_empty() {
            return false;
        }
        if self.case_sensitive {
            name.contains(self.keyword.as_str())
        } else {
            name.to_lowercase()
                .contains(&self.keyword.to_lowercase())
        }
    }
}

/// 搜索配置喵
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchConfig {
    /// 默认搜索模式喵
    pub default_mode: SearchMode,
    /// 过滤关键词规则喵(扫描/浏览/搜索时统一排除;旧配置缺字段时回填默认)喵
    #[serde(default = "default_filters")]
    pub filters: Vec<FilterRule>,
}

/// 默认过滤规则: 排除「卸载 / uninstall」类启动项喵
fn default_filters() -> Vec<FilterRule> {
    vec![
        FilterRule {
            keyword: "卸载".into(),
            case_sensitive: false,
        },
        FilterRule {
            keyword: "uninstall".into(),
            case_sensitive: false,
        },
    ]
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_mode: SearchMode::Name,
            filters: default_filters(),
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
    /// 灵动岛几何 / 动画 / 视觉喵
    #[serde(default)]
    pub island: IslandConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        let island = IslandConfig::default();
        Self {
            hotkey: HotkeyConfig::default(),
            window: WindowConfig {
                width: island.expanded_width,
                height: island.expanded_height,
                ..WindowConfig::default()
            },
            theme: ThemeConfig::default(),
            search: SearchConfig::default(),
            island,
        }
    }
}

impl AppConfig {
    /// 加载配置喵,文件不存在或解析失败时生成默认配置喵~
    pub fn load(data_dir: &Path) -> Self {
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

    /// 把窗口宽高回写到岛配置,保证两处同源喵
    pub fn sync_island_size(&mut self) {
        self.island.expanded_width = self.window.width;
        self.island.expanded_height = self.window.height;
        self.island.slot_height = self.island.height;
    }

    /// 应用是否被关键词过滤规则排除喵(名称命中任一规则即排除)喵
    pub fn is_app_filtered(&self, name: &str) -> bool {
        self.search.filters.iter().any(|f| f.matches(name))
    }

    /// 保存配置到磁盘喵,保存失败只记日志不崩溃喵~
    pub fn save(&self, data_dir: &Path) {
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