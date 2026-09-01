//! 配置 GUI 渲染层喵~
//!
//! 「暖纸精密仪器台」风格: 左侧品牌侧边栏 + 右侧内容区(页头 + 分组卡片)喵。
//! * 去掉苹果风红绿灯,改用品牌块 + 右上角方角关闭钮喵
//! * 每个可调行都带「调整类型」徽章(开关/数字/选项/动作)喵
//! * 数值行支持步进(±每步幅度)与点击输入框手动键入喵
//!
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
pub const WINDOW_RADIUS: f32 = 14.0;
/// 侧边栏宽度喵
pub const SIDEBAR_WIDTH: f32 = 168.0;

/// 页头区高度(标题 + 副标题 + 分隔线)喵
const HEADER_HEIGHT: f32 = 66.0;
/// 导航行起始 y 喵
const NAV_START_Y: f32 = 74.0;
/// 导航行高度喵
const NAV_ROW_HEIGHT: f32 = 34.0;
/// 内容区左右内边距喵
const CONTENT_PADDING: f32 = 22.0;
/// 分组圆角喵
const GROUP_RADIUS: f32 = 10.0;
/// 分组内边距喵
const GROUP_PADDING: f32 = 14.0;
/// 普通行高喵
const ROW_HEIGHT: f32 = 46.0;
/// 分组标题行高喵
const SECTION_HEIGHT: f32 = 30.0;
/// 分组间距喵
const GROUP_GAP: f32 = 14.0;
/// 调整类型徽章尺寸喵
const CHIP_W: f32 = 34.0;
const CHIP_H: f32 = 16.0;

/// 配置页面喵
#[derive(Debug, Clone)]
pub struct SettingsPage {
    /// 页面标题(导航显示)喵
    pub title: String,
    /// 页头副标题喵
    pub subtitle: String,
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
    Switch {
        id: RowId,
        label: String,
        value: bool,
    },
    /// 数值步进喵(step = 每次点击调整幅度,unit = 单位)喵
    Stepper {
        id: RowId,
        label: String,
        value: i32,
        min: i32,
        max: i32,
        step: i32,
        unit: String,
    },
    /// 只读信息喵
    Label { label: String, value: String },
    /// 动作按钮喵
    Button { id: RowId, label: String },
    /// 可点选的应用行喵
    AppPick {
        name: String,
        favorite: bool,
        tags: String,
        selected: bool,
    },
    /// 标签芯片喵
    Chip { label: String },
    /// 单行输入(显示当前草稿)喵
    Input {
        id: RowId,
        label: String,
        value: String,
    },
}

/// 行命中类型喵(供交互层使用)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowHit {
    /// 导航页(索引)喵
    Nav(usize),
    /// 右上角关闭钮喵
    Close,
    /// 开关行喵
    Switch(RowId),
    /// 步进减号喵
    StepperDec(RowId),
    /// 步进加号喵
    StepperInc(RowId),
    /// 点击数值框进入手动键入喵
    StepperEdit(RowId),
    /// 按钮行喵
    Button(RowId),
    /// 点选应用(索引)喵
    AppPick(usize),
    /// 收藏星标(索引)喵
    FavStar(usize),
    /// 标签芯片(索引)喵
    Chip(usize),
    /// 输入行喵
    Input(RowId),
}

