//! 启动器场景绘制喵~
//!
//! 一体灵动岛: 搜索槽与结果面板合成一块超椭圆,视觉走 ink/glass/outline 喵。

use crate::animation::IslandVisual;
use crate::app::{AppState, ListItem};
use crate::render::font::FontCache;
use crate::render::layout::Layout;
use crate::render::theme::Theme;
use crate::render::{shape, text};
use skia_safe::{BlurStyle, Canvas, Color, MaskFilter, Paint, Rect};

/// 绘制完整启动器场景喵
#[allow(clippy::too_many_arguments)]
pub fn paint_scene(
    canvas: &Canvas,
    theme: &Theme,
    layout: &Layout,
    state: &mut AppState,
    fonts: &FontCache,
    caret_on: bool,
    ime_preedit: &str,
) {
    canvas.clear(Color::TRANSPARENT);
    if layout.opacity <= 0.001 {
        return;
    }
    // 透明度接近 1 时不再开离屏图层,省掉整帧合成开销(动画更顺畅)喵
    let count = if layout.opacity < 0.999 {
        let c = canvas.save();
        canvas.save_layer_alpha_f(None, layout.opacity);
        c
    } else {
        canvas.save()
    };

    draw_island(canvas, theme, layout);
    draw_search_content(canvas, theme, layout, state, fonts, caret_on, ime_preedit);
    if layout.panel_height > 0.5 && layout.panel_opacity > 0.02 {
        let c2 = canvas.save();
        canvas.save_layer_alpha_f(None, layout.panel_opacity);
        draw_items(canvas, theme, layout, state, fonts);
        canvas.restore_to_count(c2);
    }

    canvas.restore_to_count(count);
}

fn draw_island(canvas: &Canvas, theme: &Theme, layout: &Layout) {
    let rect = layout.island_rect;
    let radius = layout.radius.min(rect.height() / 2.0).max(0.0);
    let path = shape::rounded_rect_path(rect, radius);

    // 阴影按岛体高度淡化为「轻微落地感」;扩展面板大时进一步减弱,
    // 避免整圈辉光包围扩展窗喵
    let base = theme.shadow;
    let factor = if rect.height() > 90.0 { 0.35 } else { 0.7 };
    let shadow = Color::from_argb(
        (base.a() as f32 * factor).round().clamp(0.0, 255.0) as u8,
        base.r(),
        base.g(),
        base.b(),
    );
    draw_shadow(canvas, &path, rect, shadow, 9.0, 5.0);

    let mut fill = Paint::default();
    fill.set_color(theme.fill);
    fill.set_anti_alias(true);
    canvas.draw_path(&path, &fill);

    // 玻璃材质加细噪点;顶部高光横线已移除,避免透明主题下突兀的亮线喵
    if theme.visual == IslandVisual::Glass {
        draw_noise(canvas, rect, 5, 26);
    }

    let mut stroke = Paint::default();
    stroke.set_color(if layout.hit {
        theme.accent
    } else {
        theme.border
    });
    stroke.set_anti_alias(true);
    stroke.set_style(skia_safe::PaintStyle::Stroke);
    stroke.set_stroke_width(if theme.visual == IslandVisual::Outline {
        1.5
    } else {
        1.25
    });
    canvas.draw_path(&path, &stroke);
}

fn draw_noise(canvas: &Canvas, rect: Rect, density: usize, alpha: u8) {
    let mut p = Paint::default();
    p.set_anti_alias(false);
    let w = rect.width() as i32;
    let h = rect.height() as i32;
    for y in (0..h).step_by(density) {
        for x in (0..w).step_by(density) {
            let n = hash32(x as u32, y as u32);
            if n & 3 != 0 {
                continue;
            }
            p.set_color(if n & 1 == 1 {
                Color::from_argb(alpha, 255, 255, 255)
            } else {
                Color::from_argb(alpha, 0, 0, 0)
            });
            canvas.draw_point((rect.left + x as f32, rect.top + y as f32), &p);
        }
    }
}

fn hash32(x: u32, y: u32) -> u32 {
    x.wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263))
        >> 24
}

