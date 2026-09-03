//! 布局计算(列表/网格)喵~ 单元测试喵

use meow_app_launcher::animation::{DynamicIsland, IslandConfig};
use meow_app_launcher::app::config::{AppConfig, AppLayout};
use meow_app_launcher::render::layout::{
    grid_horizontal_target, grid_rects, grid_vertical_target, Layout, ITEM_HEIGHT,
};
use skia_safe::Rect;

fn sample_frame(expanded: bool, count: usize) -> (AppConfig, meow_app_launcher::animation::IslandFrame, Vec<bool>) {
    let mut cfg = AppConfig::default();
    cfg.island = IslandConfig::default();
    let mut island = DynamicIsland::new(cfg.island.clone());
    island.set_stage(1280.0, 720.0);
    island.go(true, expanded);
    for _ in 0..400 {
        island.step(1.0 / 60.0);
        if island.settled() {
            break;
        }
    }
    (cfg, island.frame(), vec![false; count])
}

#[test]
fn empty_layout_has_search_slot() {
    let (cfg, frame, sections) = sample_frame(false, 0);
    let l = Layout::from_frame(&cfg, 1.0, &frame, &sections, 0.0);
    assert!(l.search_rect.height() > 0.0);
    assert!(l.island_rect.width() > 0.0);
}

#[test]
fn expanded_layout_has_items() {
    let (cfg, frame, sections) = sample_frame(true, 5);
    let l = Layout::from_frame(&cfg, 1.0, &frame, &sections, 0.0);
    assert_eq!(l.item_rects.len(), 5);
    assert!(l.panel_rect.height() > 0.0);
}

#[test]
fn scroll_offset_shifts_items() {
    let (cfg, frame, sections) = sample_frame(true, 10);
    let l0 = Layout::from_frame(&cfg, 1.0, &frame, &sections, 0.0);
    let l1 = Layout::from_frame(&cfg, 1.0, &frame, &sections, ITEM_HEIGHT);
    let dy = (l0.item_rects[0].top - l1.item_rects[0].top).abs();
    assert!((dy - ITEM_HEIGHT).abs() < 0.5, "dy={dy}");
}

#[test]
fn scale_factor_scales_proportionally() {
    let (cfg, frame, sections) = sample_frame(true, 3);
    let l1 = Layout::from_frame(&cfg, 1.0, &frame, &sections, 0.0);
    let l2 = Layout::from_frame(&cfg, 2.0, &frame, &sections, 0.0);
    assert!((l2.window_width - l1.window_width * 2.0).abs() < 0.5);
    assert!((l2.search_rect.width() - l1.search_rect.width() * 2.0).abs() < 0.5);
}

#[test]
fn 网格排版自动分行() {
    let (mut cfg, frame, sections) = sample_frame(true, 12);
    cfg.window.layout = AppLayout::Grid;
    let l = Layout::from_frame(&cfg, 1.0, &frame, &sections, 0.0);
    assert_eq!(l.item_rects.len(), 12);
    // 面板宽约 524,单元格最小 84+gap → 至少 5 列;12 个应用应至少占 3 行喵
    let rows = l
        .item_rects
        .windows(2)
        .filter(|w| w[0].top < w[1].top - 1.0)
        .count()
        + 1;
    assert!(rows >= 3, "rows={rows}");
    // 同一行两块等高且不相叠喵
    assert_eq!(l.item_rects[0].height(), l.item_rects[1].height());
    assert!(l.item_rects[0].right <= l.item_rects[1].left + 0.5);
}

#[test]
fn 网格分组头占整行() {
    let (mut cfg, frame, _) = sample_frame(true, 1);
    cfg.window.layout = AppLayout::Grid;
    let sections = vec![true, false, false];
    let l = Layout::from_frame(&cfg, 1.0, &frame, &sections, 0.0);
    assert_eq!(l.item_rects.len(), 3);
    assert!(l.item_rects[0].width() > l.item_rects[1].width(), "分组头应占整行");
    // 应用单元格在分组头下方喵
    assert!(l.item_rects[1].top >= l.item_rects[0].bottom);
}

#[test]
fn 列表内容超过面板时出现滚动条() {
    let (cfg, frame, sections) = sample_frame(true, 30);
    let l = Layout::from_frame(&cfg, 1.0, &frame, &sections, 0.0);
    assert!(l.scrollbar.is_some(), "内容溢出应有滚动条");
    assert!(l.content_height > l.panel_rect.height());
    // 列表条目右侧应留出滚动条安全槽喵
    assert!(
        l.item_rects[0].right <= l.panel_rect.right - 10.0,
        "条目应与滚动条保持安全距离"
    );
}

#[test]
fn 网格内容高度不随滚动缩水() {
    // 网格模式下 content_height 必须与滚动量解耦,否则最大滚动量/滑块会抖动回弹喵
    let (mut cfg, frame, sections) = sample_frame(true, 40);
    cfg.window.layout = AppLayout::Grid;
    let h0 = Layout::from_frame(&cfg, 1.0, &frame, &sections, 0.0).content_height;
    let h_scrolled = Layout::from_frame(&cfg, 1.0, &frame, &sections, 120.0).content_height;
    assert_eq!(h0, h_scrolled, "内容高度不应随滚动偏移变化喵");
    assert!(h_scrolled > 0.0);
}

#[test]
fn 网格垂直导航同列移动() {
    // 分组头占整行导致行不齐,垂直导航应保持在同列(水平中心最近)喵
    // 面板宽 500 → (500-2)/96 ≈ 5 列,行结构可预判喵
    let panel = Rect::from_xywh(0.0, 0.0, 500.0, 400.0);
    let sections: Vec<bool> = [vec![false; 10], vec![true], vec![false; 12]].concat();
    let rects = grid_rects(&panel, &sections, 0.0, 1.0);
    // 第一行第 3 个应用(索引 3),向下的目标应是第二行同列(索引 8)喵
    assert_eq!(grid_vertical_target(&rects, &sections, 3, 1), Some(8));
    // 组 B 首行(索引 11)向下的同列目标 = 索引 16 喵
    assert_eq!(grid_vertical_target(&rects, &sections, 11, 1), Some(16));
    // 最末行原地不动(下方无行)喵
    assert_eq!(grid_vertical_target(&rects, &sections, 22, 1), None);
    // 向上恢复: 索引 16 向上 → 同列上一行(索引 11)喵
    assert_eq!(grid_vertical_target(&rects, &sections, 16, -1), Some(11));
}

#[test]
fn 网格横向导航不跨组跳行() {
    let panel = Rect::from_xywh(0.0, 0.0, 500.0, 400.0);
    let sections: Vec<bool> = [vec![false; 5], vec![true], vec![false; 8]].concat();
    let rects = grid_rects(&panel, &sections, 0.0, 1.0);
    // 行内相邻喵
    assert_eq!(grid_horizontal_target(&rects, &sections, 0, 1), Some(1));
    // 行首/行尾原地不动,避免焦点跨到别的分组喵
    assert_eq!(grid_horizontal_target(&rects, &sections, 0, -1), None);
    assert_eq!(grid_horizontal_target(&rects, &sections, 4, 1), None);
}
