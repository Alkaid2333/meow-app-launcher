//! 主题喵~
//!
//! 第一阶段内置浅色/深色两套基础主题喵,由配置驱动喵。
//! 主题完全可自定义(后续支持 theme 配置文件)喵。

use crate::app::config::ThemeMode;
use gpui::{Hsla, rgb, rgba};
/// 应用内主题色板喵
#[derive(Debug, Clone, Copy)]
pub struct LauncherTheme {
    /// 浮窗背景喵
    pub bg: Hsla,
    /// 浮窗描边喵
    pub border: Hsla,
    /// 主文字喵
    pub text: Hsla,
    /// 次级文字(副标题/提示)喵
    pub text_dim: Hsla,
    /// 悬停/选中背景喵
    pub hover_bg: Hsla,
    /// 强调色(选中指示条)喵
    pub accent: Hsla,
    /// 强调色上的文字喵
    pub accent_text: Hsla,
    /// 搜索框背景喵(第二阶段接入自定义输入框样式)喵
    #[allow(dead_code)]
    pub input_bg: Hsla,
}

impl LauncherTheme {
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
            bg: rgba(0xF5F7FAFF).into(),
            border: rgba(0xD0D7E2FF).into(),
            text: rgb(0x1F2430).into(),
            text_dim: rgb(0x6B7280).into(),
            hover_bg: rgba(0xE8EDF5FF).into(),
            accent: rgb(0x6C5CE7).into(),
            accent_text: rgb(0xFFFFFF).into(),
            input_bg: rgba(0xFFFFFFFF).into(),
        }
    }

    /// 深色主题喵~ 深夜护眼喵!
    pub fn dark() -> Self {
        Self {
            bg: rgba(0x14161BFF).into(),
            border: rgba(0x2A2F3AFF).into(),
            text: rgb(0xE8EAED).into(),
            text_dim: rgb(0x8B93A1).into(),
            hover_bg: rgba(0x232833FF).into(),
            accent: rgb(0x8B7CF6).into(),
            accent_text: rgb(0x14161B).into(),
            input_bg: rgba(0x1B1F27FF).into(),
        }
    }
}
