//! 应用管理中心渲染层喵~
//!
//! 「工具栏 + 左列表(网格/列表双排版) + 右详情卡」三段式管理界面喵:
//! * 列表区陈列**全部**注册应用(带过滤词与滚动条),可按网格或行排列喵
//! * 详情卡展示选中应用的名称/路径/图标/标签/统计,全部可编辑喵
//! * 绘制同时产出命中区,交互层据此驱动窗口喵

use crate::apps::AppInfo;
use crate::render::edit::{draw_text_edit, TextEdit};
use crate::render::font::FontCache;
use crate::render::theme::SettingsTheme;
use crate::render::{shape, text};
use skia_safe::{Canvas, Color, Image, Paint, PaintStyle, Rect};

/// 管理窗口宽度(逻辑 px)喵
pub const MANAGER_WIDTH: f32 = 880.0;
/// 管理窗口高度(逻辑 px)喵
pub const MANAGER_HEIGHT: f32 = 560.0;
/// 顶栏高度(可拖动窗口)喵
const TITLEBAR_H: f32 = 30.0;
/// 工具栏高度喵
const TOOLBAR_H: f32 = 44.0;
/// 面板区顶部 y 喵
const PANEL_TOP: f32 = TITLEBAR_H + TOOLBAR_H + 6.0;
/// 详情面板宽度喵
const DETAIL_W: f32 = 296.0;
/// 列表行高(列表排版)喵
const ROW_H: f32 = 44.0;
/// 网格单元格高度喵
const GRID_CELL_H: f32 = 92.0;
/// 网格单元格最小宽度喵
const GRID_MIN_CELL_W: f32 = 92.0;
/// 网格间距喵
const GRID_GAP: f32 = 10.0;
/// 列表右侧滚动条安全槽喵
const GUTTER: f32 = 14.0;
/// 列表行/格左右内边距喵
const LIST_PAD: f32 = 10.0;
/// 面板圆角喵
const PANEL_RADIUS: f32 = 8.0;
/// 控件圆角喵
const CTRL_RADIUS: f32 = 4.0;

/// 详情可编辑字段喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailField {
    /// 应用名称喵
    Name,
    /// 启动路径喵
    Path,
    /// 自定义图标路径喵
    Icon,
    /// 新标签输入喵
    TagAdd,
}

/// 屏蔽词(关键词过滤规则)的行内控件喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockPart {
    /// 添加屏蔽词输入框喵
    Input,
    /// 屏蔽词芯片(下标,点击删除)喵
    Chip(usize),
}

/// 管理区命中类型喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagerHit {
    /// 顶栏(按住拖动窗口)喵
    TitleBar,
    /// 右上角关闭钮喵
    Close,
    /// 过滤词输入框喵
    FilterInput,
    /// 排版切换(网格/列表)喵
    LayoutToggle,
    /// 重新扫描系统应用喵
    Rescan,
    /// 列表条目(过滤后下标)喵
    ListItem(usize),
    /// 详情收藏星喵
    FavStar,
    /// 详情输入框喵
    DetailInput(DetailField),
    /// 标签芯片(下标,点击删除)喵
    TagChip(usize),
    /// 屏蔽词控件喵
    Block(BlockPart),
    /// 「打开所在位置」按钮喵
    Reveal,
    /// 「移除应用」按钮喵
    RemoveApp,
    /// 列表滚动条滑块喵
    ScrollThumb,
}

/// 管理窗口布局结果喵
#[derive(Debug, Clone)]
pub struct ManagerLayout {
    /// 命中区列表喵
    pub hits: Vec<(Rect, ManagerHit)>,
    /// 列表滚动条(轨道, 滑块),内容未溢出时为 None 喵
    pub scrollbar: Option<(Rect, Rect)>,
    /// 列表内容总高度(逻辑 px)喵
    pub list_content_height: f32,
    /// 列表面板矩形(供滚动计算)喵
    pub list_rect: Rect,
}

/// 各输入框的编辑草稿槽喵(全部只读借用)喵
#[derive(Debug, Clone, Copy, Default)]
pub struct ManagerEdits<'a> {
    /// 过滤词草稿喵
    pub filter: Option<&'a TextEdit>,
    /// 名称草稿喵
    pub name: Option<&'a TextEdit>,
    /// 路径草稿喵
    pub path: Option<&'a TextEdit>,
    /// 图标路径草稿喵
    pub icon: Option<&'a TextEdit>,
    /// 新标签草稿喵
    pub tag: Option<&'a TextEdit>,
    /// 屏蔽词草稿喵
    pub block: Option<&'a TextEdit>,
}

