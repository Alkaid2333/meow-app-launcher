//! 形状工具喵~
//!
//! 提供灵动岛的核心几何: 真圆弧圆角喵。
//! 每个角用一条圆锥曲线(conic)精确绘制四分之一圆弧:
//! * 圆角 = 高度一半时 → 标准胶囊(两端半圆 + 平直顶底边),观感贴合原型喵
//! * 相比折线/超椭圆采样,圆弧无锯齿、运动缩放不抖动、处处曲率连续喵

use skia_safe::{Path, PathBuilder, Rect};

/// 四分之一圆弧的圆锥曲线权重(cos 45° ≈ 0.7071,Skia 圆角标准值)喵
const QUARTER_ARC_K: f32 = 0.707_106_8;

/// 生成圆角矩形路径,圆角为真圆弧喵
///
/// 圆角半径自动钳制到不超过矩形短边的一半;等于短边一半时即「胶囊」喵。
pub fn rounded_rect_path(rect: Rect, radius: f32) -> Path {
    let r = radius.clamp(0.0, rect.width().min(rect.height()) / 2.0);

    // 圆角过小直接退化为普通矩形喵
    if r <= 0.5 {
        return Path::rect(rect, None);
    }

    let l = rect.left;
    let t = rect.top;
    let right = rect.right;
    let b = rect.bottom;

    let mut pb = PathBuilder::new();
    // 从顶边左端起,顺时针描边(顶 → 右 → 底 → 左)喵
    pb.move_to((l + r, t));
    pb.line_to((right - r, t));
    // 右上角: 四分之一圆弧喵
    pb.conic_to((right, t), (right, t + r), QUARTER_ARC_K);
    pb.line_to((right, b - r));
    // 右下角喵
    pb.conic_to((right, b), (right - r, b), QUARTER_ARC_K);
    pb.line_to((l + r, b));
    // 左下角喵
    pb.conic_to((l, b), (l, b - r), QUARTER_ARC_K);
    pb.line_to((l, t + r));
    // 左上角喵
    pb.conic_to((l, t), (l + r, t), QUARTER_ARC_K);
    pb.close();
    pb.snapshot()
}

/// 胶囊路径喵: 圆角 = 高度一半,即两端半圆喵(公开便于单测)喵
pub fn capsule_path(rect: Rect) -> Path {
    rounded_rect_path(rect, rect.height() / 2.0)
}

