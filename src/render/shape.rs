//! 形状工具喵~
//!
//! 灵动岛圆角走 Skia `RRect`(真圆弧);半径钳到短边一半时即胶囊喵。

use skia_safe::{Path, RRect, Rect};

/// 生成圆角矩形路径,圆角为真圆弧喵
///
/// 圆角半径自动钳制到不超过矩形短边的一半;等于短边一半时即「胶囊」喵。
pub fn rounded_rect_path(rect: Rect, radius: f32) -> Path {
    let r = radius.clamp(0.0, rect.width().min(rect.height()) / 2.0);
    if r <= 0.5 {
        return Path::rect(rect, None);
    }
    Path::rrect(RRect::new_rect_xy(rect, r, r), None)
}
