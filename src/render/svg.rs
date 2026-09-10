//! 极简内嵌 SVG 图标渲染喵~
//!
//! `assets/icons/` 下的 24×24 小图标素材,运行时经 `include_str!` 打进二进制,
//! 不需要 skia `svg` feature(避免换预编译二进制),只解析用到的子集:
//! `<path d>` 走 Skia `Path::from_svg`;另解析 `<line>` / `<circle>` / `<rect rx>` / `<polyline>` 喵。

use skia_safe::{Canvas, Color, Paint, PaintStyle, Path, PathBuilder, Rect};
use std::sync::OnceLock;

/// 内置图标名 → 内嵌 SVG 源喵
pub const ICONS: [(&str, &str); 10] = [
    (
        "magnifying-glass",
        include_str!("../../assets/icons/magnifying-glass.svg"),
    ),
    ("settings", include_str!("../../assets/icons/settings.svg")),
    ("close", include_str!("../../assets/icons/close.svg")),
    ("info", include_str!("../../assets/icons/info.svg")),
    (
        "rollback-arrow",
        include_str!("../../assets/icons/rollback-arrow.svg"),
    ),
    (
        "checkbox-true",
        include_str!("../../assets/icons/checkbox-ture.svg"),
    ),
    (
        "checkbox-false",
        include_str!("../../assets/icons/checkbox-false.svg"),
    ),
    (
        "calculator",
        include_str!("../../assets/icons/calculator.svg"),
    ),
    ("web", include_str!("../../assets/icons/web.svg")),
    ("power", include_str!("../../assets/icons/power.svg")),
];

/// 解析后的图标几何喵
#[derive(Debug)]
pub struct SvgIcon {
    /// viewBox 喵(min_x, min_y, width, height),默认 24×24 画布喵
    pub view: (f32, f32, f32, f32),
    shapes: Vec<Shape>,
}

/// 单个可绘制元素喵(元素自带风格,支持线稿/填充混排)喵
#[derive(Debug)]
struct Shape {
    kind: ShapeKind,
    style: IconStyle,
}

/// 图标绘制风格喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconStyle {
    /// 描边线稿喵(圆头圆角,线宽随缩放)喵
    Stroke,
    /// 实心填充喵
    Fill,
}

#[derive(Debug)]
enum ShapeKind {
    /// SVG path 数据喵
    Path(String),
    Line { x1: f32, y1: f32, x2: f32, y2: f32 },
    Circle { cx: f32, cy: f32, r: f32 },
    Rect { x: f32, y: f32, w: f32, h: f32, r: f32 },
    Polyline(Vec<(f32, f32)>),
}

/// 取内置图标(惰性解析 + 缓存)喵
pub fn icon(name: &str) -> &'static SvgIcon {
    static CACHE: [OnceLock<SvgIcon>; 10] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    let idx = ICONS.iter().position(|(n, _)| *n == name).unwrap_or(0);
    CACHE[idx].get_or_init(|| parse_svg(ICONS[idx].1))
}

