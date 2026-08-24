//! 场景绘制喵~
//!
//! 把启动器的完整视觉(搜索框胶囊 + 结果面板 + 条目)绘制到 Skia Canvas 上喵。
//! 遵循 window-design skill 的层级纪律: 背景 → 内容 → 描边分离绘制喵。
//! 所有色值走主题 token,尺寸走 Layout,动画值由弹簧驱动,这里只做「数值翻译」喵。

use crate::app::AppState;
use crate::render::font::FontCache;
use crate::render::layout::Layout;
use crate::render::theme::Theme;
use crate::render::{shape, text};
use skia_safe::{BlurStyle, Canvas, Color, MaskFilter, Paint, Rect};

/// 面板圆角(物理 px 基准,由 scale 传入已缩放值)喵
const PANEL_RADIUS: f32 = 12.0;

/// 绘制完整启动器场景喵
///
/// * `alpha` - 整体透明度(呼出/隐藏淡入淡出)喵
/// * `caret_on` - 输入光标是否可见(闪烁)喵
pub fn paint_scene(
    canvas: &Canvas,
    theme: &Theme,
    layout: &Layout,
    state: &mut AppState,
    fonts: &FontCache,
    alpha: f32,
    caret_on: bool,
) {
    // 透明背景 + 整体透明度喵
    canvas.clear(Color::TRANSPARENT);
    if alpha <= 0.001 {
        return;
    }
    let count = canvas.save();
    canvas.save_layer_alpha_f(None, alpha);

    // 1) 结果面板(先画,搜索框叠在其上)喵
    if layout.panel_height > 0.5 {
        draw_panel(canvas, theme, layout, state, fonts);
    }

    // 2) 搜索框胶囊喵
    draw_search_bar(canvas, theme, layout, state, fonts, caret_on);

    canvas.restore_to_count(count);
}

/// 绘制搜索框胶囊喵
fn draw_search_bar(
    canvas: &Canvas,
    theme: &Theme,
    layout: &Layout,
    state: &AppState,
    fonts: &FontCache,
    caret_on: bool,
) {
    let rect = layout.search_rect;
    let path = shape::capsule_path(rect);

    // 阴影喵
    draw_shadow(canvas, &path, theme.shadow, 10.0, 3.0);

    // 背景 + 描边喵
    let mut fill = Paint::default();
    fill.set_color(theme.background);
    fill.set_anti_alias(true);
    canvas.draw_path(&path, &fill);

    let mut stroke = Paint::default();
    stroke.set_color(theme.border);
    stroke.set_anti_alias(true);
    stroke.set_style(skia_safe::PaintStyle::Stroke);
    stroke.set_stroke_width(1.0);
    canvas.draw_path(&path, &stroke);

    // 内容: 搜索图标 + 文字 + 光标喵
    let icon_size = rect.height() * 0.36;
    let icon_cx = rect.left + rect.height() * 0.55;
    let icon_cy = rect.center_y();
    draw_search_icon(canvas, icon_cx, icon_cy, icon_size, theme.text_dim);

    let text_x = rect.left + rect.height() * 0.95;
    let font = fonts.font(rect.height() * 0.42);
    let text_rect = Rect::from_xywh(
        text_x,
        rect.top,
        rect.width() - (text_x - rect.left) - rect.height() * 0.4,
        rect.height(),
    );

    if state.query.is_empty() {
        // 占位提示喵
        let mut p = Paint::default();
        p.set_color(theme.text_dim);
        p.set_anti_alias(true);
        text::draw_clipped(
            canvas,
            "搜索应用喵... (t:标签 / i:首字母)",
            text_rect,
            &font,
            &p,
        );
    } else {
        // 查询文字 + 光标喵
        let mut p = Paint::default();
        p.set_color(theme.text);
        p.set_anti_alias(true);
        let caret_x = text::draw_clipped(canvas, &state.query, text_rect, &font, &p);
        if caret_on {
            let mut cp = Paint::default();
            cp.set_color(theme.accent);
            cp.set_anti_alias(true);
            let cx = caret_x.min(text_rect.right - 2.0);
            canvas.draw_line(
                (cx, text_rect.center_y() - font.size() * 0.45),
                (cx, text_rect.center_y() + font.size() * 0.45),
                &cp,
            );
        }
    }
}

/// 绘制结果面板与条目喵
fn draw_panel(
    canvas: &Canvas,
    theme: &Theme,
    layout: &Layout,
    state: &mut AppState,
    fonts: &FontCache,
) {
    let rect = layout.panel_rect;
    let path = shape::rounded_rect_path(rect, PANEL_RADIUS);

    // 阴影喵
    draw_shadow(canvas, &path, theme.shadow, 16.0, 5.0);

    // 背景 + 描边喵
    let mut fill = Paint::default();
    fill.set_color(theme.background);
    fill.set_anti_alias(true);
    canvas.draw_path(&path, &fill);

    let mut stroke = Paint::default();
    stroke.set_color(theme.border);
    stroke.set_anti_alias(true);
    stroke.set_style(skia_safe::PaintStyle::Stroke);
    stroke.set_stroke_width(1.0);
    canvas.draw_path(&path, &stroke);

    // 裁剪到面板内,条目随面板高度展开而显现喵
    canvas.save();
    canvas.clip_path(&path, None, Some(false));

    // 条目(按需绘制,超出面板高度的部分被裁剪)喵
    let selected = state.selected;
    for (i, item_rect) in layout.item_rects.iter().enumerate() {
        if let Some(app) = state.results.get(i) {
            let icon = state.icons.cached_image(app);
            draw_item(
                canvas,
                theme,
                item_rect,
                app,
                icon,
                i == selected,
                layout,
                fonts,
            );
        }
    }

    canvas.restore();
}