/// 屏蔽词横条高度喵(列表底部固定区)喵
const BLOCK_BAR_H: f32 = 56.0;

/// 列表面板矩形喵(布局与滚动共用)喵
fn list_panel(width: f32, height: f32) -> Rect {
    Rect::from_xywh(
        14.0,
        PANEL_TOP,
        width - DETAIL_W - 14.0 * 3.0,
        height - PANEL_TOP - 14.0,
    )
}

/// 详情面板矩形喵
fn detail_panel(width: f32, height: f32) -> Rect {
    Rect::from_xywh(width - DETAIL_W - 14.0, PANEL_TOP, DETAIL_W, height - PANEL_TOP - 14.0)
}

/// 绘制应用管理中心喵,返回布局结果喵
///
/// `apps` 为**过滤后**的应用列表;`block_words` 为关键词过滤规则(屏蔽词)的展示串;
/// `icons` 回调取已缓存图标(不触发提取)喵。
#[allow(clippy::too_many_arguments)]
pub fn paint_app_manager(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    apps: &[AppInfo],
    total: usize,
    selected_name: Option<&str>,
    grid: bool,
    scroll: f32,
    edits: ManagerEdits,
    block_words: &[String],
    icons: &mut dyn FnMut(&AppInfo) -> Option<Image>,
    width: f32,
    height: f32,
) -> ManagerLayout {
    let mut hits = Vec::new();

    // 清空背景 + 圆角窗口 + 柔和投影喵
    canvas.clear(Color::TRANSPARENT);
    let win_rect = Rect::from_xywh(0.0, 0.0, width, height);
    let win_path = shape::rounded_rect_path(win_rect, 12.0);
    let mut fill = Paint::default();
    fill.set_color(theme.win_bg);
    fill.set_anti_alias(true);
    canvas.draw_path(&win_path, &fill);

    canvas.save();
    canvas.clip_path(&win_path, None, Some(false));

    // 关闭钮先于顶栏登记命中: 两者区域重叠,先登记者优先命中喵
    paint_close(canvas, theme, width, &mut hits);

    // 顶栏: 标题 + 数量 + 关闭钮喵
    paint_titlebar(canvas, theme, fonts, total, apps.len(), width, &mut hits);

    // 工具栏: 过滤框 + 排版切换 + 重新扫描 + 拖入提示喵
    paint_toolbar(canvas, theme, fonts, grid, edits, width, &mut hits);

    // 列表区(可滚动,底部预留屏蔽词横条)喵
    let mut panel = list_panel(width, height);
    panel.bottom -= BLOCK_BAR_H;
    // 屏蔽词横条先登记命中: 与列表条目即使有重叠,屏蔽词也优先响应喵
    paint_block_bar(canvas, theme, fonts, block_words, edits, panel, &mut hits);
    let (list_h, scrollbar) = paint_list(canvas, theme, fonts, apps, selected_name, grid, scroll, panel, icons, &mut hits);

    // 详情面板喵
    let detail = detail_panel(width, height);
    let selected = selected_name.and_then(|n| apps.iter().find(|a| a.name == n));
    paint_detail(canvas, theme, fonts, selected, edits, detail, icons, &mut hits);

    canvas.restore();

    ManagerLayout {
        hits,
        scrollbar,
        list_content_height: list_h,
        list_rect: panel,
    }
}

/// 顶栏: 标题 + 统计喵
fn paint_titlebar(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    total: usize,
    shown: usize,
    width: f32,
    hits: &mut Vec<(Rect, ManagerHit)>,
) {
    hits.push((Rect::from_xywh(0.0, 0.0, width, TITLEBAR_H), ManagerHit::TitleBar));

    let mut tp = Paint::default();
    tp.set_color(theme.text);
    tp.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        "应用管理中心",
        Rect::from_xywh(16.0, 2.0, 200.0, 26.0),
        &fonts.font(14.0),
        &tp,
    );
    let mut sp = Paint::default();
    sp.set_color(theme.text_dim);
    sp.set_anti_alias(true);
    let stat = if shown == total {
        format!("共 {total} 个应用喵")
    } else {
        format!("{shown} / {total} 个应用喵")
    };
    text::draw_clipped(canvas, &stat, Rect::from_xywh(226.0, 3.0, 200.0, 24.0), &fonts.font(11.0), &sp);
}

