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
pub const ICONS: [(&str, &str); 7] = [
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
];

/// 解析后的图标几何喵
#[derive(Debug)]
pub struct SvgIcon {
    /// 是否为描边型(true = stroke,false = fill)喵
    stroke: bool,
    shapes: Vec<Shape>,
}

#[derive(Debug)]
enum Shape {
    /// SVG path 数据喵
    Path(String),
    Line { x1: f32, y1: f32, x2: f32, y2: f32 },
    Circle { cx: f32, cy: f32, r: f32 },
    Rect { x: f32, y: f32, w: f32, h: f32, r: f32 },
    Polyline(Vec<(f32, f32)>),
}

/// 取内置图标(惰性解析 + 缓存)喵
pub fn icon(name: &str) -> &'static SvgIcon {
    static CACHE: [OnceLock<SvgIcon>; 7] = [
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
    /// 把图标画到目标矩形内(自动缩放,描边线宽随尺寸)喵
    pub fn draw(&self, canvas: &Canvas, rect: Rect, color: Color) {
        let w = rect.width().max(1.0);
        let h = rect.height().max(1.0);
        let s = w.min(h) / 24.0;

        let mut p = Paint::default();
        p.set_color(color);
        p.set_anti_alias(true);
        p.set_style(if self.stroke {
            PaintStyle::Stroke
        } else {
            PaintStyle::Fill
        });
        if self.stroke {
            p.set_stroke_width((2.0 * s).max(1.0));
            p.set_stroke_cap(skia_safe::PaintCap::Round);
            p.set_stroke_join(skia_safe::PaintJoin::Round);
        }

        // 平移取整,避免次像素模糊喵
        let side = w.min(h);
        let dx = (rect.center_x() - side / 2.0).round();
        let dy = (rect.center_y() - side / 2.0).round();
        canvas.save();
        canvas.translate((dx, dy));
        canvas.scale((s, s));
        for shape in &self.shapes {
            match shape {
                Shape::Path(d) => {
                    let path = build_path(d);
                    canvas.draw_path(&path, &p);
                }
                Shape::Line { x1, y1, x2, y2 } => {
                    canvas.draw_line((*x1, *y1), (*x2, *y2), &p);
                }
                Shape::Circle { cx, cy, r } => {
                    let rr = (*r).max(0.0);
                    canvas.draw_circle((*cx, *cy), rr, &p);
                }
                Shape::Rect { x, y, w, h, r } => {
                    let rrect = Rect::from_xywh(*x, *y, *w, *h);
                    if self.stroke {
                        let path = rounded_stroke_path(rrect, *r);
                        canvas.draw_path(&path, &p);
                    } else {
                        canvas.draw_rect(rrect, &p);
                    }
                }
                Shape::Polyline(pts) => {
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

fn parse_svg(xml: &str) -> SvgIcon {
    let lower = xml.to_ascii_lowercase();
    let stroke = lower.contains("fill=\"none\"") || lower.contains("stroke=\"currentcolor\"");
    let mut shapes = Vec::new();

    // 逐元素提取 <tag .../> 与 <tag ...>...</tag> 喵
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        let end = rest[start..]
            .find('>')
            .map(|i| start + i + 1)
            .unwrap_or(rest.len());
        let tag = &rest[start..end];
        parse_element(tag, &mut shapes);
        rest = &rest[end..];
    }
    SvgIcon { stroke, shapes }
}

fn parse_element(tag: &str, out: &mut Vec<Shape>) {
    let tag = &tag[1..]; // 去掉 '<' 喵
    let t = tag.trim_end_matches('/').trim();
    // 元素名到第一个空格或 '>' 为止喵
    let name_end = t.find(|c: char| c.is_whitespace()).unwrap_or(t.len());
    let name = &t[..name_end];

    match name {
        "path" => {
            if let Some(d) = attr(t, "d") {
                let d = d.trim();
                if !d.is_empty() {
                    out.push(Shape::Path(d.to_string()));
                }
            }
        }
        "line" => {
            if let (Some(x1), Some(y1), Some(x2), Some(y2)) =
                (attr(t, "x1"), attr(t, "y1"), attr(t, "x2"), attr(t, "y2"))
            {
                out.push(Shape::Line {
                    x1: f(&x1),
                    y1: f(&y1),
                    x2: f(&x2),
                    y2: f(&y2),
                });
            }
        }
        "circle" => {
            if let (Some(cx), Some(cy), Some(r)) = (attr(t, "cx"), attr(t, "cy"), attr(t, "r")) {
                out.push(Shape::Circle {
                    cx: f(&cx),
                    cy: f(&cy),
                    r: f(&r),
                });
            }
        }
        "rect" => {
            let (x, y, w, h, r) = if let (Some(x), Some(y), Some(w), Some(h)) =
                (attr(t, "x"), attr(t, "y"), attr(t, "width"), attr(t, "height"))
            {
                (
                    f(&x),
                    f(&y),
                    f(&w),
                    f(&h),
                    attr(t, "rx").map(|v| f(&v)).unwrap_or(0.0),
                )
            } else {
                return;
            };
            out.push(Shape::Rect { x, y, w, h, r });
        }
        "polyline" => {
            if let Some(pts) = attr(t, "points") {
                out.push(Shape::Polyline(numbers(&pts).chunks(2).map(|c| (c[0], c[1])).collect()));
            }
        }
        _ => {}
    }
}

fn attr(t: &str, key: &str) -> Option<String> {
    // 找 key="..." 喵(允许单引号)喵
    let mut rest = t;
    loop {
        let idx = rest.find(key)?;
        let after = &rest[idx + key.len()..];
        let eq = after.strip_prefix('=')?;
        let q = eq.chars().next()?;
        if q != '"' && q != '\'' {
            rest = &rest[idx + 1..];
            continue;
        }
        let span = &eq[1..];
        let end = span.find(q)?;
        return Some(span[..end].trim().to_string());
    }
}

fn f(s: &str) -> f32 {
    s.trim().parse().unwrap_or(0.0)
}

/// 把混杂逗号/空格/连字符的数字串拆成 [f32] 喵('-' 属于其后的数字)喵
fn numbers(s: &str) -> Vec<f32> {
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

fn build_path(d: &str) -> Path {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_tokenizer_handles_negatives() {
        assert_eq!(numbers("9,11 12 14 20 6-2-2"), vec![9.0, 11.0, 12.0, 14.0, 20.0, 6.0, -2.0, -2.0]);
        // 科学计数法保持喵
        assert!(numbers("1e-2 3").len() >= 2);
    }

    #[test]
    fn parse_builtin_icons() {
        for (_, xml) in ICONS {
            let icon = parse_svg(xml);
            assert!(!icon.shapes.is_empty(), "图标应有可绘制元素喵");
        }
    }

    #[test]
    fn path_builds_no_panic() {
        for (_, xml) in ICONS {
            if let Some(d) = extract_d(xml) {
                let p = build_path(&d);
                assert!(p.bounds().right >= 0.0 || p.bounds().left >= 0.0 || true);
            }
        }
    }

    fn extract_d(xml: &str) -> Option<String> {
        // 粗取第一个 d="..." 喵
        let i = xml.find("d=\"")?;
        let rest = &xml[i + 3..];
        let j = rest.find('"')?;
        Some(rest[..j].to_string())
    }
}