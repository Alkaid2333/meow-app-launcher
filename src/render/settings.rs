//! 配置 GUI 渲染层喵~
//!
//! 「暖纸精密仪器台」风格: 左侧品牌侧边栏 + 右侧内容区(页头 + 分组卡片)喵。
//! * 去掉苹果风红绿灯,改用品牌块 + 右上角方角关闭钮喵
//! * 每个可调行都带「调整类型」徽章(开关/数字/选项/动作)喵
//! * 数值行支持步进(±每步幅度)与点击输入框手动键入喵
//!
//! 绘制同时产出「命中区列表」,供交互层做点击命中测试喵。

use crate::render::edit::{draw_text_edit, TextEdit};
use crate::render::font::FontCache;
use crate::render::shape;
use crate::render::icon;
use crate::render::theme::SettingsTheme;
use skia_safe::{BlurStyle, Canvas, Color, Data, Image, MaskFilter, Paint, PaintStyle, Rect};

/// 配置窗口宽度(逻辑 px)喵
pub const SETTINGS_WIDTH: f32 = 760.0;
/// 配置窗口高度(逻辑 px)喵
pub const SETTINGS_HEIGHT: f32 = 680.0;
/// 窗口圆角喵(Fluent 8px 到 12px 档,取 12)喵
pub const WINDOW_RADIUS: f32 = 12.0;
/// 侧边栏宽度喵
pub const SIDEBAR_WIDTH: f32 = 176.0;

/// 应用图标喵(assets/app_icons 里的 128px png,惰性解码缓存一次)喵
///
/// 配置 GUI 的品牌徽标用它展示;.ico 仅用于 exe 打包,不在此加载喵。
fn app_icon() -> Option<&'static Image> {
    static APP_ICON: std::sync::OnceLock<Option<Image>> = std::sync::OnceLock::new();
    APP_ICON
        .get_or_init(|| {
            let bytes = include_bytes!("../../assets/app_icons/128x128.png");
            Image::from_encoded(Data::new_copy(bytes))
        })
        .as_ref()
}

/// 页头区高度(标题 + 副标题 + 分隔线)喵
const HEADER_HEIGHT: f32 = 72.0;
/// 导航行起始 y 喵
const NAV_START_Y: f32 = 78.0;
/// 导航行高度喵(NavigationView 行高)喵
const NAV_ROW_HEIGHT: f32 = 36.0;
/// 内容区左右内边距喵
const CONTENT_PADDING: f32 = 24.0;
/// 分组圆角喵(Fluent 卡片 8px)喵
const GROUP_RADIUS: f32 = 8.0;
/// 分组内边距喵
const GROUP_PADDING: f32 = 14.0;
/// 普通行高喵
const ROW_HEIGHT: f32 = 44.0;
/// 分组标题行高喵
const SECTION_HEIGHT: f32 = 30.0;
/// 分组间距喵
const GROUP_GAP: f32 = 14.0;
/// 调整类型徽章尺寸喵
const CHIP_W: f32 = 34.0;
const CHIP_H: f32 = 16.0;
/// 控件圆角喵(Fluent 控件 4px)喵
const CTRL_RADIUS: f32 = 4.0;

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
    /// 循环选项喵(点击轮换档位,value 为当前档位名)喵
    Choice {
        id: RowId,
        label: String,
        value: String,
    },
    /// 单行输入(显示当前草稿)喵
    Input {
        id: RowId,
        label: String,
        value: String,
        /// 空值时的提示文案喵
        placeholder: String,
    },
    /// 过滤关键词行喵(index 对应配置里的过滤规则下标)喵
    Filter {
        index: usize,
        keyword: String,
        case_sensitive: bool,
    },
    /// 系统指令行喵(index 对应配置 commands 下标;aliases 为逗号连接的展示串)喵
    Command {
        index: usize,
        kind: crate::platform::SystemCommandKind,
        aliases: String,
    },
}

/// 行命中类型喵(供交互层使用)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowHit {
    /// 导航页(索引)喵
    Nav(usize),
    /// 窗口栏(按住拖动窗口)喵
    TitleBar,
    /// 窗口右下角缩放手柄喵
    ResizeGrip,
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
    /// 输入行喵
    Input(RowId),
    /// 过滤关键词输入框(规则下标)喵
    FilterInput(usize),
    /// 过滤关键词大小写判定(规则下标)喵
    FilterCase(usize),
    /// 删除过滤关键词(规则下标)喵
    FilterDelete(usize),
    /// 切换指令的系统命令类型(下标)喵
    CommandKind(usize),
    /// 指令别名输入框(下标)喵
    CommandAlias(usize),
    /// 删除指令(下标)喵
    CommandDelete(usize),
    /// 恢复该项默认值喵
    Restore(RowId),
    /// 循环选项行(点击在档位间轮换)喵
    Choice(RowId),
    /// 内容区滚动条滑块(按住拖动)喵
    ScrollThumb,
}

