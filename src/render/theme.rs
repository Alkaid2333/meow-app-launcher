//! 主题喵~
//!
//! 语义化配色 token。岛视觉(ink/glass/outline) × 浅/深色喵。
//! 页面里禁止写死色值,一律走主题 token 喵。

use crate::animation::IslandVisual;
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
    /// 次级文字喵
    pub text_dim: Color,
    /// 悬停/选中背景喵
    pub hover_bg: Color,
    /// 强调色喵
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
    /// 岛视觉喵
    pub visual: IslandVisual,
}

impl Theme {
    /// 按配置取主题喵
    pub fn resolve(mode: ThemeMode, backdrop: Backdrop, visual: IslandVisual) -> Self {
        let mut theme = match (visual, mode) {
            (IslandVisual::Ink, _) => Self::ink(),
            (IslandVisual::Outline, ThemeMode::Dark) => Self::outline_dark(),
            (IslandVisual::Outline, ThemeMode::Light) => Self::outline(),
            (IslandVisual::Glass, ThemeMode::Dark) => Self::glass_dark(),
            (IslandVisual::Glass, ThemeMode::Light) => Self::glass(),
        };
        theme.backdrop = backdrop;
        theme.visual = visual;
        theme.fill = match (visual, backdrop) {
            (IslandVisual::Glass, _) => theme.fill,
            (_, Backdrop::Opaque) => theme.background,
            (_, Backdrop::Mica) => {
                with_alpha(theme.background, if mode == ThemeMode::Light { 0xD8 } else { 0xC0 })
            }
            (_, Backdrop::Acrylic) => {
                with_alpha(theme.background, if mode == ThemeMode::Light { 0xB8 } else { 0xA0 })
            }
        };
        theme
    }

    /// 墨黑胶囊喵
    pub fn ink() -> Self {
        Self {
            background: Color::from_argb(0xFF, 0x0B, 0x0B, 0x0D),
            border: Color::from_argb(0x33, 0xFF, 0xFF, 0xFF),
            text: Color::from_rgb(0xF4, 0xF1, 0xEA),
            text_dim: Color::from_rgb(0x8C, 0x85, 0x77),
            hover_bg: Color::from_argb(0xFF, 0x1A, 0x18, 0x16),
            accent: Color::from_rgb(0xD9, 0x48, 0x1B),
            accent_text: Color::from_rgb(0xFB, 0xE7, 0xDE),
            shadow: Color::from_argb(0x66, 0x00, 0x00, 0x00),
            backdrop: Backdrop::Opaque,
            fill: Color::from_argb(0xFF, 0x0B, 0x0B, 0x0D),
            highlight: Color::from_argb(0x28, 0xFF, 0xFF, 0xFF),
            visual: IslandVisual::Ink,
        }
    }

    /// 玻璃喵
    pub fn glass() -> Self {
        Self {
            background: Color::from_argb(0x80, 0x0E, 0x0E, 0x11),
            border: Color::from_argb(0x40, 0xFF, 0xFF, 0xFF),
            text: Color::from_rgb(0x16, 0x15, 0x0F),
            text_dim: Color::from_rgb(0x4B, 0x46, 0x3A),
            hover_bg: Color::from_argb(0x40, 0xFF, 0xFF, 0xFF),
            accent: Color::from_rgb(0x1F, 0x6B, 0x58),
            accent_text: Color::from_rgb(0xFF, 0xFF, 0xFF),
            shadow: Color::from_argb(0x28, 0x00, 0x00, 0x00),
            backdrop: Backdrop::Acrylic,
            fill: Color::from_argb(0x80, 0x0E, 0x0E, 0x11),
            highlight: Color::from_argb(0x50, 0xFF, 0xFF, 0xFF),
            visual: IslandVisual::Glass,
        }
    }

    pub fn glass_dark() -> Self {
        let mut t = Self::glass();
        t.text = Color::from_rgb(0xF4, 0xF1, 0xEA);
        t.text_dim = Color::from_rgb(0x8C, 0x85, 0x77);
        t.fill = Color::from_argb(0x90, 0x0E, 0x0E, 0x11);
        t
    }

    /// 线稿喵
    pub fn outline() -> Self {
        Self {
            background: Color::from_argb(0xFF, 0xFB, 0xF9, 0xF5),
            border: Color::from_argb(0xFF, 0x16, 0x15, 0x0F),
            text: Color::from_rgb(0x16, 0x15, 0x0F),
            text_dim: Color::from_rgb(0x4B, 0x46, 0x3A),
            hover_bg: Color::from_argb(0xFF, 0xF4, 0xF1, 0xEA),
            accent: Color::from_rgb(0xD9, 0x48, 0x1B),
            accent_text: Color::from_rgb(0xFB, 0xF9, 0xF5),
            shadow: Color::from_argb(0x14, 0x00, 0x00, 0x00),
            backdrop: Backdrop::Opaque,
            fill: Color::from_argb(0xFF, 0xFB, 0xF9, 0xF5),
            highlight: Color::TRANSPARENT,
            visual: IslandVisual::Outline,
        }
    }