impl SvgIcon {
    /// 把图标画到目标矩形内喵(viewBox 等比缩放 + 居中,线稿/填充逐元素生效)喵
    pub fn draw(&self, canvas: &Canvas, rect: Rect, color: Color) {
        let (minx, miny, vw, vh) = self.view;
        let (vw, vh) = (vw.max(1.0), vh.max(1.0));
        // viewBox → 目标矩形的等比缩放,完整可见喵
        let scale = (rect.width() / vw).min(rect.height() / vh);
        // 平移取整,避免次像素模糊喵
        let dx = (rect.left + (rect.width() - vw * scale) / 2.0 - minx * scale).round();
        let dy = (rect.top + (rect.height() - vh * scale) / 2.0 - miny * scale).round();
        canvas.save();
        canvas.translate((dx, dy));
        canvas.scale((scale, scale));
        for shape in &self.shapes {
            let mut p = Paint::default();
            p.set_color(color);
            p.set_anti_alias(true);
            match shape.style {
                IconStyle::Stroke => {
                    p.set_style(PaintStyle::Stroke);
                    // 线宽按 viewBox 单位计,缩放后视觉一致喵
                    p.set_stroke_width(2.0);
                    p.set_stroke_cap(skia_safe::PaintCap::Round);
                    p.set_stroke_join(skia_safe::PaintJoin::Round);
                }
                IconStyle::Fill => {
                    p.set_style(PaintStyle::Fill);
                }
            }
            match &shape.kind {
                ShapeKind::Path(d) => {
                    let path = build_path(d);
                    canvas.draw_path(&path, &p);
                }
                ShapeKind::Line { x1, y1, x2, y2 } => {
                    canvas.draw_line((*x1, *y1), (*x2, *y2), &p);
                }
                ShapeKind::Circle { cx, cy, r } => {
                    canvas.draw_circle((*cx, *cy), (*r).max(0.0), &p);
                }
                ShapeKind::Rect { x, y, w, h, r } => {
                    let rrect = Rect::from_xywh(*x, *y, *w, *h);
                    if shape.style == IconStyle::Stroke {
                        let path = rounded_stroke_path(rrect, *r);
                        canvas.draw_path(&path, &p);
                    } else {
                        canvas.draw_rect(rrect, &p);
                    }
                }
                ShapeKind::Polyline(pts) => {
                    let mut b = PathBuilder::new();
                    if let Some((x, y)) = pts.first() {
                        b.move_to((*x, *y));
                        for (x, y) in pts.iter().skip(1) {
                            b.line_to((*x, *y));
                        }
                    }
                    let path = b.snapshot();
                    canvas.draw_path(&path, &p);
                }
            }
        }
        canvas.restore();
    }
}

/// 圆角矩形描边路径喵
fn rounded_stroke_path(rect: Rect, r: f32) -> Path {
    let r = r.clamp(0.0, rect.width().min(rect.height()) / 2.0);
    let mut b = PathBuilder::new();
    b.move_to((rect.left + r, rect.top));
    b.line_to((rect.right - r, rect.top));
    b.line_to((rect.right, rect.top + r));
    b.line_to((rect.right, rect.bottom - r));
    b.line_to((rect.right - r, rect.bottom));
    b.line_to((rect.left + r, rect.bottom));
    b.line_to((rect.left, rect.bottom - r));
    b.line_to((rect.left, rect.top + r));
    b.line_to((rect.left + r, rect.top));
    b.snapshot()
}

// ---------------------------------------------------------------------------
// 简易 SVG 解析喵
// ---------------------------------------------------------------------------

/// 解析内嵌 SVG 字符串喵(渲染层内部用,公开便于单测)喵
///
/// 支持: viewBox 画布映射(任意尺寸)、逐元素 fill/stroke 风格判定、
/// path/line/circle/rect/polyline 子集喵。无属性元素默认按线稿处理喵。
pub fn parse_svg(xml: &str) -> SvgIcon {
    let mut shapes = Vec::new();
    let mut view = (0.0_f32, 0.0_f32, 24.0_f32, 24.0_f32);

    // 逐元素提取 <tag .../> 与 <tag ...>...</tag> 喵
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        let end = rest[start..]
            .find('>')
            .map(|i| start + i + 1)
            .unwrap_or(rest.len());
        let tag = &rest[start..end];
        let name = tag[1..]
            .trim_end_matches('/')
            .trim()
            .split(|c: char| c.is_whitespace())
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if name == "svg" {
            // 根元素: 读 viewBox 画布喵
            if let Some(vb) = attr(tag, "viewBox") {
                let nums = numbers(&vb);
                if nums.len() >= 4 && nums[2] > 0.0 && nums[3] > 0.0 {
                    view = (nums[0], nums[1], nums[2], nums[3]);
                }
            }
        } else if let Some((style, kind)) = parse_element(tag) {
            shapes.push(Shape { kind, style });
        }
        rest = &rest[end..];
    }
    SvgIcon { view, shapes }
}

/// 判定单个元素的绘制风格喵: 显式 stroke → 线稿,显式 fill → 填充,否则线稿喵
fn element_style(tag_lower: &str) -> IconStyle {
    let stroke_none = tag_lower.contains("stroke=\"none\"")
        || tag_lower.contains("stroke='none'")
        || tag_lower.contains("stroke:none");
    let has_stroke = tag_lower.contains("stroke=") && !stroke_none;
    let fill_none = tag_lower.contains("fill=\"none\"")
        || tag_lower.contains("fill='none'")
        || tag_lower.contains("fill:none");
    let has_fill = (tag_lower.contains("fill=") || tag_lower.contains("fill:")) && !fill_none;
    if has_stroke {
        IconStyle::Stroke
    } else if has_fill {
        IconStyle::Fill
    } else {
        IconStyle::Stroke
    }
}

