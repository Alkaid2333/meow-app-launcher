//! 形状工具喵~
//!
//! 提供灵动岛的核心几何: 连续曲率圆角(超椭圆近似)喵。
//! 普通圆角(简单圆弧)在直线与圆弧衔接处曲率跳变,视觉上「圆得生硬」;
//! 连续曲率圆角从直线平滑过渡到圆弧,iOS 灵动岛质感即源于此喵。
//!
//! 这里用超椭圆(superellipse)近似连续曲率圆角: 指数越高角越「方中带圆」,
//! n=4 时观感接近 iOS 图标/灵动岛的连续曲率圆角喵。

use skia_safe::{Path, PathBuilder, Rect};
use std::f32::consts::FRAC_PI_2;

/// 超椭圆指数喵(越大角越方,4 为灵动岛推荐值)喵
const N_EXP: f32 = 4.0;
/// 每个圆角的采样段数喵(越多越平滑)喵
const CORNER_STEPS: usize = 16;

/// 生成圆角矩形路径,圆角采用连续曲率(超椭圆)喵
///
/// 圆角半径自动钳制到不超过矩形短边的一半,避免畸变喵。
pub fn rounded_rect_path(rect: Rect, radius: f32) -> Path {
    let r = radius.clamp(0.0, rect.width().min(rect.height()) / 2.0);

    // 圆角过小直接退化为普通矩形喵
    if r <= 0.5 {
        return Path::rect(rect, None);
    }

    let left = rect.left;
    let top = rect.top;
    let right = rect.right;
    let bottom = rect.bottom;

    // 超椭圆采样: s = sin(θ)^(2/n), c = cos(θ)^(2/n),θ 从 0 扫到 π/2 喵。
    // 注: f32 的 FRAC_PI_2 略大于精确 π/2,cos 可能返回微小负值,
    // 开方负数会得 NaN,故钳制到非负喵。
    let sc = |theta: f32| -> (f32, f32) {
        let c = theta.cos().max(0.0).powf(2.0 / N_EXP);
        let s = theta.sin().max(0.0).powf(2.0 / N_EXP);
        (s, c)
    };

    let mut pb = PathBuilder::new();
    // 起点: 顶边右端,之后按顺时针描边(顶→右→底→左)喵
    pb.move_to((right - r, top));

    // 右上角: 顶部 → 右侧(顺时针)喵
    corner(&mut pb, |t| {
        let (s, c) = sc(t * FRAC_PI_2);
        (right - r + r * s, top + r - r * c)
    });
    // 右边直线喵
    pb.line_to((right, bottom - r));
    // 右下角: 右侧 → 底部(顺时针)喵
    corner(&mut pb, |t| {
        let (s, c) = sc(t * FRAC_PI_2);
        (right - r + r * c, bottom - r + r * s)
    });
    // 底边直线喵
    pb.line_to((left + r, bottom));
    // 左下角: 底部 → 左侧(顺时针)喵
    corner(&mut pb, |t| {
        let (s, c) = sc(t * FRAC_PI_2);
        (left + r - r * s, bottom - r + r * c)
    });
    // 左边直线喵
    pb.line_to((left, top + r));
    // 左上角: 左侧 → 顶部(顺时针)喵
    corner(&mut pb, |t| {
        let (s, c) = sc(t * FRAC_PI_2);
        (left + r - r * c, top + r - r * s)
    });

    pb.close();
    pb.snapshot()
}

/// 胶囊路径喵: 圆角 = 高度一半,即两端半圆喵
pub fn capsule_path(rect: Rect) -> Path {
    rounded_rect_path(rect, rect.height() / 2.0)
}

/// 沿一个角的连续曲率曲线采样描边喵
fn corner(pb: &mut PathBuilder, sample: impl Fn(f32) -> (f32, f32)) {
    for i in 1..=CORNER_STEPS {
        let t = i as f32 / CORNER_STEPS as f32;
        let (x, y) = sample(t);
        pb.line_to((x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 胶囊路径应闭合且非空喵
    #[test]
    fn capsule_is_closed() {
        let rect = Rect::from_xywh(0.0, 0.0, 420.0, 44.0);
        let path = capsule_path(rect);
        assert!(!path.is_empty());
        assert_eq!(path.count_points(), 4 * CORNER_STEPS + 4);
    }

    /// 圆角半径会被钳制到短边一半,避免畸变喵
    #[test]
    fn radius_is_clamped() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 40.0);
        let path = rounded_rect_path(rect, 999.0);
        // 钳制后仍能正常生成路径喵
        assert!(!path.is_empty());
    }

    /// 极小圆角退化为普通矩形(4 个角点)喵
    #[test]
    fn tiny_radius_falls_back_to_rect() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 50.0);
        let path = rounded_rect_path(rect, 0.0);
        assert_eq!(path.count_points(), 4);
    }

    /// 路径边界应贴合输入矩形,无跑出无缺口喵
    #[test]
    fn path_bounds_match_rect() {
        let rect = Rect::from_xywh(10.0, 20.0, 200.0, 60.0);
        let path = rounded_rect_path(rect, 30.0);
        let b = *path.bounds();
        assert!((b.left - rect.left).abs() < 0.5, "left={} 期望={}", b.left, rect.left);
        assert!((b.top - rect.top).abs() < 0.5, "top={} 期望={}", b.top, rect.top);
        assert!((b.right - rect.right).abs() < 0.5, "right={} 期望={}", b.right, rect.right);
        assert!(
            (b.bottom - rect.bottom).abs() < 0.5,
            "bottom={} 期望={}",
            b.bottom,
            rect.bottom
        );
    }
}
