//! 主题喵~
//!
//! 语义化配色 token,浅色/深色两套基准主题喵。
//! 遵循 window-design skill 的铁律: 页面里禁止写死色值,一律走主题 token 喵。
//! 后续可扩展为从 theme 配置文件加载自定义主题喵。

use crate::app::config::{Backdrop, ThemeMode};
use skia_safe::Color;

/// 启动器主题色板喵
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// 浮窗/面板背景喵
    pub background: Color,
    /// 描边喵
    pub border: Color,
    /// 主文字喵
    pub text: Color,
    /// 次级文字(副标题/路径)喵
    pub text_dim: Color,
    /// 悬停/选中背景喵
    pub hover_bg: Color,
    /// 强调色(选中指示条/兜底图标)喵
    pub accent: Color,
    /// 强调色上的文字喵
    pub accent_text: Color,
    /// 面板阴影喵
    pub shadow: Color,
    /// 背景材质喵
    pub backdrop: Backdrop,
    /// 材质填充(含透明度)喵
    pub fill: Color,
    /// 顶沿高光喵
    pub highlight: Color,
}

impl Theme {
    /// 按配置模式取主题喵
    pub fn for_mode(mode: ThemeMode, backdrop: Backdrop) -> Self {
        let mut theme = match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        };
        theme.backdrop = backdrop;
        theme.fill = match backdrop {
            Backdrop::Opaque => theme.background,
            Backdrop::Mica => with_alpha(theme.background, if mode == ThemeMode::Light { 0xD8 } else { 0xC0 }),
            Backdrop::Acrylic => with_alpha(theme.background, if mode == ThemeMode::Light { 0xB8 } else { 0xA0 }),
        };
        theme
    }

    /// 浅色主题喵~ 清爽透亮喵!
    pub fn light() -> Self {
        Self {
            background: Color::from_argb(0xFF, 0xF5, 0xF7, 0xFA),
            border: Color::from_argb(0xFF, 0xD0, 0xD7, 0xE2),
            text: Color::from_rgb(0x1F, 0x24, 0x30),
            text_dim: Color::from_rgb(0x6B, 0x72, 0x80),
            hover_bg: Color::from_argb(0xFF, 0xE8, 0xED, 0xF5),
            accent: Color::from_rgb(0x6C, 0x5C, 0xE7),
            accent_text: Color::from_rgb(0xFF, 0xFF, 0xFF),
            shadow: Color::from_argb(0x28, 0x00, 0x00, 0x00),
            backdrop: Backdrop::Opaque,
            fill: Color::from_argb(0xFF, 0xF5, 0xF7, 0xFA),
            highlight: Color::from_argb(0x40, 0xFF, 0xFF, 0xFF),
        }
    }

    /// 深色主题喵~ 深夜护眼喵!
    pub fn dark() -> Self {
        Self {
            background: Color::from_argb(0xFF, 0x14, 0x16, 0x1B),
            border: Color::from_argb(0xFF, 0x2A, 0x2F, 0x3A),
            text: Color::from_rgb(0xE8, 0xEA, 0xED),
            text_dim: Color::from_rgb(0x8B, 0x93, 0xA1),
            hover_bg: Color::from_argb(0xFF, 0x23, 0x28, 0x33),
            accent: Color::from_rgb(0x8B, 0x7C, 0xF6),
            accent_text: Color::from_rgb(0x14, 0x16, 0x1B),
            shadow: Color::from_argb(0x50, 0x00, 0x00, 0x00),
            backdrop: Backdrop::Opaque,
            fill: Color::from_argb(0xFF, 0x14, 0x16, 0x1B),
            highlight: Color::from_argb(0x28, 0xFF, 0xFF, 0xFF),
        }
    }
}

fn with_alpha(color: Color, alpha: u8) -> Color {
    Color::from_argb(alpha, color.r(), color.g(), color.b())
}

/// 配置 GUI 主题色板喵(语义 token,浅色/深色两套)喵
///
/// 遵循 window-design skill 的配置 GUI 配色 token 喵。
#[derive(Debug, Clone, Copy)]
pub struct SettingsTheme {
    /// 窗口底色喵
    pub win_bg: Color,
    /// 侧边栏底色喵
    pub sidebar_bg: Color,
    /// 分组卡片底色喵
    pub group_bg: Color,
    /// 主文字喵
    pub text: Color,
    /// 次级文字喵
    pub text_dim: Color,
    /// 禁用文字喵
    pub disabled: Color,
    /// 强调色喵
    pub accent: Color,
    /// 危险色(关闭按钮)喵
    pub danger: Color,
    /// 开关开启色喵
    pub toggle_on: Color,
    /// 开关关闭色喵
    pub toggle_off: Color,
    /// 控件背景喵
    pub control_bg: Color,
    /// 控件描边喵
    pub control_border: Color,
    /// 阴影喵
    pub shadow: Color,
}

impl SettingsTheme {
    /// 按配置模式取主题喵
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }

    /// 浅色主题喵
    pub fn light() -> Self {
        Self {
            win_bg: Color::from_argb(0xFF, 0xF6, 0xF6, 0xF8),
            sidebar_bg: Color::from_argb(0xFF, 0xEB, 0xEB, 0xEE),
            group_bg: Color::from_argb(0xFF, 0xFF, 0xFF, 0xFF),
            text: Color::from_rgb(0x1C, 0x1C, 0x1E),
            text_dim: Color::from_rgb(0x5C, 0x5C, 0x61),
            disabled: Color::from_rgb(0x8E, 0x8E, 0x93),
            accent: Color::from_rgb(0x00, 0x7A, 0xFF),
            danger: Color::from_rgb(0xFF, 0x3B, 0x30),
            toggle_on: Color::from_rgb(0x34, 0xC7, 0x59),
            toggle_off: Color::from_rgb(0xC7, 0xC7, 0xCC),
            control_bg: Color::from_argb(0xFF, 0xFF, 0xFF, 0xFF),
            control_border: Color::from_argb(0x34, 0x00, 0x00, 0x00),
            shadow: Color::from_argb(0x18, 0x00, 0x00, 0x00),
        }
    }

    /// 深色主题喵
    pub fn dark() -> Self {
        Self {
            win_bg: Color::from_rgb(0x1C, 0x1C, 0x1E),
            sidebar_bg: Color::from_rgb(0x16, 0x16, 0x18),
            group_bg: Color::from_rgb(0x28, 0x28, 0x2A),
            text: Color::from_rgb(0xE8, 0xE8, 0xEA),
            text_dim: Color::from_rgb(0xA0, 0xA0, 0xA4),
            disabled: Color::from_rgb(0x5C, 0x5C, 0x61),
            accent: Color::from_rgb(0x0A, 0x84, 0xFF),
            danger: Color::from_rgb(0xFF, 0x45, 0x3A),
            toggle_on: Color::from_rgb(0x32, 0xD7, 0x4B),
            toggle_off: Color::from_rgb(0x4A, 0x4A, 0x4C),
            control_bg: Color::from_rgb(0x2C, 0x2C, 0x2E),
            control_border: Color::from_argb(0x28, 0xFF, 0xFF, 0xFF),
            shadow: Color::from_argb(0x40, 0x00, 0x00, 0x00),
        }
    }
}
