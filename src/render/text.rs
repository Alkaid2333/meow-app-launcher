//! 文本绘制辅助喵~
//!
//! 封装文字基线对齐、居中、裁剪等常见文本绘制逻辑喵。

use skia_safe::{Canvas, Font, Paint, Rect};

/// 计算垂直居中的基线 y 坐标喵
fn baseline_y(rect: &Rect, font: &Font) -> f32 {
    // metrics 返回 (max_width, FontMetrics),后者含 ascent/descent 喵
    let (_, m) = font.metrics();
    // ascent 为负(向上),descent 为正(向下),基线居中使可视区域垂直居中喵
    rect.center_y() - (m.ascent + m.descent) / 2.0
}

/// 在矩形内水平+垂直居中绘制文字喵
pub fn draw_centered(canvas: &Canvas, text: &str, rect: Rect, font: &Font, paint: &Paint) {
    let baseline = baseline_y(&rect, font);
    let (width, _) = font.measure_str(text, Some(paint));
    let x = rect.center_x() - width / 2.0;
    canvas.draw_str(text, (x, baseline), font, paint);
}

/// 在矩形内左对齐 + 垂直居中绘制文字,超出裁剪喵
///
/// 返回文字结束的 x 坐标(用于光标定位)喵。
pub fn draw_clipped(
    canvas: &Canvas,
    text: &str,
    rect: Rect,
    font: &Font,
    paint: &Paint,
) -> f32 {
    let baseline = baseline_y(&rect, font);
    let (width, _) = font.measure_str(text, Some(paint));

    canvas.save();
    canvas.clip_rect(rect, None, Some(false));
    canvas.draw_str(text, (rect.left, baseline), font, paint);
    canvas.restore();

    rect.left + width
}
