//! 形状工具(真圆弧圆角 / 胶囊)喵~ 单元测试喵

use meow_app_launcher::render::shape::{capsule_path, rounded_rect_path};
use skia_safe::Rect;

/// 胶囊应闭合,顶点数 = 1 Move + 4 Line + 4 Conic(各 2 点)喵
#[test]
fn capsule_is_closed() {
    let rect = Rect::from_xywh(0.0, 0.0, 420.0, 44.0);
    let path = capsule_path(rect);
    assert!(!path.is_empty());
    assert_eq!(path.count_points(), 1 + 4 + 4 * 2);
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

/// 半径 = 短边一半时,顶底边应保持平直(胶囊而非橄榄形)喵
#[test]
fn capsule_has_straight_edges() {
    let rect = Rect::from_xywh(0.0, 0.0, 300.0, 46.0);
    let path = capsule_path(rect);
    let pts = path.points();
    // Move 起点 = 顶边左端;第一条 LineTo 终点 = 顶边右端,两者 y 应相等喵
    let (x0, y0) = (pts[0].x, pts[0].y);
    let (x1, y1) = (pts[1].x, pts[1].y);
    assert!((y0 - y1).abs() < 0.01, "顶边应水平喵 y0={y0} y1={y1}");
    assert!(x1 > x0, "顶边应从左向右延伸喵");
    // 顶边平直段的水平长度 = width - 2r·(顶边实际起止在 r 处)喵
    assert!(
        (x1 - x0 - (rect.width() - rect.height())).abs() < 0.01,
        "平直段长度不符"
    );
}