/// 配置行身份喵(热更新时不靠页面下标)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowId {
    DarkMode,
    Backdrop,
    Visual,
    MotionMode,
    Easing,
    AutoMorph,
    Draggable,
    ReduceMotion,
    IslandW,
    IslandH,
    IslandX,
    IslandY,
    ExpandedW,
    ExpandedH,
    ExpandedR,
    InputRatio,
    Margin,
    Squash,
    IconSize,
    AlwaysOnTop,
    HotkeyEnabled,
    /// 重新录制全局热键喵
    HotkeyRecord,
    ShowRecent,
    ShowFavorites,
    ShowFrequent,
    ShowAll,
    SearchMode,
    Rescan,
    RemoveApp,
    TagInput,
    SpringDuration(u8),
    SpringBounce(u8),
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
///
/// `edit` 为正在手动键入的数值行(id + 草稿);`recording` 为热键录制中喵。
pub fn paint_settings(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    pages: &[SettingsPage],
    current_page: usize,
    scroll: f32,
    edit: Option<&(RowId, String)>,
    recording: bool,
) -> SettingsLayout {
    let mut hits = Vec::new();

    // 清空背景 + 整体圆角窗口喵
    canvas.clear(Color::TRANSPARENT);
    let win_rect = Rect::from_xywh(0.0, 0.0, SETTINGS_WIDTH, SETTINGS_HEIGHT);
    let win_path = shape::rounded_rect_path(win_rect, WINDOW_RADIUS);

    // 窗口阴影(柔和的底部投影)喵
    canvas.save();
    canvas.translate((0.0, 3.0));
    let mut shadow = Paint::default();
    shadow.set_color(theme.shadow);
    shadow.set_anti_alias(true);
    shadow.set_mask_filter(MaskFilter::blur(BlurStyle::Normal, 14.0, None));
    canvas.draw_path(&win_path, &shadow);
    canvas.restore();

    let mut fill = Paint::default();
    fill.set_color(theme.win_bg);
    fill.set_anti_alias(true);
    canvas.draw_path(&win_path, &fill);

    // 裁剪到窗口内,后续绘制不越界喵
    canvas.save();
    canvas.clip_path(&win_path, None, Some(false));

    // 侧边栏(品牌块 + 导航)喵
    paint_sidebar(canvas, theme, fonts, pages, current_page, &mut hits);

    // 内容区(页头 + 可滚动分组)喵
    let content_height = paint_content(canvas, theme, fonts, pages, current_page, scroll, edit, recording, &mut hits);

    // 右上角关闭钮喵
    paint_close_button(canvas, theme, fonts, &mut hits);

    canvas.restore();

    SettingsLayout {
        hits,
        content_height,
    }
}

/// 绘制侧边栏: 品牌块 + 导航喵
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

    // 品牌块: 小徽标 + 产品名 + 版本喵
    let logo = Rect::from_xywh(20.0, 20.0, 28.0, 28.0);
    let logo_path = shape::rounded_rect_path(logo, 8.0);
    let mut lp = Paint::default();
    lp.set_color(theme.accent);
    lp.set_anti_alias(true);
    canvas.draw_path(&logo_path, &lp);
    let mut gp = Paint::default();
    gp.set_color(Color::WHITE);
    gp.set_anti_alias(true);
    crate::render::text::draw_centered(canvas, "喵", logo, &fonts.font(13.0), &gp);

    let mut tp = Paint::default();
    tp.set_color(theme.text);
    tp.set_anti_alias(true);
    crate::render::text::draw_clipped(
        canvas,
        "meowal",
        Rect::from_xywh(logo.right + 10.0, 18.0, SIDEBAR_WIDTH - logo.right - 20.0, 24.0),
        &fonts.font(14.0),
        &tp,
    );
    let mut vp = Paint::default();
    vp.set_color(theme.text_dim);
    vp.set_anti_alias(true);
    crate::render::text::draw_clipped(
        canvas,
        &format!("配置台 v{}", env!("CARGO_PKG_VERSION")),
        Rect::from_xywh(logo.right + 10.0, 40.0, SIDEBAR_WIDTH - logo.right - 20.0, 16.0),
        &fonts.font(10.0),
        &vp,
    );

    // 品牌块下方细分隔线喵
    let mut hline = Paint::default();
    hline.set_color(theme.control_border);
    hline.set_anti_alias(true);
    canvas.draw_line(
        (14.0, 64.0),
        (SIDEBAR_WIDTH - 14.0, 64.0),
        &hline,
    );

    // 导航行喵
    for (i, page) in pages.iter().enumerate() {
        let y = NAV_START_Y + i as f32 * (NAV_ROW_HEIGHT + 4.0);
        let row_rect = Rect::from_xywh(10.0, y, SIDEBAR_WIDTH - 20.0, NAV_ROW_HEIGHT);
        let selected = i == current_page;

        if selected {
            // 选中底色: 强调色 9% 的暖晕,配上左侧 4px 指示条(仪器台指针感)喵
            let path = shape::rounded_rect_path(row_rect, 9.0);
            let a = theme.accent;
            let mut tint = Paint::default();
            tint.set_color(Color::from_argb(0x16, a.r(), a.g(), a.b()));
            tint.set_anti_alias(true);
            canvas.draw_path(&path, &tint);

            let bar = Rect::from_xywh(14.0, row_rect.center_y() - 8.0, 4.0, 16.0);
            let bar_path = shape::rounded_rect_path(bar, 2.0);
            let mut bp = Paint::default();
            bp.set_color(theme.accent);
            bp.set_anti_alias(true);
            canvas.draw_path(&bar_path, &bp);
        }

        let glyph = match page.title.as_str() {
            "巢穴" => "◉",
            "灵动岛" => "◎",
            "弹簧" => "∿",
            "应用" => "✧",
            _ => "·",
        };
        let mut gp = Paint::default();
        gp.set_color(if selected { theme.accent } else { theme.moss });
        gp.set_anti_alias(true);
        let font = fonts.font(13.0);
        crate::render::text::draw_centered(
            canvas,
            glyph,
            Rect::from_xywh(row_rect.left + 8.0, row_rect.top, 26.0, NAV_ROW_HEIGHT),
            &font,
            &gp,
        );

        let mut p = Paint::default();
        p.set_color(if selected { theme.accent } else { theme.text });
        p.set_anti_alias(true);
        let text_rect = Rect::from_xywh(row_rect.left + 36.0, row_rect.top, row_rect.width() - 44.0, NAV_ROW_HEIGHT);
        crate::render::text::draw_clipped(canvas, &page.title, text_rect, &font, &p);

        hits.push((row_rect, RowHit::Nav(i)));
    }
}

