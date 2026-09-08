//! 极简内嵌 SVG 图标渲染喵~
//!
//! `assets/icons/` 下的 24×24 小图标素材,运行时经 `include_str!` 打进二进制,
//! 不需要 skia `svg` feature(避免换预编译二进制),只解析用到的子集:
//! `<path d="...">`(M/m、L/l、H/h、V/v、A/a、Z/z)、`<line>`、`<circle>`、
//! `<rect rx>`、`<polyline>` 喵。圆弧按 SVG 端点→中心参数化采样成折线,视觉足够喵。

use skia_safe::{Canvas, Color, Paint, PaintStyle, Path, PathBuilder, Rect};
use std::f32::consts::PI;
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
    /// 可绘制元素数量喵(公开便于单测)喵
    pub fn shapes_len(&self) -> usize {
        self.shapes.len()
    }

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

// ---------------------------------------------------------------------------
// SVG path 数据 → Skia Path 喵
// ---------------------------------------------------------------------------

/// 把 path 数据切成 (命令, 数字段) 序列喵(隐含的参数续用交给构建循环)喵
fn tokenize(d: &str) -> Vec<(char, Vec<f32>)> {
    let mut out = Vec::new();
    let mut cmd = 'M';
    let mut buf = String::new();
    for c in d.chars() {
        if c.is_ascii_alphabetic() {
            if !buf.trim().is_empty() {
                out.push((cmd, numbers(&buf)));
                buf.clear();
            }
            cmd = c;
        } else {
            buf.push(c);
        }
    }
    if !buf.trim().is_empty() {
        out.push((cmd, numbers(&buf)));
    }
    out
}

/// 每个命令的参数个数喵
fn params_per(cmd: char) -> usize {
    match cmd {
        'M' | 'L' => 2,
        'H' | 'V' => 1,
        'A' => 7,
        _ => 0,
    }
}

/// 解析 SVG path 的 d 属性喵(公开便于单测)喵
pub fn build_path(d: &str) -> Path {
    let mut b = PathBuilder::new();
    let mut cur = (0.0_f32, 0.0_f32);
    let mut start = cur;
    let mut first = true;

    for (cmd, nums) in tokenize(d) {
        let rel = cmd.is_ascii_lowercase();
        let major = cmd.to_ascii_uppercase();
        if major == 'Z' {
            b.close();
            cur = start;
            first = false;
            continue;
        }
        let per = params_per(major);
        if per == 0 {
            continue;
        }
        let mut i = 0;
        while i + per <= nums.len() {
            match major {
                'M' | 'L' => {
                    let x = nums[i] + if rel { cur.0 } else { 0.0 };
                    let y = nums[i + 1] + if rel { cur.1 } else { 0.0 };
                    if major == 'M' && (first || i == 0) {
                        b.move_to((x, y));
                        start = (x, y);
                        first = false;
                    } else {
                        b.line_to((x, y));
                    }
                    cur = (x, y);
                }
                'H' => {
                    let x = nums[i] + if rel { cur.0 } else { 0.0 };
                    b.line_to((x, cur.1));
                    cur = (x, cur.1);
                }
                'V' => {
                    let y = nums[i] + if rel { cur.1 } else { 0.0 };
                    b.line_to((cur.0, y));
                    cur = (cur.0, y);
                }
                'A' => {
                    let end = (
                        nums[i + 5] + if rel { cur.0 } else { 0.0 },
                        nums[i + 6] + if rel { cur.1 } else { 0.0 },
                    );
                    arc_to(&mut b, cur, end, nums[i], nums[i + 1], nums[i + 2], nums[i + 3] != 0.0, nums[i + 4] != 0.0);
                    cur = end;
                }
                _ => break,
            }
            i += per;
        }
    }
    b.snapshot()
}

/// SVG 圆弧 → 采样折线喵(端点参数化 + 中心转换)喵
#[allow(clippy::too_many_arguments)]
fn arc_to(
    b: &mut PathBuilder,
    p0: (f32, f32),
    end: (f32, f32),
    rx: f32,
    ry: f32,
    rot_deg: f32,
    large: bool,
    sweep: bool,
) {
    let (x1, y1) = p0;
    let (x2, y2) = end;
    let mut rx = rx.abs().max(0.0001);
    let mut ry = ry.abs().max(0.0001);
    if (x1 - x2).abs() < 0.0001 && (y1 - y2).abs() < 0.0001 {
        return;
    }
    let phi = rot_deg.to_radians();
    let (cp, sp) = (phi.cos(), phi.sin());

    let dx2 = (x1 - x2) / 2.0;
    let dy2 = (y1 - y2) / 2.0;
    let x1p = cp * dx2 + sp * dy2;
    let y1p = -sp * dx2 + cp * dy2;

    let l = x1p * x1p / (rx * rx) + y1p * y1p / (ry * ry);
    if l > 1.0 {
        let k = l.sqrt();
        rx *= k;
        ry *= k;
    }
    let num = (rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p).max(0.0);
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let mut coef = if den <= 0.0 {
        0.0
    } else {
        (num / den).sqrt()
    };
    if large == sweep {
        coef = -coef;
    }
    let cxp = coef * (rx * y1p / ry);
    let cyp = coef * (-ry * x1p / rx);
    let cx = cp * cxp - sp * cyp + (x1 + x2) / 2.0;
    let cy = sp * cxp + cp * cyp + (y1 + y2) / 2.0;

    let ux = (x1p - cxp) / rx;
    let uy = (y1p - cyp) / ry;
    let vx = (-x1p - cxp) / rx;
    let vy = (-y1p - cyp) / ry;

    let mut delta = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    if !sweep && delta > 0.0 {
        delta -= 2.0 * PI;
    } else if sweep && delta < 0.0 {
        delta += 2.0 * PI;
    }

    let steps = ((delta.abs() / (PI / 24.0)).ceil() as usize).clamp(3, 256);
    for k in 1..=steps {
        let a = delta * (k as f32 / steps as f32);
        let (ca, sa) = (a.cos(), a.sin());
        let x = cx + (rx * ca) * cp - (ry * sa) * sp;
        let y = cy + (rx * ca) * sp + (ry * sa) * cp;
        b.line_to((x, y));
    }
}