/// 右上角关闭钮喵
fn paint_close(canvas: &Canvas, theme: &SettingsTheme, width: f32, hits: &mut Vec<(Rect, ManagerHit)>) {
    let rect = Rect::from_xywh(width - 44.0, 8.0, 26.0, 26.0);
    draw_icon_button(canvas, theme, rect, "close", theme.text_dim);
    hits.push((rect, ManagerHit::Close));
}

/// 工具栏: 过滤框 + 排版切换 + 重新扫描 + 拖入提示喵
fn paint_toolbar(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    grid: bool,
    edits: ManagerEdits,
    width: f32,
    hits: &mut Vec<(Rect, ManagerHit)>,
) {
    let y = TITLEBAR_H + 8.0;
    // 过滤框喵
    let filter_rect = Rect::from_xywh(16.0, y, 220.0, 28.0);
    let editing = edits.filter.is_some();
    draw_input_box(canvas, theme, filter_rect, editing, edits.filter, "输入关键词过滤喵…", fonts);
    hits.push((filter_rect, ManagerHit::FilterInput));

    // 排版切换按钮喵
    let layout_rect = Rect::from_xywh(248.0, y, 88.0, 28.0);
    let label = if grid { "网格排列" } else { "行排列" };
    draw_flat_button(canvas, theme, fonts, layout_rect, label, theme.accent);
    hits.push((layout_rect, ManagerHit::LayoutToggle));

    // 重新扫描按钮喵
    let rescan_rect = Rect::from_xywh(346.0, y, 88.0, 28.0);
    draw_flat_button(canvas, theme, fonts, rescan_rect, "重新扫描", theme.accent);
    hits.push((rescan_rect, ManagerHit::Rescan));

    // 拖入提示喵
    let mut hp = Paint::default();
    hp.set_color(theme.disabled);
    hp.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        "拖入文件即可注册喵",
        Rect::from_xywh(446.0, y + 2.0, width - DETAIL_W - 448.0, 24.0),
        &fonts.font(11.0),
        &hp,
    );
}

/// 屏蔽词横条(列表底部固定区): 说明 + 添加输入框 + 规则芯片喵
fn paint_block_bar(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    block_words: &[String],
    edits: ManagerEdits,
    list_panel: Rect,
    hits: &mut Vec<(Rect, ManagerHit)>,
) {
    let bar = Rect::from_xywh(list_panel.left, list_panel.bottom + 6.0, list_panel.width(), BLOCK_BAR_H - 6.0);

    let mut lp = Paint::default();
    lp.set_color(theme.text);
    lp.set_anti_alias(true);
    text::draw_clipped(canvas, "屏蔽词", Rect::from_xywh(bar.left + 2.0, bar.top + 4.0, 58.0, 20.0), &fonts.font(12.0), &lp);
    let mut hp = Paint::default();
    hp.set_color(theme.disabled);
    hp.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        "命中名称的启动项会被隐藏喵",
        Rect::from_xywh(bar.left + 60.0, bar.top + 6.0, 180.0, 18.0),
        &fonts.font(10.0),
        &hp,
    );

    // 添加输入框喵
    let input_rect = Rect::from_xywh(bar.left + 2.0, bar.top + 26.0, 150.0, 26.0);
    draw_input_box(canvas, theme, input_rect, edits.block.is_some(), edits.block, "回车添加屏蔽词喵", fonts);
    hits.push((input_rect, ManagerHit::Block(BlockPart::Input)));

    // 规则芯片流式排列喵
    let chip_font = fonts.font(10.5);
    let mut cx = input_rect.right + 8.0;
    let cy = bar.top + 28.0;
    for (i, word) in block_words.iter().enumerate() {
        let mut mp = Paint::default();
        mp.set_color(theme.text);
        mp.set_anti_alias(true);
        let (tw, _) = chip_font.measure_str(word, Some(&mp));
        let chip_w = (tw + 26.0).min(bar.right - cx - 2.0).max(40.0);
        if cx + chip_w > bar.right {
            break; // 一行放不下就截断,完整管理走配置 GUI 喵
        }
        let chip = Rect::from_xywh(cx, cy, chip_w, 22.0);
        let chip_path = shape::rounded_rect_path(chip, 11.0);
        let mut cbg = Paint::default();
        cbg.set_color(theme.control_bg);
        cbg.set_anti_alias(true);
        canvas.draw_path(&chip_path, &cbg);
        let mut cp = Paint::default();
        cp.set_color(theme.text);
        cp.set_anti_alias(true);
        text::draw_centered(canvas, &format!("{word} ×"), chip, &chip_font, &cp);
        hits.push((chip, ManagerHit::Block(BlockPart::Chip(i))));
        cx += chip_w + 6.0;
    }
}

