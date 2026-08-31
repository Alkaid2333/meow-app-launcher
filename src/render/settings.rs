//! 配置 GUI 渲染层喵~
//!
//! 依据 window-design skill 的配置 GUI 设计,绘制「侧边栏 + 分组卡片」设置界面喵。
//! 只取几何形状与配色 token,不照搬 WinIsland 专属的音乐/组件页等功能喵。
//!
//! 结构: 左列侧边栏(红绿灯 + 页面导航),右侧内容区(分组卡片 + 行控件)喵。
//! 绘制同时产出「命中区列表」,供交互层做点击命中测试喵。

use crate::render::font::FontCache;
use crate::render::shape;
use crate::render::theme::SettingsTheme;
use skia_safe::{BlurStyle, Canvas, Color, MaskFilter, Paint, PaintStyle, Rect};

/// 配置窗口宽度(逻辑 px)喵
pub const SETTINGS_WIDTH: f32 = 760.0;
/// 配置窗口高度(逻辑 px)喵
pub const SETTINGS_HEIGHT: f32 = 680.0;
/// 窗口圆角喵
pub const WINDOW_RADIUS: f32 = 16.0;
/// 侧边栏宽度喵
pub const SIDEBAR_WIDTH: f32 = 184.0;

/// 导航行起始 y 喵
const NAV_START_Y: f32 = 64.0;
/// 导航行高度喵
const NAV_ROW_HEIGHT: f32 = 34.0;
/// 内容区左右内边距喵
const CONTENT_PADDING: f32 = 24.0;
/// 分组圆角喵
const GROUP_RADIUS: f32 = 12.0;
/// 分组内边距喵
const GROUP_PADDING: f32 = 16.0;
/// 普通行高喵
const ROW_HEIGHT: f32 = 48.0;
/// 分组标题行高喵
const SECTION_HEIGHT: f32 = 36.0;
/// 分组间距喵
const GROUP_GAP: f32 = 16.0;

/// 配置页面喵
#[derive(Debug, Clone)]
pub struct SettingsPage {
    /// 页面标题(导航显示)喵
    pub title: String,
    /// 分组列表喵
    pub groups: Vec<SettingsGroup>,
}

/// 配置分组喵
#[derive(Debug, Clone)]
pub struct SettingsGroup {
    /// 分组标题喵
    pub title: String,
    /// 行列表喵
    pub rows: Vec<SettingsRow>,
}

/// 配置行控件喵
#[derive(Debug, Clone)]
pub enum SettingsRow {
    /// 布尔开关喵
    Switch { label: String, value: bool },
    /// 数值步进喵
    Stepper { label: String, value: i32, min: i32, max: i32 },
    /// 只读信息喵
    Label { label: String, value: String },
    /// 动作按钮喵
    Button { label: String },
}

/// 行命中类型喵(供交互层使用)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowHit {
    /// 导航页(索引)喵
    Nav(usize),
    /// 红绿灯(0=关闭 1=最小化 2=占位)喵
    TrafficLight(usize),
    /// 开关行(索引)喵
    Switch(usize),
    /// 步进减号(索引)喵
    StepperDec(usize),
    /// 步进加号(索引)喵
    StepperInc(usize),
    /// 按钮行(索引)喵
    Button(usize),
}

/// 布局结果喵(绘制时产出,供命中测试)喵
#[derive(Debug, Clone)]
pub struct SettingsLayout {
    /// 命中区列表:(矩形, 命中类型)喵
    pub hits: Vec<(Rect, RowHit)>,
    /// 内容区总高度(逻辑 px,用于滚动)喵
    pub content_height: f32,
}

/// 绘制配置窗口喵,返回布局结果喵
pub fn paint_settings(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    pages: &[SettingsPage],
    current_page: usize,
    scroll: f32,
) -> SettingsLayout {
    let mut hits = Vec::new();

    // 清空背景 + 整体圆角窗口喵
    canvas.clear(Color::TRANSPARENT);
    let win_rect = Rect::from_xywh(0.0, 0.0, SETTINGS_WIDTH, SETTINGS_HEIGHT);
    let win_path = shape::rounded_rect_path(win_rect, WINDOW_RADIUS);

    // 窗口阴影喵
    canvas.save();
    canvas.translate((0.0, 4.0));
    let mut shadow = Paint::default();
    shadow.set_color(theme.shadow);
    shadow.set_anti_alias(true);
    shadow.set_mask_filter(MaskFilter::blur(BlurStyle::Normal, 16.0, None));
    canvas.draw_path(&win_path, &shadow);
    canvas.restore();

    let mut fill = Paint::default();
    fill.set_color(theme.win_bg);
    fill.set_anti_alias(true);
    canvas.draw_path(&win_path, &fill);

    // 裁剪到窗口内,后续绘制不越界喵
    canvas.save();
    canvas.clip_path(&win_path, None, Some(false));

    // 侧边栏喵
    paint_sidebar(canvas, theme, fonts, pages, current_page, &mut hits);

    // 内容区喵
    let content_height = paint_content(canvas, theme, fonts, pages, current_page, scroll, &mut hits);

    canvas.restore();

    SettingsLayout {
        hits,
        content_height,
    }
}