/// 配置行身份喵(热更新时不靠页面下标)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowId {
    /// 主题预设切换喵
    Theme,
    /// 动画帧率切换喵
    AnimFps,
    /// 结果排版(网格/列表)切换喵
    AppLayout,
    /// 恢复默认设置喵
    Reset,
    /// 打开应用管理中心喵
    OpenAppManager,
    /// 配置面板动效开关喵
    SettingsAnim,
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
    HotkeyEnabled,
    /// 重新录制全局热键喵
    HotkeyRecord,
    /// 提权启动修饰键切换喵
    ElevateModifier,
    /// 开机自启开关喵
    AutoStart,
    /// 渲染后端切换(CPU/GPU)喵
    RenderBackend,
    ShowRecent,
    ShowFavorites,
    ShowFrequent,
    ShowAll,
    SearchMode,
    /// 添加一条过滤关键词规则喵
    FilterAdd,
    /// Web 搜索引擎切换喵
    WebEngine,
    /// 自定义浏览器输入喵
    WebBrowser,
    /// 添加一条系统指令喵
    CommandAdd,
    /// 切换指令的系统命令类型喵
    CommandKind(usize),
    /// 编辑指令别名喵
    CommandAlias(usize),
    /// 删除指令喵
    CommandDelete(usize),
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
    /// 内容区滚动条(轨道, 滑块),内容未溢出时为 None 喵
    pub scrollbar: Option<(Rect, Rect)>,
}