fn draw_search_content(
    canvas: &Canvas,
    theme: &Theme,
    layout: &Layout,
    state: &AppState,
    fonts: &FontCache,
    caret_on: bool,
    ime_preedit: &str,
) {
    let rect = layout.search_rect;
    if rect.width() < 8.0 || rect.height() < 8.0 {
        return;
    }
    let icon_size = rect.height() * 0.36;
    let icon_cx = rect.left + rect.height() * 0.55;
    let icon_cy = rect.center_y();
    draw_search_icon(canvas, icon_cx, icon_cy, icon_size, theme.text_dim);

    let text_x = rect.left + rect.height() * 0.95;
    let font = fonts.font(rect.height() * 0.38);
    let text_rect = Rect::from_xywh(
        text_x,
        rect.top,
        (rect.width() - (text_x - rect.left) - rect.height() * 0.4).max(8.0),
        rect.height(),
    );

    let shown = if ime_preedit.is_empty() {
        state.query.clone()
    } else {
        format!("{}{}", state.query, ime_preedit)
    };

    if shown.is_empty() {
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
        let mut p = Paint::default();
        p.set_color(theme.text);
        p.set_anti_alias(true);
        let caret_x = text::draw_clipped(canvas, &shown, text_rect, &font, &p);
        if !ime_preedit.is_empty() {
            let mut up = Paint::default();
            up.set_color(theme.accent);
            up.set_anti_alias(true);
            up.set_stroke_width(1.0);
            canvas.draw_line(
                (
                    caret_x - font.size() * ime_preedit.chars().count() as f32 * 0.55,
                    text_rect.center_y() + font.size() * 0.5,
                ),
                (caret_x, text_rect.center_y() + font.size() * 0.5),
                &up,
            );
        }
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

fn draw_items(
    canvas: &Canvas,
    theme: &Theme,
    layout: &Layout,
    state: &mut AppState,
    fonts: &FontCache,
) {
    let path = shape::rounded_rect_path(layout.island_rect, layout.radius);
    canvas.save();
    canvas.clip_path(&path, None, Some(false));
    canvas.clip_rect(layout.panel_rect, None, Some(false));

    let selected = state.selected;
    for (i, item_rect) in layout.item_rects.iter().enumerate() {
        if item_rect.bottom < layout.panel_rect.top || item_rect.top > layout.panel_rect.bottom {
            continue;
        }
        match state.results.get(i) {
            Some(ListItem::Section(title)) => draw_section(canvas, theme, item_rect, title, fonts),
            Some(ListItem::App(app)) => {
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
            None => {}
        }
    }
    canvas.restore();
}

fn draw_section(canvas: &Canvas, theme: &Theme, rect: &Rect, title: &str, fonts: &FontCache) {
    let font = fonts.font(rect.height() * 0.32);
    let mut p = Paint::default();
    p.set_color(theme.text_dim);
    p.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        title,
        Rect::from_xywh(rect.left + 12.0, rect.top, rect.width() - 16.0, rect.height()),
        &font,
        &p,
    );
}

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
    if selected {
        let bg_path = shape::rounded_rect_path(*rect, 8.0);
        let mut bg = Paint::default();
        bg.set_color(theme.hover_bg);
        bg.set_anti_alias(true);
        canvas.draw_path(&bg_path, &bg);

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

    let text_x = icon_rect.right + 12.0;
    let name_font = fonts.font(rect.height() * 0.38);
    let path_font = fonts.font(rect.height() * 0.26);

    let mut name_paint = Paint::default();
    name_paint.set_color(theme.text);
    name_paint.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        &app.name,
        Rect::from_xywh(text_x, rect.top, rect.right - text_x - 8.0, rect.height() * 0.58),
        &name_font,
        &name_paint,
    );

    let mut path_paint = Paint::default();
    path_paint.set_color(theme.text_dim);
    path_paint.set_anti_alias(true);
    let subtitle = if app.tags.is_empty() {
        app.path.clone()
    } else {
        format!("{}  ·  {}", app.tags.join(" / "), app.path)
    };
    text::draw_clipped(
        canvas,
        &subtitle,
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

fn draw_icon(canvas: &Canvas, image: &skia_safe::Image, dst: Rect) {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    canvas.draw_image_rect(image, None, dst, &paint);
}

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

/// 画岛下投影喵: 裁掉顶部上方的光斑,避免「重影」错觉喵
///
/// 旧实现把整条模糊路径沿 +y 移 5px,其上半圈光晕会盖在岛顶上方,
/// 动画时看起来像岛上方有一团暗色重影喵。这里把投影裁剪到岛体下方,
/// 保留底部与侧边的柔和辉光,顶部以上一律不画喵。
fn draw_shadow(canvas: &Canvas, path: &skia_safe::Path, rect: Rect, color: Color, blur: f32, dy: f32) {
    canvas.save();
    let clip = Rect::from_xywh(
        rect.left - blur,
        rect.top,
        rect.width() + blur * 2.0,
        rect.height() + blur + dy,
    );
    canvas.clip_rect(clip, None, Some(false));
    canvas.translate((0.0, dy));
    let mut p = Paint::default();
    p.set_color(color);
    p.set_anti_alias(true);
    p.set_mask_filter(MaskFilter::blur(BlurStyle::Normal, blur, None));
    canvas.draw_path(path, &p);
    canvas.restore();
}

/// 绘制搜索图标喵(圆心对齐整数像素,细环描边更锐利)喵
fn draw_search_icon(canvas: &Canvas, cx: f32, cy: f32, size: f32, color: Color) {
    let cx = cx.round();
    let cy = cy.round();
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.set_anti_alias(true);
    paint.set_style(skia_safe::PaintStyle::Stroke);
    paint.set_stroke_width(size * 0.14);

    let r = size * 0.34;
    canvas.draw_circle((cx - size * 0.08, cy - size * 0.08), r, &paint);
    canvas.draw_line(
        (cx + r * 0.65 - size * 0.08, cy + r * 0.65 - size * 0.08),
        (cx + size * 0.45, cy + size * 0.45),
        &paint,
    );
}
