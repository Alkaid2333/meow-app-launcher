//! 全局配置结构 + JSON 持久化喵~
//!
//! 配置文件生成在 `./.datas/config.json` 喵,首次启动自动创建默认配置喵。

use crate::animation::IslandConfig;
use crate::platform::SystemCommandKind;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 数据目录名: 配置、应用注册、图标快照都放这里喵
pub const DATA_DIR_NAME: &str = ".datas";
/// 配置文件路径(相对数据目录)喵
pub const CONFIG_FILE_NAME: &str = "config.json";
/// 应用注册文件路径(相对数据目录)喵
pub const APPS_FILE_NAME: &str = "apps.json";

/// 计算数据目录喵(便携式: 固定在可执行文件同目录的 `.datas` 下)喵
///
/// 不能依赖 `current_dir()`——从终端跑 `meowal register` 时 CWD 是终端目录,
/// 与 GUI 启动时的 CWD 不一致会导致注册到不同的 .datas,指令注册「无效」喵。
pub fn data_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(DATA_DIR_NAME)
}

/// 主题预设喵: 材质与配色融合成一体,
/// 三档浅色 + 一档深色,文字色与底材成对出现,任何预设下对比度都有保证喵。
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
    /// 深色: 暗夜玻璃,浅色文字喵(配色借鉴 winisland,见 README 喵)
    Dark,
}

impl ThemePreset {
    /// 全部档位喵(配置 GUI 循环选项的数据驱动化用)喵
    pub const ALL: [ThemePreset; 4] = [Self::FrostedGlass, Self::Mica, Self::Opaque, Self::Dark];

    /// 给配置 GUI 看的名字喵
    pub fn label(self) -> &'static str {
        match self {
            Self::FrostedGlass => "毛玻璃",
            Self::Mica => "云母",
            Self::Opaque => "不透明",
            Self::Dark => "深色",
        }
    }

    /// 循环切换预设喵
    pub fn cycle(self) -> Self {
        match self {
            Self::FrostedGlass => Self::Mica,
            Self::Mica => Self::Opaque,
            Self::Opaque => Self::Dark,
            Self::Dark => Self::FrostedGlass,
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

impl AppLayout {
    /// 全部档位喵(配置 GUI 循环选项用)喵
    pub const ALL: [AppLayout; 2] = [Self::Grid, Self::Row];

    /// 给配置 GUI 看的名字喵
    pub fn label(self) -> &'static str {
        match self {
            Self::Grid => "网格",
            Self::Row => "列表",
        }
    }

    /// 循环切换喵
    pub fn cycle(self) -> Self {
        match self {
            Self::Grid => Self::Row,
            Self::Row => Self::Grid,
        }
    }
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

impl SearchMode {
    /// 全部档位喵(配置 GUI 循环选项用)喵
    pub const ALL: [SearchMode; 3] = [Self::Name, Self::Tag, Self::Initial];

    /// 给配置 GUI 看的名字喵
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "名称",
            Self::Tag => "标签 t:",
            Self::Initial => "首字母 i:",
        }
    }
}

/// 渲染后端喵(可切换,GPU 初始化失败时自动回退 CPU)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RenderBackend {
    /// CPU 光栅渲染喵(默认,兼容性最好)喵
    #[default]
    Cpu,
    /// GPU 渲染喵(Windows 走 OpenGL/WGL,失败回退 CPU)喵
    Gpu,
}

impl RenderBackend {
    /// 全部档位喵(配置 GUI 循环选项用)喵
    pub const ALL: [RenderBackend; 2] = [Self::Cpu, Self::Gpu];

    /// 给配置 GUI 看的名字喵
    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Gpu => "GPU",
        }
    }

    /// 循环切换喵
    pub fn cycle(self) -> Self {
        match self {
            Self::Cpu => Self::Gpu,
            Self::Gpu => Self::Cpu,
        }
    }
}

/// 提权启动的修饰键喵
///
/// 语义: 在启动器里按住这个修饰键再确认启动,就以管理员身份运行选中项喵。
/// Windows 上管理员权限 = UAC 提权,Linux 未来对应 `sudo` —— 平台细节不在这里掺和喵。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ElevateModifier {
    /// 关闭提权启动喵
    Disabled,
    /// 默认: 按住 Shift 启动喵
    #[default]
    Shift,
    Ctrl,
    Alt,
    Win,
}

impl ElevateModifier {
    /// 全部档位喵(配置 GUI 循环选项用)喵
    pub const ALL: [ElevateModifier; 5] = [
        Self::Disabled,
        Self::Shift,
        Self::Ctrl,
        Self::Alt,
        Self::Win,
    ];

    /// 给配置 GUI 看的名字喵
    pub fn label(self) -> &'static str {
        match self {
            Self::Disabled => "关闭",
            Self::Shift => "Shift",
            Self::Ctrl => "Ctrl",
            Self::Alt => "Alt",
            Self::Win => "Win",
        }
    }

    /// 循环切换喵
    pub fn cycle(self) -> Self {
        let all = Self::ALL;
        let i = all.iter().position(|&m| m == self).unwrap_or(0);
        all[(i + 1) % all.len()]
    }

    /// 对应的平台修饰键喵(关闭时为 None)喵
    pub fn modifier(self) -> Option<crate::platform::Modifier> {
        match self {
            Self::Disabled => None,
            Self::Shift => Some(crate::platform::Modifier::Shift),
            Self::Ctrl => Some(crate::platform::Modifier::Ctrl),
            Self::Alt => Some(crate::platform::Modifier::Alt),
            Self::Win => Some(crate::platform::Modifier::Win),
        }
    }
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