    pub fn outline_dark() -> Self {
        Self {
            background: Color::from_argb(0xFF, 0x16, 0x15, 0x0F),
            border: Color::from_argb(0xFF, 0xF4, 0xF1, 0xEA),
            text: Color::from_rgb(0xF4, 0xF1, 0xEA),
            text_dim: Color::from_rgb(0x8C, 0x85, 0x77),
            hover_bg: Color::from_argb(0xFF, 0x24, 0x22, 0x1A),
            accent: Color::from_rgb(0xD9, 0x48, 0x1B),
            accent_text: Color::from_rgb(0x16, 0x15, 0x0F),
            shadow: Color::from_argb(0x40, 0x00, 0x00, 0x00),
            backdrop: Backdrop::Opaque,
            fill: Color::from_argb(0xFF, 0x16, 0x15, 0x0F),
            highlight: Color::TRANSPARENT,
            visual: IslandVisual::Outline,
        }
    }
}

fn with_alpha(color: Color, alpha: u8) -> Color {
    Color::from_argb(alpha, color.r(), color.g(), color.b())
}

/// 配置 GUI 主题色板喵
#[derive(Debug, Clone, Copy)]
pub struct SettingsTheme {
    pub win_bg: Color,
    pub sidebar_bg: Color,
    pub group_bg: Color,
    pub text: Color,
    pub text_dim: Color,
    pub disabled: Color,
    pub accent: Color,
    pub danger: Color,
    pub toggle_on: Color,
    pub toggle_off: Color,
    pub control_bg: Color,
    pub control_border: Color,
    pub shadow: Color,
    pub moss: Color,
}

impl SettingsTheme {
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }

    /// 暖纸浅色,不要苹果灰喵
    pub fn light() -> Self {
        Self {
            win_bg: Color::from_rgb(0xF4, 0xF1, 0xEA),
            sidebar_bg: Color::from_rgb(0xE7, 0xE1, 0xD4),
            group_bg: Color::from_rgb(0xFB, 0xF9, 0xF5),
            text: Color::from_rgb(0x16, 0x15, 0x0F),
            text_dim: Color::from_rgb(0x4B, 0x46, 0x3A),
            disabled: Color::from_rgb(0x8C, 0x85, 0x77),
            accent: Color::from_rgb(0xD9, 0x48, 0x1B),
            danger: Color::from_rgb(0xD9, 0x48, 0x1B),
            toggle_on: Color::from_rgb(0x1F, 0x6B, 0x58),
            toggle_off: Color::from_rgb(0xDE, 0xD9, 0xCC),
            control_bg: Color::from_rgb(0xFF, 0xFF, 0xFF),
            control_border: Color::from_argb(0x40, 0x16, 0x15, 0x0F),
            shadow: Color::from_argb(0x18, 0x16, 0x15, 0x0F),
            moss: Color::from_rgb(0x1F, 0x6B, 0x58),
        }
    }

    /// 墨夜深色喵
    pub fn dark() -> Self {
        Self {
            win_bg: Color::from_rgb(0x12, 0x11, 0x0E),
            sidebar_bg: Color::from_rgb(0x0B, 0x0B, 0x0D),
            group_bg: Color::from_rgb(0x1C, 0x1A, 0x16),
            text: Color::from_rgb(0xF4, 0xF1, 0xEA),
            text_dim: Color::from_rgb(0x8C, 0x85, 0x77),
            disabled: Color::from_rgb(0x4B, 0x46, 0x3A),
            accent: Color::from_rgb(0xD9, 0x48, 0x1B),
            danger: Color::from_rgb(0xD9, 0x48, 0x1B),
            toggle_on: Color::from_rgb(0x1F, 0x6B, 0x58),
            toggle_off: Color::from_rgb(0x2A, 0x27, 0x22),
            control_bg: Color::from_rgb(0x24, 0x22, 0x1C),
            control_border: Color::from_argb(0x28, 0xF4, 0xF1, 0xEA),
            shadow: Color::from_argb(0x50, 0x00, 0x00, 0x00),
            moss: Color::from_rgb(0x3D, 0xA3, 0x88),
        }
    }
}
