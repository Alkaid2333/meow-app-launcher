//! 布局计算喵~
//!
//! 把「逻辑像素基准 + DPI 缩放 + 弹簧进度」换算成物理像素布局喵。
//! 纯计算、无副作用,可单测喵。渲染层只消费这里的矩形结果,不自己算位置喵。

use crate::app::config::AppConfig;
use skia_safe::Rect;

/// 搜索框宽度(逻辑 px)喵
pub const BAR_WIDTH: f32 = 420.0;
/// 搜索框高度(逻辑 px)喵
pub const BAR_HEIGHT: f32 = 44.0;
/// 搜索框与面板的垂直间距(逻辑 px)喵
pub const GAP: f32 = 10.0;
/// 面板内边距(逻辑 px)喵
pub const PANEL_PADDING: f32 = 8.0;
/// 每条目高度(逻辑 px)喵
pub const ITEM_HEIGHT: f32 = 44.0;
/// 阴影安全边距(逻辑 px,为阴影预留空间)喵
pub const SHADOW_MARGIN: f32 = 16.0;

/// 一次布局的完整结果喵(全部为物理像素)喵
#[derive(Debug, Clone)]
pub struct Layout {
    /// 窗口物理尺寸(含阴影边距)喵
    pub window_width: f32,
    /// 窗口物理高度喵
    pub window_height: f32,
    /// 搜索框胶囊矩形(相对窗口)喵
    pub search_rect: Rect,
    /// 结果面板矩形(相对窗口)喵
    pub panel_rect: Rect,
    /// 每条目的矩形(相对窗口)喵
    pub item_rects: Vec<Rect>,
    /// 图标显示尺寸(物理 px)喵
    pub icon_size: f32,
    /// 面板当前高度(物理 px,由弹簧驱动)喵
    pub panel_height: f32,
}

impl Layout {
    /// 计算布局喵
    ///
    /// * `config` - 配置(面板最大高度、图标大小)喵
    /// * `scale` - DPI 缩放系数喵
    /// * `result_count` - 结果数量喵
    /// * `panel_progress` - 面板展开进度 0..1(弹簧值)喵
    /// * `scroll_offset` - 面板滚动偏移(逻辑像素)喵
    pub fn compute(
        config: &AppConfig,
        scale: f32,
        result_count: usize,
        panel_progress: f32,
        scroll_offset: f32,
    ) -> Self {
        // 面板满展开时的逻辑高度喵(无结果时高度为 0)喵
        let content_h = if result_count == 0 {
            0.0
        } else {
            result_count as f32 * ITEM_HEIGHT + PANEL_PADDING * 2.0
        };
        let panel_full = content_h.min(config.window.height.max(80.0));
        let panel_h = panel_full * panel_progress;

        // 逻辑窗口尺寸喵
        let win_w = BAR_WIDTH + SHADOW_MARGIN * 2.0;
        let win_h = SHADOW_MARGIN + BAR_HEIGHT + GAP + panel_h + SHADOW_MARGIN;

        let s = scale;
        let m = SHADOW_MARGIN * s;

        // 搜索框矩形(物理)喵
        let search_rect = Rect::from_xywh(m, m, BAR_WIDTH * s, BAR_HEIGHT * s);

        // 面板矩形(物理)喵
        let panel_top = m + BAR_HEIGHT * s + GAP * s;
        let panel_rect = Rect::from_xywh(m, panel_top, BAR_WIDTH * s, panel_h * s);

        // 条目矩形(物理)喵,纵向减去滚动偏移喵
        let item_x = m + PANEL_PADDING * s;
        let mut item_rects = Vec::with_capacity(result_count);
        for i in 0..result_count {
            let y = panel_top + (PANEL_PADDING + i as f32 * ITEM_HEIGHT - scroll_offset) * s;
            item_rects.push(Rect::from_xywh(
                item_x,
                y,
                (BAR_WIDTH - PANEL_PADDING * 2.0) * s,
                ITEM_HEIGHT * s,
            ));
        }

        Self {
            window_width: win_w * s,
            window_height: win_h * s,
            search_rect,
            panel_rect,
            item_rects,
            icon_size: config.window.icon_size * s,
            panel_height: panel_h * s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::config::AppConfig;

    /// 无结果时面板高度为 0,窗口只含搜索框 + 边距喵
    #[test]
    fn empty_layout_collapses_panel() {
        let cfg = AppConfig::default();
        let l = Layout::compute(&cfg, 1.0, 0, 1.0, 0.0);
        assert_eq!(l.panel_height, 0.0);
        assert!(l.window_height <= SHADOW_MARGIN * 2.0 + BAR_HEIGHT + GAP);
    }

    /// 有结果且进度为 1 时面板满展开喵
    #[test]
    fn full_layout_expands_panel() {
        let cfg = AppConfig::default();
        let l = Layout::compute(&cfg, 1.0, 5, 1.0, 0.0);
        assert!(l.panel_height > 0.0);
        assert_eq!(l.item_rects.len(), 5);
    }

    /// 滚动偏移应纵向平移条目喵
    #[test]
    fn scroll_offset_shifts_items() {
        let cfg = AppConfig::default();
        let l0 = Layout::compute(&cfg, 1.0, 10, 1.0, 0.0);
        let l1 = Layout::compute(&cfg, 1.0, 10, 1.0, ITEM_HEIGHT);
        // 滚过一个条目高度,第一个条目应上移一个 ITEM_HEIGHT 喵
        let dy = (l0.item_rects[0].top - l1.item_rects[0].top).abs();
        assert!((dy - ITEM_HEIGHT).abs() < 0.5, "dy={dy}");
    }

    /// DPI 缩放应等比放大喵
    #[test]
    fn scale_factor_scales_proportionally() {
        let cfg = AppConfig::default();
        let l1 = Layout::compute(&cfg, 1.0, 3, 1.0, 0.0);
        let l2 = Layout::compute(&cfg, 2.0, 3, 1.0, 0.0);
        assert!((l2.window_width - l1.window_width * 2.0).abs() < 0.5);
        assert!((l2.search_rect.width() - l1.search_rect.width() * 2.0).abs() < 0.5);
    }
}
