//! 统一图标渲染层喵~
//!
//! 一切可渲染图标(SVG 线稿 / PNG·ICO 光栅 / 字符兜底)都从这里出去,
//! 统一「等比缩放 + 居中 + 标准内边距」,视觉尺寸全局一致喵。
//! 新增图标形态时只需扩展 [`IconSource`],绘制规则不用再各写一份喵。

use super::font::FontCache;
use super::{shape, svg, text};
use crate::render::theme::Theme;
use skia_safe::{Canvas, Color, Paint, Rect};

/// 图标标准内边距比例喵(四周各收 8%,让线稿与满幅光标视觉等大)喵
pub const ICON_INSET_RATIO: f32 = 0.08;

/// 统一图标源喵
pub enum IconSource {
    /// 内置 SVG 线稿(svg.rs 内嵌素材名)喵
    Builtin(&'static str),
    /// 已解码光栅图(应用图标 png 快照等;Image 为引用计数,持有廉价)喵
    Raster(skia_safe::Image),
    /// 字符兜底喵(无图标时的首字符底牌)
    Fallback(String),
}

/// 对目标矩形做标准内边距喵
pub fn inset(rect: Rect) -> Rect {
    let dx = rect.width() * ICON_INSET_RATIO;
    let dy = rect.height() * ICON_INSET_RATIO;
    Rect::from_xywh(
        rect.left + dx,
        rect.top + dy,
        rect.width() - dx * 2.0,
        rect.height() - dy * 2.0,
    )
}

/// 统一图标绘制入口喵: 内边距 + 按源分发喵
pub fn draw(
    canvas: &Canvas,
    theme: &Theme,
    rect: Rect,
    source: IconSource,
    fonts: &FontCache,
) {
    let inner = inset(rect);
    match source {
        IconSource::Builtin(name) => svg::icon(name).draw(canvas, inner, theme.accent),
        IconSource::Raster(img) => draw_raster(canvas, &img, inner),
        IconSource::Fallback(label) => draw_fallback(canvas, theme, inner, &label, fonts),
    }
}

/// 内置 SVG 直绘喵(自带颜色,统一内边距;给非强调色场景用)喵
pub fn draw_builtin(canvas: &Canvas, rect: Rect, name: &str, color: Color) {
    svg::icon(name).draw(canvas, inset(rect), color);
}

/// 光栅图标喵: 等比铺满内缩矩形喵
fn draw_raster(canvas: &Canvas, img: &skia_safe::Image, rect: Rect) {
    let mut p = Paint::default();
    p.set_anti_alias(true);
    canvas.draw_image_rect(img, None, rect, &p);
}

/// 字符兜底图标喵: 强调色圆角底 + 标题首字符喵
fn draw_fallback(canvas: &Canvas, theme: &Theme, rect: Rect, label: &str, fonts: &FontCache) {
    let path = shape::rounded_rect_path(rect, rect.width() * 0.22);
    let mut bg = Paint::default();
    bg.set_color(theme.accent);
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);

    let label = if label.is_empty() { "?".to_string() } else { label.to_string() };
    let font = fonts.font(rect.width() * 0.38);
    let mut p = Paint::default();
    p.set_color(theme.accent_text);
    p.set_anti_alias(true);
    text::draw_centered(canvas, &label, rect, &font, &p);
}