/// 解析单个元素喵,返回 (风格, 几何)喵
fn parse_element(tag: &str) -> Option<(IconStyle, ShapeKind)> {
    let tag = &tag[1..]; // 去掉 '<' 喵
    let t = tag.trim_end_matches('/').trim();
    // 元素名到第一个空格或 '>' 为止喵
    let name_end = t.find(|c: char| c.is_whitespace()).unwrap_or(t.len());
    let name = &t[..name_end];
    let style = element_style(&t.to_ascii_lowercase());

    let kind = match name {
        "path" => {
            let d = attr(t, "d")?.trim().to_string();
            if d.is_empty() {
                return None;
            }
            ShapeKind::Path(d)
        }
        "line" => {
            let (x1, y1, x2, y2) = (attr(t, "x1")?, attr(t, "y1")?, attr(t, "x2")?, attr(t, "y2")?);
            ShapeKind::Line { x1: f(&x1), y1: f(&y1), x2: f(&x2), y2: f(&y2) }
        }
        "circle" => {
            let (cx, cy, r) = (attr(t, "cx")?, attr(t, "cy")?, attr(t, "r")?);
            ShapeKind::Circle { cx: f(&cx), cy: f(&cy), r: f(&r) }
        }
        "rect" => {
            let (x, y, w, h) = (
                attr(t, "x")?,
                attr(t, "y")?,
                attr(t, "width")?,
                attr(t, "height")?,
            );
            ShapeKind::Rect {
                x: f(&x),
                y: f(&y),
                w: f(&w),
                h: f(&h),
                r: attr(t, "rx").map(|v| f(&v)).unwrap_or(0.0),
            }
        }
        "polyline" => {
            let pts = attr(t, "points")?;
            ShapeKind::Polyline(numbers(&pts).chunks(2).map(|c| (c[0], c[1])).collect())
        }
        _ => return None,
    };
    Some((style, kind))
}

fn attr(t: &str, key: &str) -> Option<String> {
    // 必须匹配「key=」整体,避免 key 是元素名/其他属性子串的误命中喵
    // (经典翻车: 在 "circle" 里找 "r",撞上元素名直接失败)喵
    let pattern = format!("{key}=");
    let mut rest = t;
    loop {
        let idx = rest.find(&pattern)?;
        let after = &rest[idx + pattern.len()..];
        let q = after.chars().next()?;
        if q != '"' && q != '\'' {
            rest = &rest[idx + 1..];
            continue;
        }
        let span = &after[1..];
        let end = span.find(q)?;
        return Some(span[..end].trim().to_string());
    }
}

fn f(s: &str) -> f32 {
    s.trim().parse().unwrap_or(0.0)
}

/// 把混杂逗号/空格/连字符的数字串拆成 [f32] 喵('-' 属于其后的数字;公开便于单测)喵
pub fn numbers(s: &str) -> Vec<f32> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut last_was_num = false;
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            buf.push(c);
            last_was_num = true;
        } else if c == '-' || c == '+' {
            if last_was_num && !buf.is_empty() {
                out.push(buf.parse().unwrap_or(0.0));
                buf.clear();
            }
            buf.push(c);
            last_was_num = false;
        } else if c == 'e' || c == 'E' {
            buf.push(c);
        } else {
            // 逗号 / 空格 / 换行等分隔符喵
            if last_was_num && !buf.is_empty() {
                out.push(buf.parse().unwrap_or(0.0));
                buf.clear();
            }
            last_was_num = false;
        }
    }
    if last_was_num && !buf.is_empty() {
        out.push(buf.parse().unwrap_or(0.0));
    }
    out
}

/// 解析 SVG path 的 d 属性喵(公开便于单测)喵
pub fn build_path(d: &str) -> Path {
    Path::from_svg(d).unwrap_or_default()
}

