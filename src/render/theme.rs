//! 主题喵~
//!
//! 材质与配色融合为「主题预设」: 毛玻璃 / 云母 / 不透明,三档均为浅色喵。
//! 每个预设的文字色与底材成对提供,任何组合下对比度都有保证,
//! 不再出现「玻璃材质配黑字看不清」的问题喵。
//! 页面里禁止写死色值,一律走主题 token 喵。

use crate::app::config::ThemePreset;
use skia_safe::Color;

/// 启动器主题色板喵
#[derive(Debug, Clone, Copy)]
pub struct Theme {
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
    /// 岛体填充(含透明度,即材质)喵
    pub fill: Color,
    /// 是否绘制玻璃噪点(毛玻璃专属)喵
    pub glass: bool,
}

impl Theme {
    /// 按主题预设取主题喵
    pub fn resolve(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::FrostedGlass => Self {
                border: Color::from_argb(0x3C, 0x16, 0x15, 0x0F),
                text: Color::from_rgb(0x16, 0x15, 0x0F),
                text_dim: Color::from_rgb(0x55, 0x50, 0x44),
                hover_bg: Color::from_argb(0xFF, 0xEF, 0xEC, 0xE4),
                accent: Color::from_rgb(0xD9, 0x48, 0x1B),
                accent_text: Color::from_rgb(0xFF, 0xFF, 0xFF),
                shadow: Color::from_argb(0x24, 0x00, 0x00, 0x00),
                // 高不透明度白,保证深色文字清晰可读喵
                fill: Color::from_argb(0xF0, 0xFC, 0xFB, 0xF8),
                glass: true,
            },
            ThemePreset::Mica => Self {
                border: Color::from_argb(0x30, 0x16, 0x15, 0x0F),
                text: Color::from_rgb(0x16, 0x15, 0x0F),
                text_dim: Color::from_rgb(0x55, 0x50, 0x44),
                hover_bg: Color::from_argb(0xFF, 0xE9, 0xE5, 0xDC),
                // 云母预设走青苔绿强调色,与暖纸形成辨识度喵
                accent: Color::from_rgb(0x1F, 0x6B, 0x58),
                accent_text: Color::from_rgb(0xFF, 0xFF, 0xFF),
                shadow: Color::from_argb(0x18, 0x00, 0x00, 0x00),
                fill: Color::from_argb(0xF6, 0xF3, 0xF0, 0xEA),
                glass: false,
            },
            ThemePreset::Opaque => Self {
                border: Color::from_argb(0x55, 0x16, 0x15, 0x0F),
                text: Color::from_rgb(0x16, 0x15, 0x0F),
                text_dim: Color::from_rgb(0x4B, 0x46, 0x3A),
                hover_bg: Color::from_argb(0xFF, 0xEF, 0xEC, 0xE4),
                accent: Color::from_rgb(0xD9, 0x48, 0x1B),
                accent_text: Color::from_rgb(0xFF, 0xFF, 0xFF),
                shadow: Color::from_argb(0x14, 0x00, 0x00, 0x00),
                fill: Color::from_argb(0xFF, 0xF7, 0xF4, 0xEE),
                glass: false,
            },
            // 深色: 暗夜玻璃,浅色文字(配色借鉴 winisland,见 README 喵)喵
            ThemePreset::Dark => Self {
                border: Color::from_argb(0x38, 0xFF, 0xFF, 0xFF),
                text: Color::from_rgb(0xF5, 0xF5, 0xF7),
                text_dim: Color::from_rgb(0xAE, 0xAE, 0xB2),
                hover_bg: Color::from_rgb(0x3A, 0x3A, 0x3C),
                accent: Color::from_rgb(0x0A, 0x84, 0xFF),
                accent_text: Color::from_rgb(0xFF, 0xFF, 0xFF),
                shadow: Color::from_argb(0x48, 0x00, 0x00, 0x00),
                fill: Color::from_argb(0xEE, 0x1C, 0x1C, 0x1E),
                glass: true,
            },
        }
    }
}

fn with_alpha(color: Color, alpha: u8) -> Color {
    Color::from_argb(alpha, color.r(), color.g(), color.b())
}

/// 配置 GUI 主题色板喵(跟随主题预设,浅色 Fluent 风)喵
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
    /// 按主题预设取配置 GUI 色板喵
    pub fn for_preset(preset: ThemePreset) -> Self {
        // 深色整套独立色板喵
        if preset == ThemePreset::Dark {
            let mut t = Self::dark();
            t.win_bg = with_alpha(t.win_bg, 0xF2);
            return t;
        }
        let mut t = Self::light();
        // 窗口底色随预设微调: 毛玻璃半透明、云母带暖灰、不透明实心暖纸喵
        t.win_bg = match preset {
            ThemePreset::FrostedGlass => with_alpha(t.win_bg, 0xF2),
            ThemePreset::Mica => Color::from_rgb(0xF0, 0xEC, 0xE3),
            _ => Color::from_rgb(0xF4, 0xF1, 0xEA),
        };
        if preset == ThemePreset::Mica {
            t.accent = Color::from_rgb(0x1F, 0x6B, 0x58);
        }
        t
    }

    /// 暖纸浅色基准,不要苹果灰喵
    pub fn light() -> Self {
        Self {
            win_bg: Color::from_rgb(0xF4, 0xF1, 0xEA),
            sidebar_bg: Color::from_rgb(0xEC, 0xE6, 0xDA),
            group_bg: Color::from_rgb(0xFB, 0xF9, 0xF5),
            text: Color::from_rgb(0x16, 0x15, 0x0F),
            text_dim: Color::from_rgb(0x4B, 0x46, 0x3A),
            disabled: Color::from_rgb(0x8C, 0x85, 0x77),
            accent: Color::from_rgb(0xD9, 0x48, 0x1B),
            danger: Color::from_rgb(0xC2, 0x3B, 0x22),
            toggle_on: Color::from_rgb(0x1F, 0x6B, 0x58),
            toggle_off: Color::from_rgb(0xD6, 0xD0, 0xC3),
            control_bg: Color::from_rgb(0xFF, 0xFF, 0xFF),
            control_border: Color::from_argb(0x40, 0x16, 0x15, 0x0F),
            shadow: Color::from_argb(0x18, 0x16, 0x15, 0x0F),
            moss: Color::from_rgb(0x1F, 0x6B, 0x58),
        }
    }

    /// 暗夜深色基准喵(配色借鉴 winisland 的 dark_settings_theme,见 README 喵)喵
    pub fn dark() -> Self {
        Self {
            win_bg: Color::from_rgb(0x1C, 0x1C, 0x1E),
            sidebar_bg: Color::from_rgb(0x24, 0x24, 0x26),
            group_bg: Color::from_rgb(0x2C, 0x2C, 0x2E),
            text: Color::from_rgb(0xF5, 0xF5, 0xF7),
            text_dim: Color::from_rgb(0xAE, 0xAE, 0xB2),
            disabled: Color::from_rgb(0x63, 0x63, 0x66),
            accent: Color::from_rgb(0x0A, 0x84, 0xFF),
            danger: Color::from_rgb(0xE6, 0x37, 0x2D),
            toggle_on: Color::from_rgb(0x30, 0xD1, 0x58),
            toggle_off: Color::from_rgb(0x63, 0x63, 0x66),
            control_bg: Color::from_rgb(0x3A, 0x3A, 0x3C),
            control_border: Color::from_argb(0x28, 0xFF, 0xFF, 0xFF),
            shadow: Color::from_argb(0x48, 0x00, 0x00, 0x00),
            moss: Color::from_rgb(0x30, 0xD1, 0x58),
        }
    }
}