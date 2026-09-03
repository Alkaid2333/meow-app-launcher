//! 布局计算喵~
//!
//! 把灵动岛帧(逻辑像素) + DPI 缩放换算成物理像素布局喵。
//! 支持「列表 / 网格」双排版: 列表为整宽行,网格自动按面板宽度计算列数喵。
//! 右侧预留滚动条安全槽,条目内容不与滚动条重叠喵。
//! 纯计算、无副作用,可单测喵。渲染层只消费这里的矩形结果喵。

use crate::animation::IslandFrame;
use crate::app::config::{AppConfig, AppLayout};
use skia_safe::Rect;

/// 每条目高度(逻辑 px)喵
pub const ITEM_HEIGHT: f32 = 44.0;
/// 阴影安全边距(逻辑 px)喵
pub const SHADOW_MARGIN: f32 = 24.0;
/// 网格单元格最小宽度(逻辑 px)喵
const GRID_MIN_CELL_W: f32 = 84.0;
/// 网格单元格高度(逻辑 px)喵
const GRID_CELL_H: f32 = 84.0;
/// 网格间距(逻辑 px)喵
const GRID_GAP: f32 = 12.0;
/// 网格分组头高度(逻辑 px)喵
const GRID_SECTION_H: f32 = 28.0;
/// 网格单元格安全边距(逻辑 px): 首行/首列内缩,聚焦框描边不贴面板边缘喵
const GRID_CELL_INSET: f32 = 3.0;
/// 列表模式右侧滚动条安全槽(物理 px)喵
const LIST_GUTTER_PX: f32 = 14.0;

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
    /// 每条目的矩形(相对窗口,与条目索引对齐)喵
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
    /// 内容总高度(物理 px,自面板顶起)喵
    pub content_height: f32,
    /// 整体透明度喵
    pub opacity: f32,
    /// 是否碰到安全边界喵
    pub hit: bool,
}

impl Layout {
    /// 从岛帧计算布局喵
    ///
    /// `sections[i]` 标记第 i 条为分组头(网格下占整行)喵。
    /// `scroll_offset` 为逻辑像素滚动量喵。
    pub fn from_frame(
        config: &AppConfig,
        scale: f32,
        frame: &IslandFrame,
        sections: &[bool],
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

        let grid = config.window.layout == AppLayout::Grid;
        let item_rects = if grid {
            grid_rects(&panel, sections, scroll_offset, s)
        } else {
            list_rects(&panel, sections.len(), scroll_offset, s)
        };

        // 内容总高度: 列表 = 行数×行高;网格 = 末行底部 - 面板顶 + 滚动回填喵
        // 注意: item_rects 已按 scroll_offset 上移,必须把滚动量加回来,
        // 否则内容高度随滚动「缩水」,最大滚动量/滚动条滑块会抖动回弹喵。
        let content_height = if grid {
            item_rects
                .last()
                .map(|r| r.bottom - panel.top + scroll_offset * s)
                .unwrap_or(0.0)
        } else {
            sections.len() as f32 * ITEM_HEIGHT * s
        };

        // 滚动条几何: 内容超出面板时才出现,滑块高度与位置按比例映射喵
        let scrollbar = if content_height > panel.height() + 0.5 {
            let track = Rect::from_xywh(
                panel.right - 8.0,
                panel.top + 2.0,
                4.0,
                panel.height() - 4.0,
            );
            let thumb_h = (panel.height() * panel.height() / content_height).max(24.0);
            let travel = track.height() - thumb_h;
            let max_scroll = content_height - panel.height();
            let prog = ((scroll_offset * s) / max_scroll).clamp(0.0, 1.0);
            let thumb = Rect::from_xywh(
                track.left,
                track.top + travel * prog,
                track.width(),
                thumb_h,
            );
            Some((track, thumb))
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
            content_height,
            opacity: frame.opacity as f32,
            hit: frame.hit_bottom || frame.hit_x,
        }
    }
}

/// 列表排版: 整宽行(右侧留滚动条安全槽),按滚动偏移排列喵
fn list_rects(panel: &Rect, count: usize, scroll_offset: f32, s: f32) -> Vec<Rect> {
    let mut rects = Vec::with_capacity(count);
    for i in 0..count {
        let y = panel.top + (i as f32 * ITEM_HEIGHT - scroll_offset) * s;
        rects.push(Rect::from_xywh(
            panel.left,
            y,
            (panel.width() - LIST_GUTTER_PX).max(8.0),
            ITEM_HEIGHT * s,
        ));
    }
    rects
}

