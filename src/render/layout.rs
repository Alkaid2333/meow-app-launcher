//! 布局计算喵~
//!
//! 把灵动岛帧(逻辑像素) + DPI 缩放换算成物理像素布局喵。
//! 纯计算、无副作用,可单测喵。渲染层只消费这里的矩形结果喵。

use crate::animation::IslandFrame;
use crate::app::config::AppConfig;
use skia_safe::Rect;

/// 每条目高度(逻辑 px)喵
pub const ITEM_HEIGHT: f32 = 44.0;
/// 阴影安全边距(逻辑 px)喵
pub const SHADOW_MARGIN: f32 = 24.0;

/// 一次布局的完整结果喵(全部为物理像素)喵
#[derive(Debug, Clone)]
pub struct Layout {
    /// 窗口物理尺寸(含阴影边距)喵
    pub window_width: f32,
    /// 窗口物理高度喵
    pub window_height: f32,
    /// 搜索槽矩形(相对窗口)喵
    pub search_rect: Rect,
    /// 结果面板矩形(相对窗口)喵
    pub panel_rect: Rect,
    /// 岛体外轮廓(相对窗口)喵
    pub island_rect: Rect,
    /// 每条目的矩形(相对窗口)喵
    pub item_rects: Vec<Rect>,
    /// 图标显示尺寸(物理 px)喵
    pub icon_size: f32,
    /// 面板当前高度(物理 px)喵
    pub panel_height: f32,
    /// 岛圆角(物理 px)喵
    pub radius: f32,
    /// 面板透明度喵
    pub panel_opacity: f32,
    /// 滚动条(轨道, 滑块)喵,内容未溢出时为 None 喵
    pub scrollbar: Option<(Rect, Rect)>,
    /// 整体透明度喵
    pub opacity: f32,
    /// 是否碰到安全边界喵
    pub hit: bool,
}

impl Layout {
    /// 从岛帧计算布局喵
    pub fn from_frame(
        config: &AppConfig,
        scale: f32,
        frame: &IslandFrame,
        result_count: usize,
        scroll_offset: f32,
    ) -> Self {
        let s = scale.max(0.01);
        let m = SHADOW_MARGIN * s;
        let island = Rect::from_xywh(m, m, frame.width as f32 * s, frame.height as f32 * s);
        let slot = Rect::from_xywh(
            island.left + frame.slot.x as f32 * s,
            island.top + frame.slot.y as f32 * s,
            frame.slot.w as f32 * s,
            frame.slot.h as f32 * s,
        );
        let panel = Rect::from_xywh(
            island.left + frame.panel.x as f32 * s,
            island.top + (frame.panel.y + frame.panel_shift) as f32 * s,
            frame.panel.w as f32 * s,
            frame.panel.h as f32 * s,
        );

        let item_x = panel.left;
        let mut item_rects = Vec::with_capacity(result_count);
        for i in 0..result_count {
            let y = panel.top + (i as f32 * ITEM_HEIGHT - scroll_offset) * s;
            item_rects.push(Rect::from_xywh(item_x, y, panel.width(), ITEM_HEIGHT * s));
        }

        // 滚动条几何: 内容超出面板时才出现,滑块高度与位置按比例映射喵
        let scrollbar = if result_count > 0 && panel.height() > 8.0 {
            let content_h = result_count as f32 * ITEM_HEIGHT * s;
            if content_h > panel.height() + 0.5 {
                let track = Rect::from_xywh(
                    panel.right - 8.0,
                    panel.top + 2.0,
                    4.0,
                    panel.height() - 4.0,
                );
                let thumb_h = (panel.height() * panel.height() / content_h).max(24.0);
                let travel = panel.height() - 4.0 - thumb_h;
                let max_scroll = content_h - panel.height();
                let prog = (scroll_offset * s / max_scroll).clamp(0.0, 1.0);
                let thumb = Rect::from_xywh(
                    track.left,
                    track.top + travel * prog,
                    track.width(),
                    thumb_h,
                );
                Some((track, thumb))
            } else {
                None
            }
        } else {
            None
        };

        Self {
            window_width: island.width() + m * 2.0,
            window_height: island.height() + m * 2.0,
            search_rect: slot,
            panel_rect: panel,
            island_rect: island,
            item_rects,
            icon_size: config.window.icon_size * s,
            panel_height: panel.height(),
            radius: frame.radius as f32 * s,
            panel_opacity: frame.panel_opacity as f32,
            scrollbar,
            opacity: frame.opacity as f32,
            hit: frame.hit_bottom || frame.hit_x,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{DynamicIsland, IslandConfig};

    fn sample_frame(expanded: bool, count: usize) -> (AppConfig, IslandFrame) {
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
        let _ = count;
        (cfg, island.frame())
    }

    #[test]
    fn empty_layout_has_search_slot() {
        let (cfg, frame) = sample_frame(false, 0);
        let l = Layout::from_frame(&cfg, 1.0, &frame, 0, 0.0);
        assert!(l.search_rect.height() > 0.0);
        assert!(l.island_rect.width() > 0.0);
    }

    #[test]
    fn expanded_layout_has_items() {
        let (cfg, frame) = sample_frame(true, 5);
        let l = Layout::from_frame(&cfg, 1.0, &frame, 5, 0.0);
        assert_eq!(l.item_rects.len(), 5);
        assert!(l.panel_rect.height() > 0.0);
    }

    #[test]
    fn scroll_offset_shifts_items() {
        let (cfg, frame) = sample_frame(true, 10);
        let l0 = Layout::from_frame(&cfg, 1.0, &frame, 10, 0.0);
        let l1 = Layout::from_frame(&cfg, 1.0, &frame, 10, ITEM_HEIGHT);
        let dy = (l0.item_rects[0].top - l1.item_rects[0].top).abs();
        assert!((dy - ITEM_HEIGHT).abs() < 0.5, "dy={dy}");
    }

    #[test]
    fn scale_factor_scales_proportionally() {
        let (cfg, frame) = sample_frame(true, 3);
        let l1 = Layout::from_frame(&cfg, 1.0, &frame, 3, 0.0);
        let l2 = Layout::from_frame(&cfg, 2.0, &frame, 3, 0.0);
        assert!((l2.window_width - l1.window_width * 2.0).abs() < 0.5);
        assert!((l2.search_rect.width() - l1.search_rect.width() * 2.0).abs() < 0.5);
    }
}