/// 绘制侧边栏(底色 + 红绿灯 + 导航)喵
fn paint_sidebar(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    pages: &[SettingsPage],
    current_page: usize,
    hits: &mut Vec<(Rect, RowHit)>,
) {
    let sidebar_rect = Rect::from_xywh(0.0, 0.0, SIDEBAR_WIDTH, SETTINGS_HEIGHT);
    let mut bg = Paint::default();
    bg.set_color(theme.sidebar_bg);
    bg.set_anti_alias(true);
    canvas.draw_rect(sidebar_rect, &bg);

    // 红绿灯: 三枚圆点,圆心 (20,20)/(40,20)/(60,20),半径 6 喵
    let colors = [
        theme.danger,                      // 关闭喵
        Color::from_rgb(0xFE, 0xBC, 0x2E), // 最小化喵
        theme.disabled,                    // 占位喵
    ];
    for (i, color) in colors.iter().enumerate() {
        let cx = 20.0 + i as f32 * 20.0;
        let cy = 20.0;
        let mut p = Paint::default();
        p.set_color(*color);
        p.set_anti_alias(true);
        canvas.draw_circle((cx, cy), 6.0, &p);
        // 命中区(略大于视觉)喵
        hits.push((
            Rect::from_xywh(cx - 8.0, cy - 8.0, 16.0, 16.0),
            RowHit::TrafficLight(i),
        ));
    }

    // 导航行喵
    for (i, page) in pages.iter().enumerate() {
        let y = NAV_START_Y + i as f32 * NAV_ROW_HEIGHT;
        let row_rect = Rect::from_xywh(8.0, y, SIDEBAR_WIDTH - 16.0, NAV_ROW_HEIGHT);
        let selected = i == current_page;

        // 选中态: accent 底色圆角行喵
        if selected {
            let path = shape::rounded_rect_path(row_rect, 7.0);
            let mut p = Paint::default();
            p.set_color(theme.accent);
            p.set_anti_alias(true);
            canvas.draw_path(&path, &p);
        }

        // 标签喵
        let font = fonts.font(13.0);
        let mut p = Paint::default();
        p.set_color(if selected {
            Color::from_rgb(0xFF, 0xFF, 0xFF)
        } else {
            theme.text
        });
        p.set_anti_alias(true);
        let text_rect = Rect::from_xywh(16.0, y, row_rect.width() - 16.0, NAV_ROW_HEIGHT);
        crate::render::text::draw_clipped(canvas, &page.title, text_rect, &font, &p);

        hits.push((row_rect, RowHit::Nav(i)));
    }
}

/// 绘制内容区(分组卡片),返回内容总高度喵
fn paint_content(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    pages: &[SettingsPage],
    current_page: usize,
    scroll: f32,
    hits: &mut Vec<(Rect, RowHit)>,
) -> f32 {
    let content_left = SIDEBAR_WIDTH;
    let content_width = SETTINGS_WIDTH - SIDEBAR_WIDTH;

    // 内容区裁剪(可滚动)喵
    let content_rect = Rect::from_xywh(content_left, 0.0, content_width, SETTINGS_HEIGHT);
    canvas.save();
    canvas.clip_rect(content_rect, None, Some(false));
    canvas.translate((0.0, -scroll));

    let Some(page) = pages.get(current_page) else {
        canvas.restore();
        return 0.0;
    };

    let mut y = CONTENT_PADDING;
    let mut row_index = 0;

    for group in &page.groups {
        // 计算分组高度喵
        let group_h = SECTION_HEIGHT + group.rows.len() as f32 * ROW_HEIGHT + GROUP_PADDING * 2.0;
        let group_rect = Rect::from_xywh(
            content_left + CONTENT_PADDING,
            y,
            content_width - CONTENT_PADDING * 2.0,
            group_h,
        );

        // 分组卡片底色喵
        let path = shape::rounded_rect_path(group_rect, GROUP_RADIUS);
        let mut bg = Paint::default();
        bg.set_color(theme.group_bg);
        bg.set_anti_alias(true);
        canvas.draw_path(&path, &bg);

        // 分组标题喵
        let title_font = fonts.font(13.0);
        let mut title_paint = Paint::default();
        title_paint.set_color(theme.text_dim);
        title_paint.set_anti_alias(true);
        let title_rect = Rect::from_xywh(
            group_rect.left + GROUP_PADDING,
            group_rect.top,
            group_rect.width() - GROUP_PADDING * 2.0,
            SECTION_HEIGHT,
        );
        crate::render::text::draw_clipped(canvas, &group.title, title_rect, &title_font, &title_paint);

        // 分组内行喵
        let mut row_y = group_rect.top + SECTION_HEIGHT;
        for row in &group.rows {
            let row_rect = Rect::from_xywh(
                group_rect.left + GROUP_PADDING,
                row_y,
                group_rect.width() - GROUP_PADDING * 2.0,
                ROW_HEIGHT,
            );
            paint_row(canvas, theme, fonts, row, row_rect, row_index, hits);
            row_y += ROW_HEIGHT;
            row_index += 1;
        }

        y += group_h + GROUP_GAP;
    }

    // 内容总高度(用于滚动)喵
    let content_height = y + CONTENT_PADDING;
    canvas.restore();
    content_height
}