/// 内容坐标 → 屏幕坐标喵(命中测试用;绘制时 canvas 已 -scroll,命中要加回来)喵
fn screen_hit(rect: Rect, scroll: f32) -> Rect {
    Rect::from_xywh(rect.left, rect.top - scroll, rect.width(), rect.height())
}

/// 列表区绘制,返回(内容总高度, 滚动条)喵
#[allow(clippy::too_many_arguments)]
fn paint_list(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    apps: &[AppInfo],
    selected_name: Option<&str>,
    grid: bool,
    scroll: f32,
    panel: Rect,
    icons: &mut dyn FnMut(&AppInfo) -> Option<Image>,
    hits: &mut Vec<(Rect, ManagerHit)>,
) -> (f32, Option<(Rect, Rect)>) {
    // 面板底卡喵
    let path = shape::rounded_rect_path(panel, PANEL_RADIUS);
    let mut bg = Paint::default();
    bg.set_color(theme.group_bg);
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);
    let mut border = Paint::default();
    border.set_color(theme.control_border);
    border.set_anti_alias(true);
    border.set_style(PaintStyle::Stroke);
    border.set_stroke_width(1.0);
    canvas.draw_path(&path, &border);

    // 内容裁剪喵
    canvas.save();
    canvas.clip_rect(panel, None, Some(false));
    canvas.translate((0.0, -scroll));

    // 命中区登记起点: 本函数产出的条目命中区要保证落在面板内喵
    let hit_start = hits.len();

    let content_h = if grid {
        let cell_w = grid_cell_w(panel.width());
        let cols = ((panel.width() - LIST_PAD) / (cell_w + GRID_GAP)).floor().max(1.0) as usize;
        let avail_w = panel.width() - LIST_PAD * 2.0;
        let stretch = (avail_w - GRID_GAP * (cols as f32 - 1.0)) / cols as f32;
        for (i, app) in apps.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = panel.left + LIST_PAD + col as f32 * (stretch + GRID_GAP);
            let y = panel.top + LIST_PAD + row as f32 * (GRID_CELL_H + GRID_GAP);
            let rect = Rect::from_xywh(x, y, stretch, GRID_CELL_H);
            paint_grid_cell(canvas, theme, fonts, app, rect, scroll, selected_name == Some(app.name.as_str()), icons, hits, i);
        }
        let rows = apps.len().div_ceil(cols).max(1);
        LIST_PAD * 2.0 + rows as f32 * (GRID_CELL_H + GRID_GAP)
    } else {
        let row_w = panel.width() - LIST_PAD * 2.0 - GUTTER;
        for (i, app) in apps.iter().enumerate() {
            let y = panel.top + LIST_PAD + i as f32 * ROW_H;
            let rect = Rect::from_xywh(panel.left + LIST_PAD, y, row_w, ROW_H);
            paint_row_item(canvas, theme, fonts, app, rect, scroll, selected_name == Some(app.name.as_str()), icons, hits, i);
        }
        LIST_PAD * 2.0 + apps.len() as f32 * ROW_H
    };
    canvas.restore();

    // 视口剔除: 滚出面板的条目命中区一律作废(不许压到面板外,例如屏蔽词横条)喵
    for idx in (hit_start..hits.len()).rev() {
        let (r, _) = &hits[idx];
        if r.bottom <= panel.top + 0.5 || r.top >= panel.bottom - 0.5 {
            hits.swap_remove(idx);
        }
    }

    // 滚动条喵
    let scrollbar = if content_h > panel.height() + 0.5 {
        let track = Rect::from_xywh(panel.right - 8.0, panel.top + 2.0, 4.0, panel.height() - 4.0);
        let thumb_h = (track.height() * track.height() / content_h).max(28.0);
        let travel = (track.height() - thumb_h).max(1.0);
        let max_scroll = content_h - panel.height();
        let prog = (scroll / max_scroll).clamp(0.0, 1.0);
        let thumb = Rect::from_xywh(track.left, track.top + travel * prog, track.width(), thumb_h);

        let mut tp = Paint::default();
        tp.set_color(theme.control_border);
        tp.set_anti_alias(true);
        canvas.draw_path(&shape::rounded_rect_path(track, 2.0), &tp);
        let mut mp = Paint::default();
        mp.set_color(theme.disabled);
        mp.set_anti_alias(true);
        canvas.draw_path(&shape::rounded_rect_path(thumb, 2.0), &mp);
        hits.push((thumb, ManagerHit::ScrollThumb));
        Some((track, thumb))
    } else {
        None
    };
    (content_h, scrollbar)
}

