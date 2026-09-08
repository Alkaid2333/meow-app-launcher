//! 启动器场景绘制喵~
//!
//! 一体灵动岛: 搜索槽与结果面板合成一块连续曲率圆角,视觉随主题预设喵。

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
    caret: usize,
    selection: Option<(usize, usize)>,
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
    draw_search_content(
        canvas,
        theme,
        layout,
        state,
        fonts,
        caret_on,
        ime_preedit,
        caret,
        selection,
    );
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

    // 毛玻璃材质加细噪点;文字色与底材在主题预设内成对出现,对比有保证喵
    if theme.glass {
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
    stroke.set_stroke_width(1.25);
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

/// 绘制搜索槽内容喵(文本 + 选中态 + IME 下划线 + 光标)喵
#[allow(clippy::too_many_arguments)]
fn draw_search_content(
    canvas: &Canvas,
    theme: &Theme,
    layout: &Layout,
    state: &AppState,
    fonts: &FontCache,
    caret_on: bool,
    ime_preedit: &str,
    caret: usize,
    selection: Option<(usize, usize)>,
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
        // 聚焦且空文本: 在起点闪烁光标,提示可输入喵
        if caret_on {
            let mut cp = Paint::default();
            cp.set_color(theme.accent);
            cp.set_anti_alias(true);
            let cx = text_x.round().clamp(text_rect.left, text_rect.right - 2.0);
            canvas.draw_line(
                (cx, text_rect.center_y() - font.size() * 0.45),
                (cx, text_rect.center_y() + font.size() * 0.45),
                &cp,
            );
        }
    } else {
        let query = &state.query;
        let mut p = Paint::default();
        p.set_color(theme.text);
        p.set_anti_alias(true);
        // 文字实际落点与坐标换算统一起点(与 hit-test 一致,避免光标/选点漂移)喵
        let start_x = text_x.round();

        // 选中区背景(强调色淡底,比 hover_bg 更明显;IME 组字期间不画选中)喵
        if ime_preedit.is_empty()
            && let Some((lo, hi)) = selection
        {
            let lo = lo.min(query.len());
            let hi = hi.min(query.len());
            if lo < hi {
                let (w0, _) = font.measure_str(&query[..lo], Some(&p));
                let (w1, _) = font.measure_str(&query[..hi], Some(&p));
                let sel = Rect::from_xywh(
                    start_x + w0,
                    text_rect.top + 2.0,
                    (w1 - w0).max(1.0),
                    (text_rect.height() - 4.0).max(1.0),
                );
                let a = theme.accent;
                let mut bg = Paint::default();
                bg.set_color(Color::from_argb(0x30, a.r(), a.g(), a.b()));
                bg.set_anti_alias(true);
                let path = shape::rounded_rect_path(sel, 3.0);
                canvas.save();
                canvas.clip_rect(text_rect, None, Some(false));
                canvas.draw_path(&path, &bg);
                canvas.restore();
            }
        }

        let end_x = text::draw_clipped(canvas, &shown, text_rect, &font, &p);
        if !ime_preedit.is_empty() {
            let mut up = Paint::default();
            up.set_color(theme.accent);
            up.set_anti_alias(true);
            up.set_stroke_width(1.0);
            canvas.draw_line(
                (
                    end_x - font.size() * ime_preedit.chars().count() as f32 * 0.55,
                    text_rect.center_y() + font.size() * 0.5,
                ),
                (end_x, text_rect.center_y() + font.size() * 0.5),
                &up,
            );
        }
        if caret_on {
            // 光标位置: 组字时贴文末,平时贴在插入点(命中测试算出的字节位置)喵
            let caret = if ime_preedit.is_empty() {
                caret.min(query.len())
            } else {
                query.len()
            };
            let (w, _) = font.measure_str(&query[..caret], Some(&p));
            let mut cp = Paint::default();
            cp.set_color(theme.accent);
            cp.set_anti_alias(true);
            let cx = (start_x + w).clamp(text_rect.left, text_rect.right - 2.0);
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
    let grid = state.config.window.layout == crate::app::config::AppLayout::Grid;
    for (i, item_rect) in layout.item_rects.iter().enumerate() {
        if item_rect.bottom < layout.panel_rect.top || item_rect.top > layout.panel_rect.bottom {
            continue;
        }
        match state.results.get(i) {
            Some(ListItem::Section(title)) => draw_section(canvas, theme, item_rect, title, fonts),
            Some(ListItem::Item(item)) => {
                // 应用走缓存图标,内置图标在 draw_item 里用 SVG 画喵
                let icon = match &item.icon {
                    crate::search::ItemIcon::App(app) => state.icons.cached_image(app),
                    crate::search::ItemIcon::Builtin(_) => None,
                };
                draw_item(
                    canvas,
                    theme,
                    item_rect,
                    item,
                    icon,
                    i == selected,
                    layout,
                    fonts,
                    grid,
                );
            }
            None => {}
        }
    }
    canvas.restore();

    // 滚动条(内容溢出时出现)喵
    if let Some((track, thumb)) = layout.scrollbar {
        draw_scrollbar(canvas, theme, track, thumb);
    }
}

/// 绘制面板滚动条喵: 细轨道 + 强调色圆角滑块,贴合面板右缘喵
fn draw_scrollbar(canvas: &Canvas, theme: &Theme, track_rect: Rect, thumb_rect: Rect) {
    let track_path = shape::rounded_rect_path(track_rect, track_rect.width() / 2.0);
    let mut track = Paint::default();
    track.set_color(theme.hover_bg);
    track.set_anti_alias(true);
    canvas.draw_path(&track_path, &track);

    let thumb_path = shape::rounded_rect_path(thumb_rect, thumb_rect.width() / 2.0);
    let mut thumb = Paint::default();
    thumb.set_color(theme.accent);
    thumb.set_anti_alias(true);
    canvas.draw_path(&thumb_path, &thumb);
}

fn draw_section(canvas: &Canvas, theme: &Theme, rect: &Rect, title: &str, fonts: &FontCache) {
    // 网格分组的行高更矮,字号保底 10px 保证可读喵
    let font = fonts.font((rect.height() * 0.32).max(10.0));
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
    item: &crate::search::SearchItem,
    icon: Option<skia_safe::Image>,
    selected: bool,
    layout: &Layout,
    fonts: &FontCache,
    grid: bool,
) {
    // 网格排版: 单元格卡片式绘制(图标居中上排 + 名称居中)喵
    if grid {
        return draw_grid_item(canvas, theme, rect, item, icon, selected, layout, fonts);
    }

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
    // 图标统一走 icon 层: 光栅直出,无图时按源类型选 SVG 或字符兜底喵
    let source = match icon {
        Some(img) => super::icon::IconSource::Raster(img),
        None => match &item.icon {
            crate::search::ItemIcon::App(_) => {
                super::icon::IconSource::Fallback(item.fallback_label())
            }
            crate::search::ItemIcon::Builtin(bi) => {
                super::icon::IconSource::Builtin(bi.svg_name())
            }
        },
    };
    super::icon::draw(canvas, theme, icon_rect, source, fonts);

    let text_x = icon_rect.right + 12.0;
    let name_font = fonts.font(rect.height() * 0.38);
    let path_font = fonts.font(rect.height() * 0.26);

    let mut name_paint = Paint::default();
    name_paint.set_color(theme.text);
    name_paint.set_anti_alias(true);
    text::draw_clipped(
        canvas,
        &item.title,
        Rect::from_xywh(text_x, rect.top, rect.right - text_x - 8.0, rect.height() * 0.58),
        &name_font,
        &name_paint,
    );

    let mut path_paint = Paint::default();
    path_paint.set_color(theme.text_dim);
    path_paint.set_anti_alias(true);
    let subtitle = item.subtitle.clone().unwrap_or_default();
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

/// 绘制网格单元格喵: 卡片底 + 居中图标 + 居中名称(隐藏副标题)喵
#[allow(clippy::too_many_arguments)]
fn draw_grid_item(
    canvas: &Canvas,
    theme: &Theme,
    rect: &Rect,
    item: &crate::search::SearchItem,
    icon: Option<skia_safe::Image>,
    selected: bool,
    layout: &Layout,
    fonts: &FontCache,
) {
    // 选中: 圆角卡片底 + 强调色描边喵
    let bg_path = shape::rounded_rect_path(*rect, 10.0);
    if selected {
        let mut bg = Paint::default();
        bg.set_color(theme.hover_bg);
        bg.set_anti_alias(true);
        canvas.draw_path(&bg_path, &bg);
        let mut sel = Paint::default();
        sel.set_color(theme.accent);
        sel.set_anti_alias(true);
        sel.set_style(skia_safe::PaintStyle::Stroke);
        sel.set_stroke_width(1.5);
        canvas.draw_path(&bg_path, &sel);
    }

    // 图标居中于单元上段,尺寸上限 0.38 倍高,保证图标完全落在聚焦框内喵
    let icon_size = layout.icon_size.min(rect.height() * 0.38).max(24.0);
    let icon_cy = rect.top + rect.height() * 0.20;
    let icon_rect = Rect::from_xywh(
        rect.center_x() - icon_size / 2.0,
        icon_cy - icon_size / 2.0,
        icon_size,
        icon_size,
    );
    // 图标统一走 icon 层喵(网格与列表同一套缩放标准)喵
    let source = match icon {
        Some(img) => super::icon::IconSource::Raster(img),
        None => match &item.icon {
            crate::search::ItemIcon::App(_) => {
                super::icon::IconSource::Fallback(item.fallback_label())
            }
            crate::search::ItemIcon::Builtin(bi) => {
                super::icon::IconSource::Builtin(bi.svg_name())
            }
        },
    };
    super::icon::draw(canvas, theme, icon_rect, source, fonts);

    // 名称左对齐渲染(长名称只从右侧裁剪,不再左右同时截断)喵
    let name_font = fonts.font((rect.height() * 0.16).max(10.0));
    let mut np = Paint::default();
    np.set_color(theme.text);
    np.set_anti_alias(true);
    let name_rect = Rect::from_xywh(
        rect.left + 4.0,
        icon_rect.bottom + 3.0,
        (rect.width() - 8.0).max(4.0),
        rect.bottom - icon_rect.bottom - 3.0,
    );
    canvas.save();
    canvas.clip_rect(name_rect, None, Some(false));
    text::draw_clipped(canvas, &item.title, name_rect, &name_font, &np);
    canvas.restore();
}

/// 绘制搜索图标喵(统一图标层的内置放大镜,随主题配色)喵
fn draw_search_icon(canvas: &Canvas, cx: f32, cy: f32, size: f32, color: Color) {
    let rect = Rect::from_xywh(cx - size / 2.0, cy - size / 2.0, size, size);
    super::icon::draw_builtin(canvas, rect, "magnifying-glass", color);
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
