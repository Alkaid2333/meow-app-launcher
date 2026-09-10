//! 形状工具(真圆弧圆角 / 胶囊)喵~ 单元测试喵

use meow_app_launcher::render::shape::rounded_rect_path;
use skia_safe::Rect;

fn capsule(rect: Rect) -> skia_safe::Path {
    rounded_rect_path(rect, rect.height() / 2.0)
}

#[test]
fn capsule_is_closed() {
    let path = capsule(Rect::from_xywh(0.0, 0.0, 420.0, 44.0));
    assert!(!path.is_empty());
    assert!(path.is_last_contour_closed());
}

#[test]
fn radius_is_clamped() {
    let path = rounded_rect_path(Rect::from_xywh(0.0, 0.0, 100.0, 40.0), 999.0);
    assert!(!path.is_empty());
}

#[test]
fn tiny_radius_falls_back_to_rect() {
    let path = rounded_rect_path(Rect::from_xywh(0.0, 0.0, 100.0, 50.0), 0.0);
    assert_eq!(path.count_points(), 4);
}

#[test]
fn path_bounds_match_rect() {
    let rect = Rect::from_xywh(10.0, 20.0, 200.0, 60.0);
    let b = *rounded_rect_path(rect, 30.0).bounds();
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

#[test]
fn capsule_bounds_match_rect() {
    let rect = Rect::from_xywh(0.0, 0.0, 300.0, 46.0);
    let b = *capsule(rect).bounds();
    assert!((b.width() - rect.width()).abs() < 0.5);
    assert!((b.height() - rect.height()).abs() < 0.5);
}