/// 网格单元宽度喵
fn grid_cell_w(panel_w: f32) -> f32 {
    GRID_MIN_CELL_W.max((panel_w - LIST_PAD) * 0.2)
}

/// 列表排版单行喵
#[allow(clippy::too_many_arguments)]
fn paint_row_item(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    app: &AppInfo,
    rect: Rect,
    scroll: f32,
    selected: bool,
    icons: &mut dyn FnMut(&AppInfo) -> Option<Image>,
    hits: &mut Vec<(Rect, ManagerHit)>,
    index: usize,
) {
    if selected {
        let path = shape::rounded_rect_path(rect, 6.0);
        let a = theme.accent;
        let mut bg = Paint::default();
        bg.set_color(Color::from_argb(0x1A, a.r(), a.g(), a.b()));
        bg.set_anti_alias(true);
        canvas.draw_path(&path, &bg);
    }

    // 图标喵
    let icon_rect = Rect::from_xywh(rect.left + 6.0, rect.center_y() - 14.0, 28.0, 28.0);
    draw_app_icon(canvas, theme, fonts, app, icon_rect, icons);

    // 名称 + 副文本喵
    let text_left = rect.left + 44.0;
    let mut np = Paint::default();
    np.set_color(theme.text);
    np.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        &app.name,
        Rect::from_xywh(text_left, rect.top + 3.0, rect.width() - 90.0, rect.height() * 0.55),
        &fonts.font(13.0),
        &np,
    );
    let sub = if app.tags.is_empty() {
        app.path.clone()
    } else {
        app.tags.join(" / ")
    };
    let mut sp = Paint::default();
    sp.set_color(theme.text_dim);
    sp.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        &sub,
        Rect::from_xywh(text_left, rect.top + rect.height() * 0.52, rect.width() - 90.0, rect.height() * 0.45),
        &fonts.font(10.5),
        &sp,
    );

    // 收藏星喵
    let star = if app.favorite { "★" } else { "☆" };
    let mut fp = Paint::default();
    fp.set_color(if app.favorite { theme.accent } else { theme.disabled });
    fp.set_anti_alias(true);
    text::draw_centered(
        canvas,
        star,
        Rect::from_xywh(rect.right - 28.0, rect.top, 24.0, rect.height()),
        &fonts.font(14.0),
        &fp,
    );

    // 命中区换算回屏幕坐标,滚动后点击不偏移喵
    hits.push((screen_hit(rect, scroll), ManagerHit::ListItem(index)));
}

/// 网格排版单格喵
#[allow(clippy::too_many_arguments)]
fn paint_grid_cell(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    app: &AppInfo,
    rect: Rect,
    scroll: f32,
    selected: bool,
    icons: &mut dyn FnMut(&AppInfo) -> Option<Image>,
    hits: &mut Vec<(Rect, ManagerHit)>,
    index: usize,
) {
    if selected {
        let path = shape::rounded_rect_path(rect, 6.0);
        let a = theme.accent;
        let mut bg = Paint::default();
        bg.set_color(Color::from_argb(0x1A, a.r(), a.g(), a.b()));
        bg.set_anti_alias(true);
        canvas.draw_path(&path, &bg);
    }

    let icon_rect = Rect::from_xywh(rect.center_x() - 22.0, rect.top + 8.0, 44.0, 44.0);
    draw_app_icon(canvas, theme, fonts, app, icon_rect, icons);

    let mut np = Paint::default();
    np.set_color(theme.text);
    np.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        &app.name,
        Rect::from_xywh(rect.left + 4.0, rect.top + 56.0, rect.width() - 8.0, 26.0),
        &fonts.font(11.0),
        &np,
    );
    if app.favorite {
        let mut fp = Paint::default();
        fp.set_color(theme.accent);
        fp.set_anti_alias(true);
        text::draw_centered(
            canvas,
            "★",
            Rect::from_xywh(rect.right - 20.0, rect.top + 2.0, 16.0, 16.0),
            &fonts.font(10.0),
            &fp,
        );
    }

    // 命中区换算回屏幕坐标,滚动后点击不偏移喵
    hits.push((screen_hit(rect, scroll), ManagerHit::ListItem(index)));
}