/// 当前文本编辑焦点喵(绘制层只读;同时只有一个输入框在编辑)喵
#[derive(Debug, Clone, Copy, Default)]
pub struct EditFocus<'a> {
    /// 正在编辑的数值行喵
    pub stepper: Option<(RowId, &'a TextEdit)>,
    /// 正在编辑的过滤关键词行(规则下标)喵
    pub filter: Option<(usize, &'a TextEdit)>,
    /// 正在编辑的指令别名行(下标)喵
    pub command: Option<(usize, &'a TextEdit)>,
    /// 正在编辑的自定义浏览器输入喵
    pub browser: Option<&'a TextEdit>,
}

/// 绘制配置窗口喵,返回布局结果喵
///
/// `edit` 为当前正在编辑的文本焦点(数值/过滤关键词/标签三类输入框);
/// `recording` 为热键录制中;`dirty` 为偏离默认值的行(行首画恢复图标);
/// `fade` 为内容区淡入透明度(1.0 = 不透明,页面切换动画用)喵。
/// `width`/`height` 为窗口当前逻辑尺寸(支持拖拽缩放)喵。
#[allow(clippy::too_many_arguments)]
pub fn paint_settings(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    pages: &[SettingsPage],
    current_page: usize,
    scroll: f32,
    fade: f32,
    edit: EditFocus,
    recording: bool,
    dirty: &[RowId],
    width: f32,
    height: f32,
) -> SettingsLayout {
    let mut hits = Vec::new();

    // 清空背景 + 整体圆角窗口喵
    canvas.clear(Color::TRANSPARENT);
    let win_rect = Rect::from_xywh(0.0, 0.0, width, height);
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
    paint_sidebar(canvas, theme, fonts, pages, current_page, height, &mut hits);

    // 内容区(页头 + 可滚动分组)喵
    let (content_height, scrollbar) =
        paint_content(canvas, theme, fonts, pages, current_page, scroll, fade, edit, recording, dirty, width, height, &mut hits);

    // 右上角关闭钮喵
    paint_close_button(canvas, theme, width, &mut hits);

    // 窗口栏(整条顶部区域可拖动窗口) + 右下角缩放手柄喵
    paint_window_chrome(canvas, theme, width, height, &mut hits);

    canvas.restore();

    SettingsLayout {
        hits,
        content_height,
        scrollbar,
    }
}

/// 绘制侧边栏: 品牌块 + 导航喵
fn paint_sidebar(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    pages: &[SettingsPage],
    current_page: usize,
    height: f32,
    hits: &mut Vec<(Rect, RowHit)>,
) {
    let sidebar_rect = Rect::from_xywh(0.0, 0.0, SIDEBAR_WIDTH, height);
    let mut bg = Paint::default();
    bg.set_color(theme.sidebar_bg);
    bg.set_anti_alias(true);
    canvas.draw_rect(sidebar_rect, &bg);

    // 品牌块: 应用图标(png,裁剪进圆角徽标) + 产品全称 + 版本喵
    let logo = Rect::from_xywh(16.0, 18.0, 30.0, 30.0);
    let logo_path = shape::rounded_rect_path(logo, 8.0);
    let mut lp = Paint::default();
    lp.set_color(theme.accent);
    lp.set_anti_alias(true);
    canvas.draw_path(&logo_path, &lp);
    if let Some(icon) = app_icon() {
        // 图标按圆角徽标裁剪,避免方形图片戳出圆角喵
        canvas.save();
        canvas.clip_path(&logo_path, None, Some(false));
        let mut img_paint = Paint::default();
        img_paint.set_anti_alias(true);
        canvas.draw_image_rect(icon, None, logo, &img_paint);
        canvas.restore();
    } else {
        // 图标解码失败时兜底: 内嵌设置齿轮喵
        icon::draw_builtin(canvas, logo, "settings", Color::WHITE);
    }

    let mut tp = Paint::default();
    tp.set_color(theme.text);
    tp.set_anti_alias(true);
    crate::render::text::draw_clipped(
        canvas,
        "meow app launcher",
        Rect::from_xywh(54.0, 15.0, SIDEBAR_WIDTH - 66.0, 22.0),
        &fonts.font(11.5),
        &tp,
    );
    let mut vp = Paint::default();
    vp.set_color(theme.text_dim);
    vp.set_anti_alias(true);
    crate::render::text::draw_clipped(
        canvas,
        &format!("配置面板 · v{}", env!("CARGO_PKG_VERSION")),
        Rect::from_xywh(54.0, 37.0, SIDEBAR_WIDTH - 66.0, 16.0),
        &fonts.font(9.5),
        &vp,
    );

    // 品牌块下方细分隔线喵
    let mut hline = Paint::default();
    hline.set_color(theme.control_border);
    hline.set_anti_alias(true);
    canvas.draw_line(
        (14.0, 62.0),
        (SIDEBAR_WIDTH - 14.0, 62.0),
        &hline,
    );

    // 导航行喵
    for (i, page) in pages.iter().enumerate() {
        let y = NAV_START_Y + i as f32 * (NAV_ROW_HEIGHT + 4.0);
        let row_rect = Rect::from_xywh(10.0, y, SIDEBAR_WIDTH - 20.0, NAV_ROW_HEIGHT);
        let selected = i == current_page;

        if selected {
            // NavigationView 式选中态: 强调色淡底(8px 圆角) + 靠内指示条喵
            let path = shape::rounded_rect_path(row_rect, 8.0);
            let a = theme.accent;
            let mut tint = Paint::default();
            tint.set_color(Color::from_argb(0x16, a.r(), a.g(), a.b()));
            tint.set_anti_alias(true);
            canvas.draw_path(&path, &tint);

            let bar = Rect::from_xywh(row_rect.left + 4.0, row_rect.center_y() - 8.0, 3.0, 16.0);
            let bar_path = shape::rounded_rect_path(bar, 1.5);
            let mut bp = Paint::default();
            bp.set_color(theme.accent);
            bp.set_anti_alias(true);
            canvas.draw_path(&bar_path, &bp);
        }

        let glyph_rect = Rect::from_xywh(row_rect.left + 8.0, row_rect.top, 26.0, NAV_ROW_HEIGHT);
        let glyph_color = if selected { theme.accent } else { theme.moss };
        let font = fonts.font(13.0);
        match page.title.as_str() {
            // 「关于」页用内嵌信息图标,其余沿用字符图标喵
            "关于" => icon::draw_builtin(canvas, glyph_rect, "info", glyph_color),
            _ => {
                let glyph = match page.title.as_str() {
                    "常规" => "◉",
                    "灵动岛" => "◎",
                    "弹簧" => "∿",
                    "应用" => "✧",
                    _ => "·",
                };
                let mut gp = Paint::default();
                gp.set_color(glyph_color);
                gp.set_anti_alias(true);
                crate::render::text::draw_centered(canvas, glyph, glyph_rect, &font, &gp);
            }
        }

        let mut p = Paint::default();
        p.set_color(if selected { theme.accent } else { theme.text });
        p.set_anti_alias(true);
        let text_rect = Rect::from_xywh(row_rect.left + 36.0, row_rect.top, row_rect.width() - 44.0, NAV_ROW_HEIGHT);
        crate::render::text::draw_clipped(canvas, &page.title, text_rect, &font, &p);

        hits.push((row_rect, RowHit::Nav(i)));
    }
}

/// 绘制右上角关闭钮喵(内嵌 SVG 关闭图标)喵
fn paint_close_button(
    canvas: &Canvas,
    theme: &SettingsTheme,
    width: f32,
    hits: &mut Vec<(Rect, RowHit)>,
) {
    let rect = Rect::from_xywh(width - 44.0, 14.0, 28.0, 28.0);
    let path = shape::rounded_rect_path(rect, CTRL_RADIUS);
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
    // 关闭符号(内嵌 SVG)喵
    icon::draw_builtin(canvas, rect, "close", theme.text_dim);
    hits.push((rect, RowHit::Close));
}

/// 绘制窗口栏装饰与命中区喵: 顶部栏可拖动窗口,右下角为缩放手柄喵
fn paint_window_chrome(
    canvas: &Canvas,
    theme: &SettingsTheme,
    width: f32,
    height: f32,
    hits: &mut Vec<(Rect, RowHit)>,
) {
    // 窗口栏: 顶部整条(侧边栏上沿 + 内容页头上沿)都可拖拽窗口喵
    let title_rect = Rect::from_xywh(0.0, 0.0, width, 30.0);
    hits.push((title_rect, RowHit::TitleBar));

    // 右下角缩放手柄: 三条短斜线提示可拖拽缩放喵
    let grip = Rect::from_xywh(width - 18.0, height - 18.0, 18.0, 18.0);
    let mut lp = Paint::default();
    lp.set_color(theme.disabled);
    lp.set_anti_alias(true);
    lp.set_stroke_width(1.5);
    for i in 0..3 {
        let off = 4.0 + i as f32 * 4.0;
        canvas.draw_line(
            (grip.right - off, grip.bottom - 2.0),
            (grip.right - 2.0, grip.bottom - off),
            &lp,
        );
    }
    hits.push((grip, RowHit::ResizeGrip));
}

/// 绘制内容区(页头 + 分组卡片 + 滚动条),返回(内容总高度, 滚动条几何)喵
#[allow(clippy::too_many_arguments)]
fn paint_content(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    pages: &[SettingsPage],
    current_page: usize,
    scroll: f32,
    fade: f32,
    edit: EditFocus,
    recording: bool,
    dirty: &[RowId],
    width: f32,
    height: f32,
    hits: &mut Vec<(Rect, RowHit)>,
) -> (f32, Option<(Rect, Rect)>) {
    let content_left = SIDEBAR_WIDTH;
    let content_width = width - SIDEBAR_WIDTH;

    let Some(page) = pages.get(current_page) else {
        return (0.0, None);
    };

    // 页头: 大标题 + 副标题 + 分隔线(固定在顶部不滚动)喵
    let mut tp = Paint::default();
    tp.set_color(theme.text);
    tp.set_anti_alias(true);
    let title_rect = Rect::from_xywh(content_left + CONTENT_PADDING, 12.0, content_width - 96.0, 26.0);
    crate::render::text::draw_clipped(canvas, &page.title, title_rect, &fonts.font(18.0), &tp);
    let mut sp = Paint::default();
    sp.set_color(theme.text_dim);
    sp.set_anti_alias(true);
    let sub_rect = Rect::from_xywh(content_left + CONTENT_PADDING, 42.0, content_width - 96.0, 18.0);
    crate::render::text::draw_clipped(canvas, &page.subtitle, sub_rect, &fonts.font(11.5), &sp);
    let mut hline = Paint::default();
    hline.set_color(theme.control_border);
    hline.set_anti_alias(true);
    canvas.draw_line(
        (content_left + 14.0, HEADER_HEIGHT - 2.0),
        (width - 14.0, HEADER_HEIGHT - 2.0),
        &hline,
    );

    // 内容区裁剪: 上沿卡在页头下方,滚动内容不会盖过页头标题栏喵
    let content_rect = Rect::from_xywh(content_left, HEADER_HEIGHT, content_width, height - HEADER_HEIGHT);
    canvas.save();
    canvas.clip_rect(content_rect, None, Some(false));
    canvas.translate((0.0, -scroll));
    // 页面切换淡入层喵(fade = 1 时跳过,省一次离屏合成)喵
    let fading = fade < 1.0;
    if fading {
        canvas.save_layer_alpha_f(None, fade);
    }

    let viewport = Rect::from_xywh(0.0, HEADER_HEIGHT, width, height - HEADER_HEIGHT);
    let mut y = HEADER_HEIGHT + 10.0;

    for group in &page.groups {
        // 计算分组高度喵
        let group_h = SECTION_HEIGHT + group.rows.len() as f32 * ROW_HEIGHT + GROUP_PADDING * 2.0;
        let group_rect = Rect::from_xywh(
            content_left + CONTENT_PADDING,
            y,
            content_width - CONTENT_PADDING * 2.0,
            group_h,
        );

        // 分组卡片: 底色 + 1px 描边(SettingsCard 式分层感)喵
        let path = shape::rounded_rect_path(group_rect, GROUP_RADIUS);
        let mut bg = Paint::default();
        bg.set_color(theme.group_bg);
        bg.set_anti_alias(true);
        canvas.draw_path(&path, &bg);
        let mut card_border = Paint::default();
        card_border.set_color(theme.control_border);
        card_border.set_anti_alias(true);
        card_border.set_style(PaintStyle::Stroke);
        card_border.set_stroke_width(1.0);
        canvas.draw_path(&path, &card_border);

        // 分组标题: Fluent caption 风格,前置强调色圆点喵
        let dot = Rect::from_xywh(
            group_rect.left + GROUP_PADDING + 2.0,
            group_rect.top + (SECTION_HEIGHT - 6.0) / 2.0,
            6.0,
            6.0,
        );
        let mut dp = Paint::default();
        dp.set_color(theme.accent);
        dp.set_anti_alias(true);
        canvas.draw_circle(dot.center(), 3.0, &dp);

        let mut title_paint = Paint::default();
        title_paint.set_color(theme.text_dim);
        title_paint.set_anti_alias(true);
        let title_rect = Rect::from_xywh(
            group_rect.left + GROUP_PADDING + 14.0,
            group_rect.top,
            group_rect.width() - GROUP_PADDING * 2.0 - 14.0,
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
            paint_row(canvas, theme, fonts, row, row_rect, scroll, viewport, edit, recording, dirty, hits);
            row_y += ROW_HEIGHT;
        }

        y += group_h + GROUP_GAP;
    }

    // 内容总高度(用于滚动)喵
    let content_height = y + CONTENT_PADDING;
    if fading {
        canvas.restore();
    }
    canvas.restore();

    // 滚动条(窗口坐标,固定在内容区右缘): 内容溢出才出现喵
    let scrollbar = if content_height > height + 0.5 {
        let track = Rect::from_xywh(
            width - 12.0,
            HEADER_HEIGHT + 2.0,
            4.0,
            height - HEADER_HEIGHT - 4.0,
        );
        let thumb_h = (track.height() * track.height() / content_height).max(28.0);
        let travel = (track.height() - thumb_h).max(1.0);
        let max_scroll = content_height - height;
        let prog = (scroll / max_scroll).clamp(0.0, 1.0);
        let thumb = Rect::from_xywh(track.left, track.top + travel * prog, track.width(), thumb_h);

        let mut track_paint = Paint::default();
        track_paint.set_color(theme.control_border);
        track_paint.set_anti_alias(true);
        let track_path = shape::rounded_rect_path(track, track.width() / 2.0);
        canvas.draw_path(&track_path, &track_paint);

        let mut thumb_paint = Paint::default();
        thumb_paint.set_color(theme.disabled);
        thumb_paint.set_anti_alias(true);
        let thumb_path = shape::rounded_rect_path(thumb, thumb.width() / 2.0);
        canvas.draw_path(&thumb_path, &thumb_paint);
        hits.push((thumb, RowHit::ScrollThumb));

        Some((track, thumb))
    } else {
        None
    };

    (content_height, scrollbar)
}

/// 绘制单行控件喵
///
/// `viewport` 为内容区可视范围(窗口坐标): 滚出视口的行不再登记命中区,
/// 避免看不见的控件被误点喵。
#[allow(clippy::too_many_arguments)]
fn paint_row(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    row: &SettingsRow,
    rect: Rect,
    scroll: f32,
    viewport: Rect,
    edit: EditFocus,
    recording: bool,
    dirty: &[RowId],
    hits: &mut Vec<(Rect, RowHit)>,
) {
    // 命中登记统一走这里: 换算屏幕坐标 + 剔除滚出视口的部分喵
    let mut push_hit = |r: Rect, h: RowHit| {
        let sr = screen_hit(r, scroll);
        if sr.bottom > viewport.top && sr.top < viewport.bottom {
            hits.push((sr, h));
        }
    };

    // 可调行: 偏离默认值时在行首画「恢复默认」图标喵
    let rid = match row {
        SettingsRow::Switch { id, .. }
        | SettingsRow::Stepper { id, .. }
        | SettingsRow::Button { id, .. }
        | SettingsRow::Choice { id, .. } => Some(*id),
        _ => None,
    };
    let label_off = if let Some(r) = rid
        && dirty.contains(&r)
    {
        let icon = Rect::from_xywh(rect.left, rect.center_y() - 9.0, 18.0, 18.0);
        draw_restore_icon(canvas, theme, icon);
        push_hit(icon, RowHit::Restore(r));
        22.0
    } else {
        0.0
    };

    match row {
        SettingsRow::Switch { id, label, value } => {
            draw_row_label(canvas, theme, fonts, label, rect, 96.0, label_off);
            draw_chip(canvas, fonts, rect.right - 84.0, rect, "开关", theme.moss);
            draw_toggle(canvas, theme, rect, *value);
            push_hit(rect, RowHit::Switch(*id));
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
            draw_row_label(canvas, theme, fonts, label, rect, 236.0, label_off);
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
            push_hit(dec_rect, RowHit::StepperDec(*id));

            // 数值框(可点击进入手动键入 / 光标定位 / 拖选)喵
            let box_rect = Rect::from_xywh(rect.right - 112.0, rect.center_y() - 14.0, 64.0, 28.0);
            let te = edit.stepper.and_then(|(eid, te)| (eid == *id).then_some(te));
            draw_value_box(canvas, theme, fonts, box_rect, *value, unit, te);
            push_hit(box_rect, RowHit::StepperEdit(*id));

            // 步进加号喵
            let inc_rect = Rect::from_xywh(rect.right - 34.0, rect.center_y() - 13.0, 26.0, 26.0);
            draw_control_button(canvas, theme, fonts, inc_rect, "+");
            push_hit(inc_rect, RowHit::StepperInc(*id));

            let _ = (min, max);
        }
        SettingsRow::Label { label, value } => {
            draw_row_label(canvas, theme, fonts, label, rect, 240.0, 0.0);
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
            draw_flat_button(canvas, theme, fonts, btn_rect, label, ButtonTone::for_id(*id, rec));
            push_hit(rect, RowHit::Button(*id));
        }
        SettingsRow::Choice { id, label, value } => {
            let mut btn_rect = rect;
            btn_rect.inset((0.0, 8.0));
            let text = format!("{label} · {value}");
            draw_flat_button(canvas, theme, fonts, btn_rect, &text, ButtonTone::Accent);
            push_hit(rect, RowHit::Choice(*id));
        }
        SettingsRow::Input { id, label, value, placeholder } => {
            draw_row_label(canvas, theme, fonts, label, rect, 240.0, 0.0);
            let box_rect = Rect::from_xywh(rect.right - 220.0, rect.center_y() - 14.0, 220.0, 28.0);
            let path = shape::rounded_rect_path(box_rect, CTRL_RADIUS);
            let mut bg = Paint::default();
            bg.set_color(theme.control_bg);
            bg.set_anti_alias(true);
            canvas.draw_path(&path, &bg);
            // 浏览器行用独立编辑槽喵
            let (editing, draft) = (edit.browser.is_some(), edit.browser);
            let mut border = Paint::default();
            border.set_color(if editing { theme.accent } else { theme.control_border });
            border.set_anti_alias(true);
            border.set_style(PaintStyle::Stroke);
            border.set_stroke_width(if editing { 1.5 } else { 1.0 });
            canvas.draw_path(&path, &border);
            match draft {
                Some(te) => draw_text_edit(
                    canvas,
                    box_rect,
                    &fonts.font(12.0),
                    theme.text,
                    theme.accent,
                    &te.text,
                    Some(te),
                ),
                None => {
                    let shown = if value.is_empty() { placeholder.as_str() } else { value };
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
                }
            }
            push_hit(box_rect, RowHit::Input(*id));
        }
        SettingsRow::Filter {
            index,
            keyword,
            case_sensitive,
        } => {
            // 过滤关键词行: 左侧输入框 + 中部大小写判定(checkbox) + 右侧删除按钮喵
            let input_rect = Rect::from_xywh(
                rect.left,
                rect.center_y() - 14.0,
                (rect.width() - 168.0).max(120.0),
                28.0,
            );
            let input_path = shape::rounded_rect_path(input_rect, CTRL_RADIUS);
            let mut ibg = Paint::default();
            ibg.set_color(theme.control_bg);
            ibg.set_anti_alias(true);
            canvas.draw_path(&input_path, &ibg);
            let editing = edit.filter.is_some_and(|(i, _)| i == *index);
            let mut iborder = Paint::default();
            iborder.set_color(if editing { theme.accent } else { theme.control_border });
            iborder.set_anti_alias(true);
            iborder.set_style(PaintStyle::Stroke);
            iborder.set_stroke_width(if editing { 1.5 } else { 1.0 });
            canvas.draw_path(&input_path, &iborder);
            if let Some((_, te)) = edit.filter.filter(|(i, _)| i == index) {
                draw_text_edit(
                    canvas,
                    input_rect,
                    &fonts.font(12.0),
                    theme.text,
                    theme.accent,
                    &te.text,
                    Some(te),
                );
            } else {
                let shown = if keyword.is_empty() { "输入过滤关键词喵…" } else { keyword };
                let mut ip = Paint::default();
                ip.set_color(if keyword.is_empty() { theme.text_dim } else { theme.text });
                ip.set_anti_alias(true);
                crate::render::text::draw_clipped(
                    canvas,
                    shown,
                    Rect::from_xywh(input_rect.left + 8.0, input_rect.top, input_rect.width() - 12.0, input_rect.height()),
                    &fonts.font(12.0),
                    &ip,
                );
            }
            push_hit(input_rect, RowHit::FilterInput(*index));

            // 大小写判定 checkbox(用内置 SVG) + 说明文字喵
            let check_rect = Rect::from_xywh(rect.right - 132.0, rect.center_y() - 9.0, 18.0, 18.0);
            draw_checkbox(canvas, theme, check_rect, *case_sensitive);
            push_hit(check_rect, RowHit::FilterCase(*index));
            let mut cp = Paint::default();
            cp.set_color(theme.text_dim);
            cp.set_anti_alias(true);
            crate::render::text::draw_clipped(
                canvas,
                "大小写",
                Rect::from_xywh(rect.right - 108.0, rect.top, 50.0, rect.height()),
                &fonts.font(11.0),
                &cp,
            );

            // 删除按钮(内嵌关闭 SVG)喵
            let del_rect = Rect::from_xywh(rect.right - 34.0, rect.center_y() - 13.0, 26.0, 26.0);
            draw_icon_button(canvas, theme, del_rect, "close", theme.text);
            push_hit(del_rect, RowHit::FilterDelete(*index));
        }
        SettingsRow::Command { index, kind, aliases } => {
            // 指令行: 左侧别名输入框 + 中部命令类型按钮 + 右侧删除按钮喵
            let input_rect = Rect::from_xywh(
                rect.left,
                rect.center_y() - 14.0,
                (rect.width() - 178.0).max(120.0),
                28.0,
            );
            let input_path = shape::rounded_rect_path(input_rect, CTRL_RADIUS);
            let mut ibg = Paint::default();
            ibg.set_color(theme.control_bg);
            ibg.set_anti_alias(true);
            canvas.draw_path(&input_path, &ibg);
            let editing = edit.command.is_some_and(|(i, _)| i == *index);
            let mut iborder = Paint::default();
            iborder.set_color(if editing { theme.accent } else { theme.control_border });
            iborder.set_anti_alias(true);
            iborder.set_style(PaintStyle::Stroke);
            iborder.set_stroke_width(if editing { 1.5 } else { 1.0 });
            canvas.draw_path(&input_path, &iborder);
            if let Some((_, te)) = edit.command.filter(|(i, _)| i == index) {
                draw_text_edit(
                    canvas,
                    input_rect,
                    &fonts.font(12.0),
                    theme.text,
                    theme.accent,
                    &te.text,
                    Some(te),
                );
            } else {
                let shown = if aliases.is_empty() { "输入别名,逗号分隔喵…" } else { aliases };
                let mut ip = Paint::default();
                ip.set_color(if aliases.is_empty() { theme.text_dim } else { theme.text });
                ip.set_anti_alias(true);
                crate::render::text::draw_clipped(
                    canvas,
                    shown,
                    Rect::from_xywh(input_rect.left + 8.0, input_rect.top, input_rect.width() - 12.0, input_rect.height()),
                    &fonts.font(12.0),
                    &ip,
                );
            }
            push_hit(input_rect, RowHit::CommandAlias(*index));

            // 命令类型循环按钮(点击在 锁屏/睡眠/关机/重启 间轮换)喵
            let kind_rect = Rect::from_xywh(rect.right - 136.0, rect.center_y() - 14.0, 92.0, 28.0);
            let kind_path = shape::rounded_rect_path(kind_rect, CTRL_RADIUS);
            let mut kbg = Paint::default();
            kbg.set_color(theme.control_bg);
            kbg.set_anti_alias(true);
            canvas.draw_path(&kind_path, &kbg);
            let mut kb = Paint::default();
            kb.set_color(theme.control_border);
            kb.set_anti_alias(true);
            kb.set_style(PaintStyle::Stroke);
            kb.set_stroke_width(1.0);
            canvas.draw_path(&kind_path, &kb);
            let mut kp = Paint::default();
            kp.set_color(theme.text);
            kp.set_anti_alias(true);
            crate::render::text::draw_centered(canvas, kind.title(), kind_rect, &fonts.font(12.0), &kp);
            push_hit(kind_rect, RowHit::CommandKind(*index));

            // 删除按钮(内嵌关闭 SVG)喵
            let del_rect = Rect::from_xywh(rect.right - 34.0, rect.center_y() - 13.0, 26.0, 26.0);
            draw_icon_button(canvas, theme, del_rect, "close", theme.text);
            push_hit(del_rect, RowHit::CommandDelete(*index));
        }
    }
}

/// 按钮色调喵(动作语义决定颜色)喵
enum ButtonTone {
    /// 强调色文字喵
    Accent,
    /// 警示色文字(删除/重置类)喵
    Danger,
    /// 强调色底 + 白字(录制中)喵
    Recording,
}

impl ButtonTone {
    /// 按行身份推断色调喵
    fn for_id(id: RowId, recording: bool) -> Self {
        if recording {
            Self::Recording
        } else if id == RowId::Reset {
            Self::Danger
        } else {
            Self::Accent
        }
    }
}

/// 扁平动作按钮喵(描边底 + 语义色文字)喵
fn draw_flat_button(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    rect: Rect,
    label: &str,
    tone: ButtonTone,
) {
    let path = shape::rounded_rect_path(rect, CTRL_RADIUS);
    let mut bg = Paint::default();
    bg.set_color(match tone {
        ButtonTone::Recording => theme.accent,
        _ => theme.control_bg,
    });
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);

    let font = fonts.font(13.0);
    let mut p = Paint::default();
    p.set_color(match tone {
        ButtonTone::Recording => Color::WHITE,
        ButtonTone::Danger => theme.danger,
        ButtonTone::Accent => theme.accent,
    });
    p.set_anti_alias(true);
    crate::render::text::draw_centered(canvas, label, rect, &font, &p);
}

/// 小图标按钮喵(描边底 + 内嵌 SVG)喵
fn draw_icon_button(
    canvas: &Canvas,
    theme: &SettingsTheme,
    rect: Rect,
    icon_name: &str,
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
    icon::draw_builtin(canvas, rect, icon_name, color);
}

/// 绘制数值框喵: 静态时显示「值+单位」;编辑中显示草稿 + 光标 + 选中态喵
#[allow(clippy::too_many_arguments)]
fn draw_value_box(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    rect: Rect,
    value: i32,
    unit: &str,
    te: Option<&TextEdit>,
) {
    let path = shape::rounded_rect_path(rect, CTRL_RADIUS);
    let selected = te.is_some_and(|t| t.selection().is_some());
    let mut bg = Paint::default();
    bg.set_color(if selected {
        Color::from_argb(0x2E, theme.accent.r(), theme.accent.g(), theme.accent.b())
    } else {
        theme.control_bg
    });
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);

    // 编辑中给强调色描边,标明「正在键入」喵
    let mut border = Paint::default();
    border.set_color(if te.is_some() { theme.accent } else { theme.control_border });
    border.set_anti_alias(true);
    border.set_style(PaintStyle::Stroke);
    border.set_stroke_width(if te.is_some() { 1.5 } else { 1.0 });
    canvas.draw_path(&path, &border);

    match te {
        Some(te) => draw_text_edit(
            canvas,
            rect,
            &fonts.font(12.0),
            theme.text,
            theme.accent,
            &te.text,
            Some(te),
        ),
        None => {
            let font = fonts.font(12.0);
            let mut p = Paint::default();
            p.set_color(theme.text);
            p.set_anti_alias(true);
            crate::render::text::draw_centered(
                canvas,
                &format!("{value} {unit}"),
                Rect::from_xywh(rect.left, rect.top, rect.width(), rect.height()),
                &font,
                &p,
            );
        }
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

/// 矩形内缩(返回新值,Skia 的 inset 是原地修改返回 ())喵
/// 命中区转屏幕坐标(内容区绘制时 canvas 已 -scroll,命中要加回来)喵
fn screen_hit(rect: Rect, scroll: f32) -> Rect {
    Rect::from_xywh(rect.left, rect.top - scroll, rect.width(), rect.height())
}

/// 行内标签喵: `x_offset` 为行首恢复图标预留的缩进喵
#[allow(clippy::too_many_arguments)]
fn draw_row_label(
    canvas: &Canvas,
    theme: &SettingsTheme,
    fonts: &FontCache,
    label: &str,
    rect: Rect,
    right_reserve: f32,
    x_offset: f32,
) {
    let font = fonts.font(13.0);
    let mut p = Paint::default();
    p.set_color(theme.text);
    p.set_anti_alias(true);
    let label_rect = Rect::from_xywh(
        rect.left + x_offset,
        rect.top,
        (rect.width() - x_offset - right_reserve).max(40.0),
        rect.height(),
    );
    crate::render::text::draw_clipped(canvas, label, label_rect, &font, &p);
}

/// 绘制「恢复默认」图标喵: 小圆角底 + 内嵌回滚箭头 SVG 喵
fn draw_restore_icon(canvas: &Canvas, theme: &SettingsTheme, rect: Rect) {
    let path = shape::rounded_rect_path(rect, 6.0);
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
    icon::draw_builtin(canvas, rect, "rollback-arrow", theme.accent);
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

    let f = fonts.font(14.0);
    let mut p = Paint::default();
    p.set_color(theme.text);
    p.set_anti_alias(true);
    crate::render::text::draw_centered(canvas, symbol, rect, &f, &p);
}

/// 绘制 checkbox(大小写判定开关)喵: 用内置 SVG,勾选时强调色喵
fn draw_checkbox(canvas: &Canvas, theme: &SettingsTheme, rect: Rect, checked: bool) {
    if checked {
        icon::draw_builtin(canvas, rect, "checkbox-true", theme.accent);
    } else {
        icon::draw_builtin(canvas, rect, "checkbox-false", theme.control_border);
    }
}