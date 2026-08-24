//! 主题喵~
//!
//! 语义化配色 token,浅色/深色两套基准主题喵。
//! 遵循 window-design skill 的铁律: 页面里禁止写死色值,一律走主题 token 喵。
//! 后续可扩展为从 theme 配置文件加载自定义主题喵。

use crate::app::config::ThemeMode;
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
}

impl Theme {
    /// 按配置模式取主题喵
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
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
        }
    }
}