/// 详情面板喵(选中应用信息 + 编辑框)喵
#[allow(clippy::too_many_arguments)]
fn paint_detail(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    app: Option<&AppInfo>,
    edits: ManagerEdits,
    panel: Rect,
    icons: &mut dyn FnMut(&AppInfo) -> Option<Image>,
    hits: &mut Vec<(Rect, ManagerHit)>,
) {
    let path = shape::rounded_rect_path(panel, PANEL_RADIUS);
    let mut bg = Paint::default();
    bg.set_color(theme.group_bg);
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);
    let mut border = Paint::default();
    border.set_color(theme.control_border);
    border.set_anti_alias(true);
    border.set_style(PaintStyle::Stroke);
    border.set_stroke_width(1.0);
    canvas.draw_path(&path, &border);

    let pad = 14.0;
    let inner_w = panel.width() - pad * 2.0;

    let Some(app) = app else {
        let mut ep = Paint::default();
        ep.set_color(theme.disabled);
        ep.set_anti_alias(true);
        text::draw_centered(
            canvas,
            "选中左侧应用查看详情喵~",
            Rect::from_xywh(panel.left, panel.center_y() - 14.0, panel.width(), 28.0),
            &fonts.font(12.5),
            &ep,
        );
        return;
    };

    // 大图标 + 收藏星喵
    let big = Rect::from_xywh(panel.left + pad + 6.0, panel.top + pad, 60.0, 60.0);
    draw_app_icon(canvas, theme, fonts, app, big, icons);
    let star_rect = Rect::from_xywh(big.right + 8.0, big.top + 2.0, 32.0, 32.0);
    let mut sp = Paint::default();
    sp.set_color(if app.favorite { theme.accent } else { theme.disabled });
    sp.set_anti_alias(true);
    text::draw_centered(canvas, if app.favorite { "★" } else { "☆" }, star_rect, &fonts.font(18.0), &sp);
    hits.push((star_rect, ManagerHit::FavStar));

    // 名称大字 + 来源徽章喵
    let mut np = Paint::default();
    np.set_color(theme.text);
    np.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        &app.name,
        Rect::from_xywh(big.left, big.bottom + 6.0, inner_w - 6.0, 22.0),
        &fonts.font(15.0),
        &np,
    );
    let source = match app.source {
        crate::apps::AppSource::Manual => "手动注册",
        crate::apps::AppSource::Scanned => "扫描发现",
    };
    let mut sp2 = Paint::default();
    sp2.set_color(theme.moss);
    sp2.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        source,
        Rect::from_xywh(big.left, big.bottom + 30.0, inner_w - 6.0, 16.0),
        &fonts.font(10.0),
        &sp2,
    );

    let mut y = big.bottom + 54.0;
    // 名称/路径/图标 三行编辑喵
    y = paint_field(
        canvas,
        theme,
        fonts,
        "名称",
        DetailField::Name,
        &app.name,
        edits.name,
        panel.left + pad,
        y,
        inner_w,
        hits,
    );
    y = paint_field(
        canvas,
        theme,
        fonts,
        "路径",
        DetailField::Path,
        &app.path,
        edits.path,
        panel.left + pad,
        y,
        inner_w,
        hits,
    );
    let icon_value = app.icon_path.clone().unwrap_or_default();
    y = paint_field(
        canvas,
        theme,
        fonts,
        "图标",
        DetailField::Icon,
        &icon_value,
        edits.icon,
        panel.left + pad,
        y,
        inner_w,
        hits,
    );

    // 标签区: 添加输入框 + 芯片流式排列喵
    let tag_rect = Rect::from_xywh(panel.left + pad + 44.0, y + 6.0, inner_w - 44.0, 26.0);
    let editing = edits.tag.is_some();
    draw_input_box(canvas, theme, tag_rect, editing, edits.tag, "输入标签回车添加喵", fonts);
    let mut lp = Paint::default();
    lp.set_color(theme.text);
    lp.set_anti_alias(true);
    text::draw_clipped(canvas, "标签", Rect::from_xywh(panel.left + pad, y + 8.0, 40.0, 24.0), &fonts.font(12.0), &lp);
    hits.push((tag_rect, ManagerHit::DetailInput(DetailField::TagAdd)));

    // 芯片流式喵(放不下换行)喵
    let mut cx = panel.left + pad;
    let mut cy = tag_rect.bottom + 8.0;
    let chip_font = fonts.font(10.5);
    for (i, tag) in app.tags.iter().enumerate() {
        let mut mp = Paint::default();
        mp.set_color(theme.text);
        mp.set_anti_alias(true);
        let (tw, _) = chip_font.measure_str(tag, Some(&mp));
        let chip_w = (tw + 26.0).min(inner_w);
        if cx + chip_w > panel.right - pad {
            cx = panel.left + pad;
            cy += 28.0;
        }
        let chip = Rect::from_xywh(cx, cy, chip_w, 22.0);
        let chip_path = shape::rounded_rect_path(chip, 11.0);
        let mut cbg = Paint::default();
        cbg.set_color(theme.control_bg);
        cbg.set_anti_alias(true);
        canvas.draw_path(&chip_path, &cbg);
        let mut cp = Paint::default();
        cp.set_color(theme.text);
        cp.set_anti_alias(true);
        text::draw_centered(canvas, &format!("{tag} ×"), chip, &chip_font, &cp);
        hits.push((chip, ManagerHit::TagChip(i)));
        cx += chip_w + 6.0;
    }

    // 统计信息喵
    let info_y = cy + 36.0;
    let info = [
        format!("启动次数: {}", app.launch_count),
        format!("最近使用: {}", format_unix(app.last_used)),
    ];
    let mut ip = Paint::default();
    ip.set_color(theme.disabled);
    ip.set_anti_alias(true);
    for (i, line) in info.iter().enumerate() {
        text::draw_clipped(
            canvas,
            line,
            Rect::from_xywh(panel.left + pad, info_y + i as f32 * 20.0, inner_w, 18.0),
            &fonts.font(11.0),
            &ip,
        );
    }

    // 动作按钮喵
    let btn_y = panel.bottom - 14.0 - 30.0;
    let btn_w = (inner_w - 10.0) / 2.0;
    let reveal_rect = Rect::from_xywh(panel.left + pad, btn_y, btn_w, 30.0);
    draw_flat_button(canvas, theme, fonts, reveal_rect, "打开所在位置", theme.accent);
    hits.push((reveal_rect, ManagerHit::Reveal));
    let remove_rect = Rect::from_xywh(reveal_rect.right + 10.0, btn_y, btn_w, 30.0);
    draw_flat_button(canvas, theme, fonts, remove_rect, "移除应用", theme.danger);
    hits.push((remove_rect, ManagerHit::RemoveApp));
}