/// 网格模式列数(输入面板逻辑宽度),供布局与键盘导航共用喵
pub fn grid_cols(panel_logical_width: f32) -> usize {
    let gap = GRID_GAP;
    let cell = GRID_MIN_CELL_W;
    let item_w = (panel_logical_width - LIST_GUTTER_PX).max(cell);
    (((item_w + gap) / (cell + gap)).floor() as usize).max(1)
}

/// 网格排版: 分组头占整行,应用按 `min 单元格宽` 自动算列数,行满换行喵
///
/// 首行 / 首列内缩 `GRID_CELL_INSET`,聚焦框(1.5px 描边)不会贴面板边缘
/// 被裁掉一半,保证框线完整可见喵。(公开便于单测)喵
pub fn grid_rects(panel: &Rect, sections: &[bool], scroll_offset: f32, s: f32) -> Vec<Rect> {
    let gap = GRID_GAP * s;
    let cell_h = GRID_CELL_H * s;
    let item_w = panel.width() - LIST_GUTTER_PX;
    let cols = grid_cols(panel.width() / s);
    let cell_stretch = (item_w - gap * (cols as f32 - 1.0)) / cols as f32;
    let inset = GRID_CELL_INSET * s;

    let mut rects = Vec::with_capacity(sections.len());
    let mut y = panel.top - scroll_offset * s + inset;
    let mut col = 0usize;
    let mut row_open = false;

    for is_section in sections {
        if *is_section {
            // 分组头: 换行并占整行喵
            if row_open {
                y += cell_h + gap;
                col = 0;
                row_open = false;
            }
            rects.push(Rect::from_xywh(panel.left + inset, y, item_w - inset, GRID_SECTION_H * s));
            y += GRID_SECTION_H * s + gap;
        } else {
            if col == 0 {
                if row_open {
                    y += cell_h + gap; // 上一行已满,换行喵
                } else {
                    row_open = true;
                }
            }
            let x = panel.left + inset + col as f32 * (cell_stretch + gap);
            rects.push(Rect::from_xywh(x, y, cell_stretch, cell_h));
            col = (col + 1) % cols;
        }
    }
    rects
}

/// 网格可视行分组喵(跳过全行分组头,成员为 (条目索引, 水平中心))喵
fn grid_rows(rects: &[Rect], sections: &[bool]) -> Vec<(f32, Vec<(usize, f32)>)> {
    let mut rows: Vec<(f32, Vec<(usize, f32)>)> = Vec::new();
    for (i, (r, is_section)) in rects.iter().zip(sections.iter()).enumerate() {
        if *is_section {
            continue;
        }
        if let Some(last) = rows.last_mut()
            && (last.0 - r.top).abs() < 0.5
        {
            last.1.push((i, r.center_x()));
        } else {
            rows.push((r.top, vec![(i, r.center_x())]));
        }
    }
    rows
}

/// 网格纵向导航: 取「同列」最近相邻行的条目喵。
///
/// 分组头占整行导致行排列不齐,旧实现按「固定列数」做索引跳步,
/// 会跳过整组应用;改用几何找行内水平中心最近的条目,焦点严格逐行移动喵。
pub fn grid_vertical_target(rects: &[Rect], sections: &[bool], cur: usize, dir: isize) -> Option<usize> {
    let rows = grid_rows(rects, sections);
    let row_idx = rows
        .iter()
        .position(|(_, items)| items.iter().any(|(i, _)| *i == cur))?;
    let target_row: &(f32, Vec<(usize, f32)>) = if dir < 0 {
        rows.get(row_idx.checked_sub(1)?)?
    } else {
        rows.get(row_idx + 1)?
    };
    let cur_cx = rects.get(cur)?.center_x();
    target_row
        .1
        .iter()
        .min_by(|a, b| (a.1 - cur_cx).abs().partial_cmp(&(b.1 - cur_cx).abs()).unwrap())
        .map(|(i, _)| *i)
}

/// 网格横向导航: 同行内最近邻居(行首/行尾原地不动,不跨组跳行)喵
pub fn grid_horizontal_target(rects: &[Rect], sections: &[bool], cur: usize, dir: isize) -> Option<usize> {
    let rows = grid_rows(rects, sections);
    let row = rows.iter().find(|(_, items)| items.iter().any(|(i, _)| *i == cur))?;
    let pos = row.1.iter().position(|(i, _)| *i == cur)?;
    let n = row.1.len();
    let target = if dir < 0 {
        pos.checked_sub(1)?
    } else {
        (pos + 1).min(n - 1)
    };
    (target != pos).then(|| row.1[target].0)
}

