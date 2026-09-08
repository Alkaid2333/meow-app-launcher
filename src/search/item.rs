//! 多源搜索结果模型喵~
//!
//! 一切搜索结果(应用/计算器/Web/系统命令/未来的文件与插件)
//! 统一成 [`SearchItem`],条目展示与「被激活时执行什么」([`Action`])彻底分离喵:
//! 这样渲染层只管画,执行层只管做,新增搜索源时两边都不用改喵。

use crate::apps::AppInfo;
use crate::platform::SystemCommandKind;

/// 条目被激活(回车/点击)时执行的动作喵
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// 启动可执行路径(exe/lnk/任意 Shell 可打开类型)喵
    Launch(String),
    /// 用默认浏览器打开链接喵
    OpenUrl(String),
    /// 复制文本到剪贴板喵
    CopyText(String),
    /// 执行系统命令(仅内置预定义,不接受任意输入)喵
    SystemCommand(SystemCommandKind),
}

/// 条目图标喵
#[derive(Debug, Clone)]
pub enum ItemIcon {
    /// 应用: 走 IconManager 缓存图标,缺图兜底字符图标喵
    App(AppInfo),
    /// 内置 SVG 线稿图标喵
    Builtin(BuiltinIcon),
}

/// 内置 SVG 图标集喵(assets/icons 内嵌素材)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinIcon {
    /// 计算器喵
    Calculator,
    /// 网页/地球喵
    Web,
    /// 电源/系统命令喵
    Power,
}

impl BuiltinIcon {
    /// 对应 svg.rs 内置图标名喵
    pub fn svg_name(self) -> &'static str {
        match self {
            Self::Calculator => "calculator",
            Self::Web => "web",
            Self::Power => "power",
        }
    }
}

/// 统一搜索结果条目喵
#[derive(Debug, Clone)]
pub struct SearchItem {
    /// 主标题(第一行)喵
    pub title: String,
    /// 副标题(第二行,可省略;网格模式不显示)喵
    pub subtitle: Option<String>,
    /// 图标喵
    pub icon: ItemIcon,
    /// 激活动作喵
    pub action: Action,
}

impl SearchItem {
    /// 由应用信息构造条目喵(副标题 = 标签 · 路径)喵
    pub fn from_app(app: &AppInfo) -> Self {
        let subtitle = if app.tags.is_empty() {
            app.path.clone()
        } else {
            format!("{}  ·  {}", app.tags.join(" / "), app.path)
        };
        Self {
            title: app.name.clone(),
            subtitle: Some(subtitle),
            icon: ItemIcon::App(app.clone()),
            action: Action::Launch(app.path.clone()),
        }
    }

    /// 兜底字符图标用的短文本喵(取标题前两个字符)喵
    pub fn fallback_label(&self) -> String {
        let chars: String = self.title.chars().take(2).collect();
        if chars.is_empty() { "?".to_string() } else { chars }
    }
}

/// 带排序分的条目喵(聚合层排序后剥掉分数喵)
#[derive(Debug, Clone)]
pub struct Scored {
    /// 匹配分喵(越大越靠前)
    pub score: f32,
    /// 条目本体喵
    pub item: SearchItem,
}

impl Scored {
    /// 快捷构造喵
    pub fn new(score: f32, item: SearchItem) -> Self {
        Self { score, item }
    }
}