/// 详情单字段编辑行喵,返回下一行 y 喵
#[allow(clippy::too_many_arguments)]
fn paint_field(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    label: &str,
    field: DetailField,
    value: &str,
    edit: Option<&TextEdit>,
    x: f32,
    y: f32,
    width: f32,
    hits: &mut Vec<(Rect, ManagerHit)>,
) -> f32 {
    let mut lp = Paint::default();
    lp.set_color(theme.text);
    lp.set_anti_alias(true);
    text::draw_clipped(canvas, label, Rect::from_xywh(x, y + 4.0, 40.0, 24.0), &fonts.font(12.0), &lp);
    let box_rect = Rect::from_xywh(x + 44.0, y, width - 44.0, 28.0);
    draw_input_box(canvas, theme, box_rect, edit.is_some(), edit, "", fonts);
    if edit.is_none() {
        // 静态展示当前值(空值显示占位)喵
        let shown = if value.is_empty() { "(空)" } else { value };
        let mut vp = Paint::default();
        vp.set_color(if value.is_empty() { theme.disabled } else { theme.text });
        vp.set_anti_alias(true);
        text::draw_clipped(
            canvas,
            shown,
            Rect::from_xywh(box_rect.left + 8.0, box_rect.top, box_rect.width() - 12.0, box_rect.height()),
            &fonts.font(11.5),
            &vp,
        );
    }
    hits.push((box_rect, ManagerHit::DetailInput(field)));
    y + 34.0
}