/// Web 搜索引擎喵(跳转默认浏览器,零网络依赖)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WebEngine {
    /// 百度喵
    #[default]
    Baidu,
    /// 必应喵
    Bing,
    /// 搜狗喵
    Sogou,
    /// 谷歌喵
    Google,
    /// DuckDuckGo 喵
    Duckduckgo,
}

impl WebEngine {
    /// 全部档位喵(配置 GUI 循环选项用)喵
    pub const ALL: [WebEngine; 5] = [
        Self::Baidu,
        Self::Bing,
        Self::Sogou,
        Self::Google,
        Self::Duckduckgo,
    ];

    /// 给配置 GUI 看的名字喵
    pub fn label(self) -> &'static str {
        match self {
            Self::Baidu => "百度",
            Self::Bing => "必应",
            Self::Sogou => "搜狗",
            Self::Google => "谷歌",
            Self::Duckduckgo => "DuckDuckGo",
        }
    }

    /// 生成搜索链接喵(查询词自动百分号编码)喵
    pub fn search_url(self, query: &str) -> String {
        let q = crate::utils::percent_encode_component(query);
        match self {
            Self::Baidu => format!("https://www.baidu.com/s?wd={q}"),
            Self::Bing => format!("https://www.bing.com/search?q={q}"),
            Self::Sogou => format!("https://www.sogou.com/web?query={q}"),
            Self::Google => format!("https://www.google.com/search?q={q}"),
            Self::Duckduckgo => format!("https://duckduckgo.com/?q={q}"),
        }
    }
}

/// 系统指令条目喵: 一条命令 + 任意多个触发别名(中英文/拼音缩写随配)喵
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandEntry {
    /// 触发别名喵(任一别名命中即出结果)喵
    #[serde(default)]
    pub aliases: Vec<String>,
    /// 对应的系统命令喵
    pub kind: SystemCommandKind,
}

/// 默认指令表喵(中英双语,老配置缺字段时回填)喵
fn default_commands() -> Vec<CommandEntry> {
    use crate::platform::SystemCommandKind as K;
    vec![
        CommandEntry { aliases: vec!["锁屏".into(), "lock".into()], kind: K::Lock },
        CommandEntry { aliases: vec!["睡眠".into(), "sleep".into()], kind: K::Sleep },
        CommandEntry { aliases: vec!["关机".into(), "shutdown".into()], kind: K::Shutdown },
        CommandEntry {
            aliases: vec!["重启".into(), "restart".into(), "reboot".into()],
            kind: K::Restart,
        },
    ]
}

/// 搜索配置喵
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchConfig {
    /// 默认搜索模式喵
    pub default_mode: SearchMode,
    /// 过滤关键词规则喵(扫描/浏览/搜索时统一排除;旧配置缺字段时回填默认)喵
    #[serde(default = "default_filters")]
    pub filters: Vec<FilterRule>,
    /// Web 搜索引擎喵(旧配置缺字段时回填默认)喵
    #[serde(default)]
    pub web_engine: WebEngine,
    /// Web 搜索用的浏览器喵(空 = 系统默认;支持含 %1 占位符的路径)喵
    #[serde(default)]
    pub web_browser: String,
    /// 指令模块喵(系统命令别名表,可自由增删改)喵
    #[serde(default = "default_commands")]
    pub commands: Vec<CommandEntry>,
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
            web_engine: WebEngine::default(),
            web_browser: String::new(),
            commands: default_commands(),
        }
    }
}

/// 全局配置喵
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    /// 全局热键喵
    pub hotkey: HotkeyConfig,
    /// 提权启动修饰键喵(按住它启动 = 以管理员身份运行;默认 Shift)喵
    #[serde(default)]
    pub elevate_modifier: ElevateModifier,
    /// 浮窗窗口喵
    pub window: WindowConfig,
    /// 主题喵
    pub theme: ThemeConfig,
    /// 搜索喵
    pub search: SearchConfig,
    /// 灵动岛几何 / 动画 / 视觉喵
    #[serde(default)]
    pub island: IslandConfig,
    /// 开机自启喵(仅 Windows 生效,写注册表 Run 键;默认关闭)喵
    #[serde(default)]
    pub auto_start: bool,
    /// 渲染后端喵(CPU/GPU 可切换,GPU 失败自动回退 CPU)喵
    #[serde(default)]
    pub render_backend: RenderBackend,
    /// 配置面板动效喵(滚动平滑 + 页面过渡;默认开启)喵
    #[serde(default = "default_true")]
    pub settings_anim: bool,
}

/// serde 默认值辅助: 恒真喵
fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        let island = IslandConfig::default();
        Self {
            hotkey: HotkeyConfig::default(),
            elevate_modifier: ElevateModifier::default(),
            window: WindowConfig::default(),
            theme: ThemeConfig::default(),
            search: SearchConfig::default(),
            island,
            auto_start: false,
            render_backend: RenderBackend::default(),
            settings_anim: true,
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