/// 绘制单行控件喵
fn paint_row(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    row: &SettingsRow,
    rect: Rect,
    index: usize,
    hits: &mut Vec<(Rect, RowHit)>,
) {
    match row {
        SettingsRow::Switch { label, value } => {
            // 标签喵
            draw_row_label(canvas, theme, fonts, label, rect);
            // 开关喵
            draw_toggle(canvas, theme, rect, *value);
            hits.push((rect, RowHit::Switch(index)));
        }
        SettingsRow::Stepper { label, value, .. } => {
            draw_row_label(canvas, theme, fonts, label, rect);
            // 减号按钮喵
            let dec_rect = Rect::from_xywh(rect.right - 96.0, rect.center_y() - 14.0, 28.0, 28.0);
            draw_control_button(canvas, theme, fonts, dec_rect, "−");
            hits.push((dec_rect, RowHit::StepperDec(index)));
            // 数值喵
            let value_font = fonts.font(13.0);
            let mut vp = Paint::default();
            vp.set_color(theme.text);
            vp.set_anti_alias(true);
            let value_rect = Rect::from_xywh(rect.right - 68.0, rect.center_y() - 14.0, 40.0, 28.0);
            crate::render::text::draw_centered(canvas, &value.to_string(), value_rect, &value_font, &vp);
            // 加号按钮喵
            let inc_rect = Rect::from_xywh(rect.right - 28.0, rect.center_y() - 14.0, 28.0, 28.0);
            draw_control_button(canvas, theme, fonts, inc_rect, "+");
            hits.push((inc_rect, RowHit::StepperInc(index)));
        }
        SettingsRow::Label { label, value } => {
            draw_row_label(canvas, theme, fonts, label, rect);
            let value_font = fonts.font(13.0);
            let mut vp = Paint::default();
            vp.set_color(theme.text_dim);
            vp.set_anti_alias(true);
            let value_rect = Rect::from_xywh(rect.right - 220.0, rect.top, 220.0, rect.height());
            crate::render::text::draw_clipped(canvas, value, value_rect, &value_font, &vp);
        }
        SettingsRow::Button { label } => {
            // 整行按钮喵
            let mut btn_rect = rect;
            btn_rect.inset((0.0, 8.0));
            let path = shape::rounded_rect_path(btn_rect, 8.0);
            let mut bg = Paint::default();
            bg.set_color(theme.control_bg);
            bg.set_anti_alias(true);
            canvas.draw_path(&path, &bg);
            let font = fonts.font(13.0);
            let mut p = Paint::default();
            p.set_color(theme.accent);
            p.set_anti_alias(true);
            crate::render::text::draw_centered(canvas, label, btn_rect, &font, &p);
            hits.push((rect, RowHit::Button(index)));
        }
    }
}

/// 绘制行标签喵
fn draw_row_label(canvas: &Canvas, theme: &SettingsTheme, fonts: &FontCache, label: &str, rect: Rect) {
    let font = fonts.font(13.0);
    let mut p = Paint::default();
    p.set_color(theme.text);
    p.set_anti_alias(true);
    let label_rect = Rect::from_xywh(rect.left, rect.top, rect.width() - 120.0, rect.height());
    crate::render::text::draw_clipped(canvas, label, label_rect, &font, &p);
}

/// 绘制开关喵
fn draw_toggle(canvas: &Canvas, theme: &SettingsTheme, row_rect: Rect, on: bool) {
    let track_w = 36.0;
    let track_h = 20.0;
    let track_rect = Rect::from_xywh(
        row_rect.right - track_w,
        row_rect.center_y() - track_h / 2.0,
        track_w,
        track_h,
    );
    let track_path = shape::rounded_rect_path(track_rect, 10.0);
    let mut p = Paint::default();
    p.set_color(if on { theme.toggle_on } else { theme.toggle_off });
    p.set_anti_alias(true);
    canvas.draw_path(&track_path, &p);

    // 旋钮喵
    let knob_size = 16.0;
    let knob_x = if on {
        track_rect.right - knob_size - 2.0
    } else {
        track_rect.left + 2.0
    };
    let knob_rect = Rect::from_xywh(
        knob_x,
        track_rect.center_y() - knob_size / 2.0,
        knob_size,
        knob_size,
    );
    let mut kp = Paint::default();
    kp.set_color(Color::WHITE);
    kp.set_anti_alias(true);
    canvas.draw_circle(knob_rect.center(), knob_size / 2.0, &kp);
}

/// 绘制控件按钮(步进 +/-)喵
fn draw_control_button(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    rect: Rect,
    symbol: &str,
) {
    let path = shape::rounded_rect_path(rect, 7.0);
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

    let f = fonts.font(14.0);
    let mut p = Paint::default();
    p.set_color(theme.text);
    p.set_anti_alias(true);
    crate::render::text::draw_centered(canvas, symbol, rect, &f, &p);
}