/// 通用输入框底喵(底 + 边框 + 草稿文本/占位文案)喵
fn draw_input_box(
    canvas: &Canvas,
    theme: &SettingsTheme,
    rect: Rect,
    editing: bool,
    edit: Option<&TextEdit>,
    placeholder: &str,
    fonts: &FontCache,
) {
    let path = shape::rounded_rect_path(rect, CTRL_RADIUS);
    let mut bg = Paint::default();
    bg.set_color(theme.control_bg);
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);
    let mut border = Paint::default();
    border.set_color(if editing { theme.accent } else { theme.control_border });
    border.set_anti_alias(true);
    border.set_style(PaintStyle::Stroke);
    border.set_stroke_width(if editing { 1.5 } else { 1.0 });
    canvas.draw_path(&path, &border);

    match edit {
        Some(te) => draw_text_edit(
            canvas,
            rect,
            &fonts.font(12.0),
            theme.text,
            theme.accent,
            &te.text,
            Some(te),
        ),
        None if !placeholder.is_empty() => {
            let mut p = Paint::default();
            p.set_color(theme.text_dim);
            p.set_anti_alias(true);
            text::draw_clipped(
                canvas,
                placeholder,
                Rect::from_xywh(rect.left + 8.0, rect.top, rect.width() - 12.0, rect.height()),
                &fonts.font(11.5),
                &p,
            );
        }
        None => {}
    }
}

/// 应用图标绘制喵: 有图用图,无图用「首字符 + 强调色底」兜底喵
fn draw_app_icon(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    app: &AppInfo,
    rect: Rect,
    icons: &mut dyn FnMut(&AppInfo) -> Option<Image>,
) {
    let inset = rect.width() * 0.08;
    let inner = Rect::from_xywh(
        rect.left + inset,
        rect.top + inset,
        rect.width() - inset * 2.0,
        rect.height() - inset * 2.0,
    );
    if let Some(img) = icons(app) {
        let mut p = Paint::default();
        p.set_anti_alias(true);
        canvas.draw_image_rect(&img, None, inner, &p);
        return;
    }
    // 兜底: 强调色圆角底 + 名称前两字符喵
    let path = shape::rounded_rect_path(inner, inner.width() * 0.22);
    let mut bg = Paint::default();
    bg.set_color(theme.accent);
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);
    let label: String = app.name.chars().take(2).collect();
    let mut p = Paint::default();
    p.set_color(Color::WHITE);
    p.set_anti_alias(true);
    text::draw_centered(canvas, &label, inner, &fonts.font(inner.width() * 0.38), &p);
}

/// 扁平按钮喵
fn draw_flat_button(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    rect: Rect,
    label: &str,
    color: Color,
) {
    let path = shape::rounded_rect_path(rect, CTRL_RADIUS);
    let mut bg = Paint::default();
    bg.set_color(theme.control_bg);
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);
    let mut border = Paint::default();
    border.set_color(theme.control_border);
    border.set_anti_alias(true);
    border.set_style(PaintStyle::Stroke);
    border.set_stroke_width(1.0);
    canvas.draw_path(&path, &border);
    let mut p = Paint::default();
    p.set_color(color);
    p.set_anti_alias(true);
    text::draw_centered(canvas, label, rect, &fonts.font(12.0), &p);
}

/// 小图标按钮喵
fn draw_icon_button(
    canvas: &Canvas,
    theme: &SettingsTheme,
    rect: Rect,
    icon_name: &str,
    color: Color,
) {
    let path = shape::rounded_rect_path(rect, CTRL_RADIUS);
    let mut bg = Paint::default();
    bg.set_color(theme.group_bg);
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);
    let mut border = Paint::default();
    border.set_color(theme.control_border);
    border.set_anti_alias(true);
    border.set_style(PaintStyle::Stroke);
    border.set_stroke_width(1.0);
    canvas.draw_path(&path, &border);
    crate::render::icon::draw_builtin(canvas, rect, icon_name, color);
}

/// Unix 秒 → 「YYYY-MM-DD HH:MM」喵(0 = 从未)喵
fn format_unix(secs: u64) -> String {
    if secs == 0 {
        return "从未".into();
    }
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (h, m) = (rem / 3600, (rem % 3600) / 60);
    // Howard Hinnant 的 civil_from_days 算法,零依赖换算公历日期喵
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{d:02} {h:02}:{m:02}")
}