/// 绘制右上角关闭钮喵(替换苹果风红绿灯)喵
fn paint_close_button(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    hits: &mut Vec<(Rect, RowHit)>,
) {
    let rect = Rect::from_xywh(SETTINGS_WIDTH - 44.0, 14.0, 28.0, 28.0);
    let path = shape::rounded_rect_path(rect, 8.0);
    // 底 + 描边喵
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
    // 关闭符号喵
    let mut p = Paint::default();
    p.set_color(theme.text_dim);
    p.set_anti_alias(true);
    crate::render::text::draw_centered(canvas, "✕", rect, &fonts.font(13.0), &p);
    hits.push((rect, RowHit::Close));
}

/// 绘制内容区(页头 + 分组卡片),返回内容总高度喵
fn paint_content(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    pages: &[SettingsPage],
    current_page: usize,
    scroll: f32,
    edit: Option<&(RowId, String)>,
    recording: bool,
    hits: &mut Vec<(Rect, RowHit)>,
) -> f32 {
    let content_left = SIDEBAR_WIDTH;
    let content_width = SETTINGS_WIDTH - SIDEBAR_WIDTH;

    let Some(page) = pages.get(current_page) else {
        return 0.0;
    };

    // 页头: 标题 + 副标题 + 分隔线(固定在顶部不滚动)喵
    let mut tp = Paint::default();
    tp.set_color(theme.text);
    tp.set_anti_alias(true);
    let title_rect = Rect::from_xywh(content_left + CONTENT_PADDING, 10.0, content_width - 100.0, 40.0);
    crate::render::text::draw_clipped(canvas, &page.title, title_rect, &fonts.font(17.0), &tp);
    let mut sp = Paint::default();
    sp.set_color(theme.text_dim);
    sp.set_anti_alias(true);
    let sub_rect = Rect::from_xywh(content_left + CONTENT_PADDING, 36.0, content_width - 100.0, 22.0);
    crate::render::text::draw_clipped(canvas, &page.subtitle, sub_rect, &fonts.font(10.5), &sp);
    let mut hline = Paint::default();
    hline.set_color(theme.control_border);
    hline.set_anti_alias(true);
    canvas.draw_line(
        (content_left + 14.0, HEADER_HEIGHT - 2.0),
        (SETTINGS_WIDTH - 14.0, HEADER_HEIGHT - 2.0),
        &hline,
    );

    // 内容区裁剪(可滚动)喵
    let content_rect = Rect::from_xywh(content_left, 0.0, content_width, SETTINGS_HEIGHT);
    canvas.save();
    canvas.clip_rect(content_rect, None, Some(false));
    canvas.translate((0.0, -scroll));

    let mut y = HEADER_HEIGHT + 10.0;
    let mut pick_i = 0usize;
    let mut chip_i = 0usize;

    for (gi, group) in page.groups.iter().enumerate() {
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

        // 分组标题: 序号徽标 + 标题喵
        let mut seq = Paint::default();
        seq.set_color(theme.accent);
        seq.set_anti_alias(true);
        crate::render::text::draw_clipped(
            canvas,
            &format!("{:02}", gi + 1),
            Rect::from_xywh(group_rect.left + GROUP_PADDING, group_rect.top, 34.0, SECTION_HEIGHT),
            &fonts.font(11.0),
            &seq,
        );
        let mut title_paint = Paint::default();
        title_paint.set_color(theme.text_dim);
        title_paint.set_anti_alias(true);
        let title_rect = Rect::from_xywh(
            group_rect.left + GROUP_PADDING + 34.0,
            group_rect.top,
            group_rect.width() - GROUP_PADDING * 2.0 - 34.0,
            SECTION_HEIGHT,
        );
        crate::render::text::draw_clipped(canvas, &group.title, title_rect, &fonts.font(12.0), &title_paint);

        // 分组内行喵
        let mut row_y = group_rect.top + SECTION_HEIGHT;
        for row in &group.rows {
            let row_rect = Rect::from_xywh(
                group_rect.left + GROUP_PADDING,
                row_y,
                group_rect.width() - GROUP_PADDING * 2.0,
                ROW_HEIGHT,
            );
            paint_row(canvas, theme, fonts, row, row_rect, scroll, edit, recording, hits, &mut pick_i, &mut chip_i);
            row_y += ROW_HEIGHT;
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
    scroll: f32,
    edit: Option<&(RowId, String)>,
    recording: bool,
    hits: &mut Vec<(Rect, RowHit)>,
    pick_i: &mut usize,
    chip_i: &mut usize,
) {
    match row {
        SettingsRow::Switch { id, label, value } => {
            draw_row_label(canvas, theme, fonts, label, rect, 96.0);
            draw_chip(canvas, fonts, rect.right - 84.0, rect, "开关", theme.moss);
            draw_toggle(canvas, theme, rect, *value);
            hits.push((screen_hit(rect, scroll), RowHit::Switch(*id)));
        }
        SettingsRow::Stepper {
            id,
            label,
            value,
            min,
            max,
            step,
            unit,
        } => {
            draw_row_label(canvas, theme, fonts, label, rect, 236.0);
            // 调整类型徽章喵
            draw_chip(canvas, fonts, rect.right - 222.0, rect, "数字", theme.accent);

            // 每步幅度提示(±n)喵
            let mut hp = Paint::default();
            hp.set_color(theme.text_dim);
            hp.set_anti_alias(true);
            let hint_rect = Rect::from_xywh(rect.right - 184.0, rect.top, 32.0, rect.height());
            crate::render::text::draw_clipped(
                canvas,
                &format!("±{step}"),
                hint_rect,
                &fonts.font(9.5),
                &hp,
            );

            // 步进减号喵
            let dec_rect = Rect::from_xywh(rect.right - 146.0, rect.center_y() - 13.0, 26.0, 26.0);
            draw_control_button(canvas, theme, fonts, dec_rect, "−");
            hits.push((screen_hit(dec_rect, scroll), RowHit::StepperDec(*id)));

            // 数值框(可点击进入手动键入)喵
            let box_rect = Rect::from_xywh(rect.right - 112.0, rect.center_y() - 14.0, 64.0, 28.0);
            let editing = matches!(edit, Some((eid, _)) if *eid == *id);
            let draft: Option<&str> = if editing { edit.map(|e| e.1.as_str()) } else { None };
            draw_value_box(canvas, theme, fonts, box_rect, *value, unit, editing, draft);
            hits.push((screen_hit(box_rect, scroll), RowHit::StepperEdit(*id)));

            // 步进加号喵
            let inc_rect = Rect::from_xywh(rect.right - 34.0, rect.center_y() - 13.0, 26.0, 26.0);
            draw_control_button(canvas, theme, fonts, inc_rect, "+");
            hits.push((screen_hit(inc_rect, scroll), RowHit::StepperInc(*id)));

            let _ = (min, max);
        }
        SettingsRow::Label { label, value } => {
            draw_row_label(canvas, theme, fonts, label, rect, 240.0);
            let value_font = fonts.font(12.5);
            let mut vp = Paint::default();
            vp.set_color(theme.disabled);
            vp.set_anti_alias(true);
            let value_rect = Rect::from_xywh(rect.right - 240.0, rect.top, 240.0, rect.height());
            crate::render::text::draw_clipped(canvas, value, value_rect, &value_font, &vp);
        }
        SettingsRow::Button { id, label } => {
            let mut btn_rect = rect;
            btn_rect.inset((0.0, 8.0));
            // 热键录制中: 高亮成强调色,提示「正在等待按键」喵
            let rec = *id == RowId::HotkeyRecord && recording;
            let path = shape::rounded_rect_path(btn_rect, 8.0);
            let mut bg = Paint::default();
            bg.set_color(if rec { theme.accent } else { theme.control_bg });
            bg.set_anti_alias(true);
            canvas.draw_path(&path, &bg);
            let font = fonts.font(13.0);
            let mut p = Paint::default();
            // 删除类动作用警示色区分;录制中用白字;其余用强调色喵
            p.set_color(if rec {
                Color::WHITE
            } else if *id == RowId::RemoveApp {
                theme.danger
            } else {
                theme.accent
            });
            p.set_anti_alias(true);
            crate::render::text::draw_centered(canvas, label, btn_rect, &font, &p);
            hits.push((screen_hit(rect, scroll), RowHit::Button(*id)));
        }
        SettingsRow::AppPick {
            name,
            favorite,
            tags,
            selected,
        } => {
            if *selected {
                let path = shape::rounded_rect_path(rect, 8.0);
                let mut bg = Paint::default();
                bg.set_color(theme.accent);
                bg.set_anti_alias(true);
                canvas.draw_path(&path, &bg);
                let mut overlay = Paint::default();
                overlay.set_color(theme.group_bg);
                overlay.set_anti_alias(true);
                overlay.set_alpha_f(0.82);
                canvas.draw_path(&path, &overlay);
            }
            let star = if *favorite { "★" } else { "☆" };
            let star_rect = Rect::from_xywh(rect.left, rect.top, 28.0, rect.height());
            let font = fonts.font(15.0);
            let mut sp = Paint::default();
            sp.set_color(if *favorite { theme.accent } else { theme.text_dim });
            sp.set_anti_alias(true);
            crate::render::text::draw_centered(canvas, star, star_rect, &font, &sp);
            hits.push((screen_hit(star_rect, scroll), RowHit::FavStar(*pick_i)));

            let mut np = Paint::default();
            np.set_color(theme.text);
            np.set_anti_alias(true);
            crate::render::text::draw_clipped(
                canvas,
                name,
                Rect::from_xywh(rect.left + 34.0, rect.top, rect.width() - 40.0, rect.height() * 0.55),
                &fonts.font(13.0),
                &np,
            );
            let mut tp = Paint::default();
            tp.set_color(theme.text_dim);
            tp.set_anti_alias(true);
            crate::render::text::draw_clipped(
                canvas,
                tags,
                Rect::from_xywh(rect.left + 34.0, rect.top + rect.height() * 0.5, rect.width() - 40.0, rect.height() * 0.45),
                &fonts.font(11.0),
                &tp,
            );
            hits.push((screen_hit(rect, scroll), RowHit::AppPick(*pick_i)));
            *pick_i += 1;
        }
        SettingsRow::Chip { label } => {
            let chip = Rect::from_xywh(rect.left, rect.center_y() - 12.0, 120.0f32.min(rect.width()), 24.0);
            let path = shape::rounded_rect_path(chip, 12.0);
            let mut bg = Paint::default();
            bg.set_color(theme.control_bg);
            bg.set_anti_alias(true);
            canvas.draw_path(&path, &bg);
            let mut p = Paint::default();
            p.set_color(theme.text);
            p.set_anti_alias(true);
            crate::render::text::draw_centered(canvas, &format!("{label} ×"), chip, &fonts.font(12.0), &p);
            hits.push((screen_hit(chip, scroll), RowHit::Chip(*chip_i)));
            *chip_i += 1;
        }
        SettingsRow::Input { id, label, value } => {
            draw_row_label(canvas, theme, fonts, label, rect, 240.0);
            let box_rect = Rect::from_xywh(rect.right - 220.0, rect.center_y() - 14.0, 220.0, 28.0);
            let path = shape::rounded_rect_path(box_rect, 7.0);
            let mut bg = Paint::default();
            bg.set_color(theme.control_bg);
            bg.set_anti_alias(true);
            canvas.draw_path(&path, &bg);
            let shown = if value.is_empty() { "输入后回车喵" } else { value };
            let mut p = Paint::default();
            p.set_color(if value.is_empty() { theme.text_dim } else { theme.text });
            p.set_anti_alias(true);
            crate::render::text::draw_clipped(
                canvas,
                shown,
                Rect::from_xywh(box_rect.left + 8.0, box_rect.top, box_rect.width() - 12.0, box_rect.height()),
                &fonts.font(12.0),
                &p,
            );
            hits.push((screen_hit(box_rect, scroll), RowHit::Input(*id)));
        }
    }
}

/// 绘制数值框喵: 静态时显示「值+单位」,编辑中显示草稿 + 光标喵
#[allow(clippy::too_many_arguments)]
fn draw_value_box(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    rect: Rect,
    value: i32,
    unit: &str,
    editing: bool,
    draft: Option<&str>,
) {
    let path = shape::rounded_rect_path(rect, 7.0);
    let mut bg = Paint::default();
    bg.set_color(theme.control_bg);
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);

    // 编辑中给强调色描边,标明「正在键入」喵
    let mut border = Paint::default();
    border.set_color(if editing { theme.accent } else { theme.control_border });
    border.set_anti_alias(true);
    border.set_style(PaintStyle::Stroke);
    border.set_stroke_width(if editing { 1.5 } else { 1.0 });
    canvas.draw_path(&path, &border);

    let font = fonts.font(12.0);
    let mut p = Paint::default();
    p.set_color(theme.text);
    p.set_anti_alias(true);

    if editing {
        let text = draft.unwrap_or("");
        let x = rect.left + 8.0;
        let (w, _) = font.measure_str(text, Some(&p));
        let baseline = rect.center_y() - (font.metrics().1.ascent + font.metrics().1.descent) / 2.0;
        let baseline = baseline.round();
        canvas.draw_str(text, (x.round(), baseline), &font, &p);
        // 光标竖线喵
        let mut cp = Paint::default();
        cp.set_color(theme.accent);
        cp.set_anti_alias(true);
        cp.set_stroke_width(1.5);
        let cx = x + w + 2.0;
        canvas.draw_line((cx, rect.top + 5.0), (cx, rect.bottom - 5.0), &cp);
    } else {
        crate::render::text::draw_centered(
            canvas,
            &format!("{value} {unit}"),
            Rect::from_xywh(rect.left, rect.top, rect.width(), rect.height()),
            &font,
            &p,
        );
    }
}

/// 调整类型徽章喵: 小圆角胶囊标签,标注本行调什么喵
#[allow(clippy::too_many_arguments)]
fn draw_chip(
    canvas: &Canvas,
    fonts: &FontCache,
    x: f32,
    row_rect: Rect,
    text: &str,
    color: Color,
) {
    let chip = Rect::from_xywh(x, row_rect.center_y() - CHIP_H / 2.0, CHIP_W, CHIP_H);
    let path = shape::rounded_rect_path(chip, CHIP_H / 2.0);
    let mut bg = Paint::default();
    bg.set_color(Color::from_argb(0x14, color.r(), color.g(), color.b()));
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);
    let mut p = Paint::default();
    p.set_color(color);
    p.set_anti_alias(true);
    crate::render::text::draw_centered(canvas, text, chip, &fonts.font(9.5), &p);
}

/// 命中区转屏幕坐标(内容区绘制时 canvas 已 -scroll,命中要加回来)喵
fn screen_hit(rect: Rect, scroll: f32) -> Rect {
    Rect::from_xywh(rect.left, rect.top - scroll, rect.width(), rect.height())
}

fn draw_row_label(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    label: &str,
    rect: Rect,
    right_reserve: f32,
) {
    let font = fonts.font(13.0);
    let mut p = Paint::default();
    p.set_color(theme.text);
    p.set_anti_alias(true);
    let label_rect = Rect::from_xywh(rect.left, rect.top, (rect.width() - right_reserve).max(40.0), rect.height());
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