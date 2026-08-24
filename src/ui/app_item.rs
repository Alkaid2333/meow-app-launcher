//! 应用条目组件喵~
//!
//! 显示一个应用: 图标(或字符合成兜底)+ 名称 + 路径/标签喵。
//! 支持选中态高亮喵。

use crate::apps::AppInfo;
use crate::ui::LauncherTheme;
use gpui::{AnyElement, InteractiveElement, div, img, px, rgb, IntoElement, ParentElement, Styled, SharedString};
use std::path::PathBuf;

/// 渲染一个应用条目喵
///
/// * `app` - 应用信息喵
/// * `icon_path` - 图标快照路径(无则字符合成)喵
/// * `selected` - 是否选中喵
/// * `theme` - 主题喵
/// * `icon_size` - 图标尺寸(px)喵
pub fn app_item(
    app: &AppInfo,
    icon_path: Option<PathBuf>,
    selected: bool,
    theme: &LauncherTheme,
    icon_size: f32,
) -> AnyElement {
    let name = app.name.clone();
    let path = app.path.clone();

    let icon = match icon_path {
        Some(p) => img(p).size(px(icon_size)).h(px(icon_size)).into_any_element(),
        None => fallback_icon(&name, icon_size, theme).into_any_element(),
    };

    let bg = if selected { theme.hover_bg } else { theme.bg };
    // 左侧选中指示条喵
    let accent_bar = if selected {
        div()
            .id(SharedString::from(format!("accent-{name}")))
            .w(px(3.0))
            .h(px(icon_size * 0.7))
            .rounded(px(1.5))
            .bg(theme.accent)
            .into_any_element()
    } else {
        div()
            .id(SharedString::from(format!("accent-{name}-off")))
            .w(px(3.0))
            .into_any_element()
    };

    // 名称 + 路径两行布局喵
    let text_col = div()
        .flex()
        .flex_col()
        .justify_center()
        .min_w_0()
        .child(
            div()
                .text_color(if selected { theme.text } else { theme.text })
                .child(name.clone()),
        )
        .child(
            div()
                .mt_0p5()
                .text_color(theme.text_dim)
                .text_size(px(11.0))
                .max_w(px(420.0))
                .overflow_x_hidden()
                .whitespace_nowrap()
                .child(path),
        )
        .into_any_element();

    div()
        .id(SharedString::from(format!("app-{name}")))
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(8.0))
        .bg(bg)
        .child(accent_bar)
        .child(icon)
        .child(text_col)
        .into_any_element()
}

/// 字符合成兜底图标喵: 品牌色背景 + 应用名前两个字符喵
fn fallback_icon(name: &str, size: f32, theme: &LauncherTheme) -> impl IntoElement {
    // 取前两个 UTF-8 字符喵
    let chars: Vec<char> = name.chars().take(2).collect();
    let text: String = if chars.is_empty() {
        "?".to_string()
    } else {
        chars.iter().collect()
    };

    div()
        .size(px(size))
        .h(px(size))
        .rounded(px(size * 0.22))
        .bg(theme.accent)
        .flex()
        .items_center()
        .justify_center()
        .text_color(theme.accent_text)
        .text_size(px(size * 0.4))
        .font_weight(gpui::FontWeight::BOLD)
        .child(text)
}

// 小工具: 让 rgb 颜色可读一点喵
#[allow(dead_code)]
fn _color_hex(hex: u32) -> gpui::Rgba {
    rgb(hex)
}