/// 绘制单个应用条目喵
#[allow(clippy::too_many_arguments)]
fn draw_item(
    canvas: &Canvas,
    theme: &Theme,
    rect: &Rect,
    app: &crate::apps::AppInfo,
    icon: Option<skia_safe::Image>,
    selected: bool,
    layout: &Layout,
    fonts: &FontCache,
) {
    // 选中背景喵
    if selected {
        let bg_path = shape::rounded_rect_path(*rect, 8.0);
        let mut bg = Paint::default();
        bg.set_color(theme.hover_bg);
        bg.set_anti_alias(true);
        canvas.draw_path(&bg_path, &bg);

        // 左侧选中指示条喵
        let bar_w = 3.0;
        let bar = Rect::from_xywh(
            rect.left + 4.0,
            rect.center_y() - rect.height() * 0.3,
            bar_w,
            rect.height() * 0.6,
        );
        let bar_path = shape::rounded_rect_path(bar, bar_w / 2.0);
        let mut accent = Paint::default();
        accent.set_color(theme.accent);
        accent.set_anti_alias(true);
        canvas.draw_path(&bar_path, &accent);
    }

    // 图标喵
    let icon_size = layout.icon_size;
    let icon_rect = Rect::from_xywh(
        rect.left + 12.0,
        rect.center_y() - icon_size / 2.0,
        icon_size,
        icon_size,
    );
    match icon {
        Some(img) => draw_icon(canvas, &img, icon_rect),
        None => draw_fallback_icon(canvas, theme, icon_rect, &app.name, fonts),
    }

    // 名称 + 路径喵
    let text_x = icon_rect.right + 12.0;
    let name_font = fonts.font(rect.height() * 0.38);
    let path_font = fonts.font(rect.height() * 0.26);

    let mut name_paint = Paint::default();
    name_paint.set_color(theme.text);
    name_paint.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        &app.name,
        Rect::from_xywh(
            text_x,
            rect.top,
            rect.right - text_x - 8.0,
            rect.height() * 0.58,
        ),
        &name_font,
        &name_paint,
    );

    let mut path_paint = Paint::default();
    path_paint.set_color(theme.text_dim);
    path_paint.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        &app.path,
        Rect::from_xywh(
            text_x,
            rect.top + rect.height() * 0.56,
            rect.right - text_x - 8.0,
            rect.height() * 0.4,
        ),
        &path_font,
        &path_paint,
    );
}

/// 绘制缩放后的图标喵
fn draw_icon(canvas: &Canvas, image: &skia_safe::Image, dst: Rect) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    canvas.draw_image_rect(image, None, dst, &paint);
}

/// 兜底图标: 品牌色圆角背景 + 应用名前两字符喵
fn draw_fallback_icon(canvas: &Canvas, theme: &Theme, rect: Rect, name: &str, fonts: &FontCache) {
    let path = shape::rounded_rect_path(rect, rect.width() * 0.22);
    let mut bg = Paint::default();
    bg.set_color(theme.accent);
    bg.set_anti_alias(true);
    canvas.draw_path(&path, &bg);

    let chars: String = name.chars().take(2).collect();
    let label = if chars.is_empty() {
        "?".to_string()
    } else {
        chars
    };
    let font = fonts.font(rect.width() * 0.38);
    let mut p = Paint::default();
    p.set_color(theme.accent_text);
    p.set_anti_alias(true);
    text::draw_centered(canvas, &label, rect, &font, &p);
}

/// 绘制圆角矩形的柔和阴影喵
fn draw_shadow(canvas: &Canvas, path: &skia_safe::Path, color: Color, blur: f32, dy: f32) {
    canvas.save();
    canvas.translate((0.0, dy));
    let mut p = Paint::default();
    p.set_color(color);
    p.set_anti_alias(true);
    p.set_mask_filter(MaskFilter::blur(BlurStyle::Normal, blur, None));
    canvas.draw_path(path, &p);
    canvas.restore();
}

/// 绘制放大镜搜索图标(纯矢量,不依赖 emoji 字体)喵
fn draw_search_icon(canvas: &Canvas, cx: f32, cy: f32, size: f32, color: Color) {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    paint.set_style(skia_safe::PaintStyle::Stroke);
    paint.set_stroke_width(size * 0.14);

    let r = size * 0.34;
    // 镜圈喵
    canvas.draw_circle((cx - size * 0.08, cy - size * 0.08), r, &paint);
    // 手柄喵
    canvas.draw_line(
        (cx + r * 0.65 - size * 0.08, cy + r * 0.65 - size * 0.08),
        (cx + size * 0.45, cy + size * 0.45),
        &paint,
    );
}
