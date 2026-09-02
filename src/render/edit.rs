//! 通用文本编辑状态与渲染喵~
//!
//! 启动器搜索框、配置窗口的过滤关键词 / 标签 / 数值框共用同一套模型:
//! * 文本 + 光标 + 选中区(均为 **字节偏移**,保证 UTF-8 安全)喵
//! * 单击定位光标、拖拽双向选择、键入/退格/删除按选中区替换喵
//! * 尺寸测量与绘制共用同一几何,命中与渲染不漂移喵
//!
//! 本模块不关心布局,只负责「文本怎么编辑、怎么画」喵。

use skia_safe::{Canvas, Color, Font, Paint, Rect};

/// 可编辑文本缓冲喵
#[derive(Debug, Clone, Default)]
pub struct TextEdit {
    /// 文本内容喵
    pub text: String,
    /// 光标位置(字节偏移,UTF-8 边界)喵
    pub caret: usize,
    /// 选中区(字节区间 [lo, hi)),None 表示无选中喵
    pub selection: Option<(usize, usize)>,
    /// 拖选锚点(字节偏移): 按下鼠标时的位置,反向拖选也以此为基准喵
    pub anchor: Option<usize>,
}

impl TextEdit {
    /// 新建缓冲喵(光标在文末)喵
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let caret = text.len();
        Self {
            text,
            caret,
            selection: None,
            anchor: None,
        }
    }

    /// 光标有效位置(钳制到文本长度,保证是合法边界)喵
    pub fn caret(&self) -> usize {
        self.caret.min(self.text.len())
    }

    /// 选中区(钳制范围后返回)喵
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.selection
            .map(|(lo, hi)| (lo.min(self.text.len()), hi.min(self.text.len())))
            .filter(|(lo, hi)| lo < hi)
    }

    /// 全选喵(聚焦首次点击的标配行为)喵
    pub fn select_all(&mut self) {
        if self.text.is_empty() {
            self.caret = 0;
            self.selection = None;
            self.anchor = None;
            return;
        }
        self.caret = self.text.len();
        self.selection = Some((0, self.text.len()));
        self.anchor = None;
    }

    /// 拖选起点锚定喵(记录锚点,保证反向拖选正确)喵
    pub fn begin_select(&mut self, byte: usize) {
        let b = byte.min(self.text.len());
        self.caret = b;
        self.anchor = Some(b);
        self.selection = Some((b, b));
    }

    /// 拖选延续喵(以锚点为基准取 min/max,正向/反向都可靠)喵
    pub fn extend_select(&mut self, byte: usize) {
        let b = byte.min(self.text.len());
        let a = self.anchor.unwrap_or(b);
        self.selection = Some((a.min(b), a.max(b)));
        self.caret = b;
    }

    /// 结束拖选模态喵(清掉锚点,保留选中区)喵
    pub fn end_select(&mut self) {
        self.anchor = None;
    }

    /// 键入一个字符(替换选中区,否则插到光标处)喵
    pub fn insert_char(&mut self, ch: char) {
        let c = self.caret();
        if let Some((lo, hi)) = self.selection {
            self.text.replace_range(lo..hi, &ch.to_string());
            self.caret = lo + ch.len_utf8();
        } else {
            self.text.insert(c, ch);
            self.caret = c + ch.len_utf8();
        }
        self.selection = None;
        self.anchor = None;
    }

    /// 退格喵(删除选中区,否则删光标前一个字符)喵
    pub fn backspace(&mut self) {
        let c = self.caret();
        if let Some((lo, hi)) = self.selection {
            self.text.replace_range(lo..hi, "");
            self.caret = lo;
        } else if c > 0 {
            let by = self.text[..c]
                .chars()
                .next_back()
                .map(|ch| ch.len_utf8())
                .unwrap_or(1);
            self.text.replace_range(c - by..c, "");
            self.caret = c - by;
        }
        self.selection = None;
        self.anchor = None;
    }

    /// Delete 键喵(删除选中区,否则删光标后一个字符)喵
    pub fn delete(&mut self) {
        let c = self.caret();
        if let Some((lo, hi)) = self.selection {
            self.text.replace_range(lo..hi, "");
            self.caret = lo;
        } else if c < self.text.len() {
            let n = self.text[c..]
                .chars()
                .next()
                .map(|ch| ch.len_utf8())
                .unwrap_or(1);
            self.text.replace_range(c..c + n, "");
        }
        self.selection = None;
        self.anchor = None;
    }
}

/// 由鼠标 x 换算文本内光标字节位置喵
///
/// `text_left` 为首个字符落点 x(与渲染一致),`x` 为命中点,
/// 返回离视觉最近的字符边界(字节偏移,UTF-8 安全)喵。
pub fn caret_from_x(font: &Font, paint: &Paint, text: &str, text_left: f32, x: f32) -> usize {
    if text.is_empty() {
        return 0;
    }
    let total = font.measure_str(text, Some(paint)).0;
    if x >= text_left + total {
        return text.len();
    }
    if x <= text_left {
        return 0;
    }
    // 线性扫描各字符边界,取视觉最近的一个喵
    let mut best = 0usize;
    let mut best_dx = (x - text_left).abs();
    for (i, _) in text.char_indices() {
        let w = font.measure_str(&text[..i], Some(paint)).0;
        let dx = (x - (text_left + w)).abs();
        if dx < best_dx {
            best_dx = dx;
            best = i;
        }
    }
    best
}

/// 在输入框内绘制文本编辑态喵: 选中高亮 + 文本 + 光标喵
///
/// 文字从 `rect.left + PAD_X` 开始,垂直居中,超出裁剪;
/// `edit` 提供光标与选中区(字节偏移),None 表示静态展示无光标喵。
pub fn draw_text_edit(
    canvas: &Canvas,
    rect: Rect,
    font: &Font,
    fg_color: Color,
    accent: Color,
    text: &str,
    edit: Option<&TextEdit>,
) {
    let text_left = rect.left + PAD_X;
    let clip = Rect::from_xywh(
        rect.left + PAD_X,
        rect.top,
        (rect.width() - PAD_X * 2.0).max(4.0),
        rect.height(),
    );

    // 字体度量 → 垂直居中基线喵
    let (_, m) = font.metrics();
    let baseline = (rect.center_y() - (m.ascent + m.descent) / 2.0).round();

    // 选中高亮喵
    if let Some(edit) = edit
        && let Some((lo, hi)) = edit.selection()
    {
        let p = color_paint(fg_color);
        let (w0, _) = font.measure_str(&text[..lo], Some(&p));
        let (w1, _) = font.measure_str(&text[..hi], Some(&p));
        let sel = Rect::from_xywh(
            text_left + w0,
            rect.top + 2.0,
            (w1 - w0).max(2.0),
            (rect.height() - 4.0).max(2.0),
        );
        let a = accent;
        let mut bg = Paint::default();
        bg.set_color(Color::from_argb(0x33, a.r(), a.g(), a.b()));
        bg.set_anti_alias(true);
        canvas.save();
        canvas.clip_rect(clip, None, Some(false));
        canvas.draw_rect(sel, &bg);
        canvas.restore();
    }

    // 文本喵
    let mut tp = Paint::default();
    tp.set_color(fg_color);
    tp.set_anti_alias(true);
    canvas.save();
    canvas.clip_rect(clip, None, Some(false));
    canvas.draw_str(text, (text_left.round(), baseline), font, &tp);
    canvas.restore();

    // 光标喵
    if let Some(edit) = edit {
        let caret = edit.caret().min(text.len());
        let p = color_paint(fg_color);
        let (w, _) = font.measure_str(&text[..caret], Some(&p));
        let mut cp = Paint::default();
        cp.set_color(accent);
        cp.set_anti_alias(true);
        cp.set_stroke_width(1.5);
        let cx = (text_left + w).min(clip.right - 2.0).max(clip.left);
        canvas.draw_line(
            (cx, rect.top + 5.0),
            (cx, rect.bottom - 5.0),
            &cp,
        );
    }
}

/// 输入框文字左内边距(逻辑 px)喵
pub const PAD_X: f32 = 8.0;

/// 构造一个仅用于测量宽度的画笔喵
pub fn color_paint(c: Color) -> Paint {
    let mut p = Paint::default();
    p.set_color(c);
    p.set_anti_alias(true);
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caret_limits_and_utf8() {
        let mut e = TextEdit::new("abc");
        assert_eq!(e.caret(), 3);
        e.insert_char('x');
        assert_eq!(e.text, "abcx");
        e.backspace();
        assert_eq!(e.text, "abc");
        // 中文: 每字 3 字节,光标/删除都要按字符走喵
        let mut c = TextEdit::new("喵喵");
        c.backspace();
        assert_eq!(c.text, "喵");
        c.caret = 0;
        c.selection = None;
        c.insert_char('a');
        assert_eq!(c.text, "a喵");
    }

    #[test]
    fn selection_replaced_in_both_directions() {
        let mut e = TextEdit::new("abcdef");
        e.begin_select(4);
        e.extend_select(1); // 反向拖选 → [1,4) 喵
        assert_eq!(e.selection(), Some((1, 4)));
        e.insert_char('X');
        assert_eq!(e.text, "aXef");
        // 正向拖选喵
        let mut f = TextEdit::new("abcdef");
        f.begin_select(1);
        f.extend_select(4);
        assert_eq!(f.selection(), Some((1, 4)));
        f.backspace();
        assert_eq!(f.text, "aef");
    }

    #[test]
    fn select_all_and_override() {
        let mut e = TextEdit::new("你好");
        e.select_all();
        assert_eq!(e.selection(), Some((0, 6)));
        e.insert_char('嗨');
        assert_eq!(e.text, "嗨");
    }

    #[test]
    fn backward_drag_with_anchor() {
        // 从右往左拖: 锚点固定在下按时位置,端点取 min/max 喵
        let mut e = TextEdit::new("abcdef");
        e.begin_select(4); // 鼠标按下在 4 处喵
        e.extend_select(1); // 向左拖到 1 喵
        assert_eq!(e.selection(), Some((1, 4)));
        // 继续向左喵
        e.extend_select(0);
        assert_eq!(e.selection(), Some((0, 4)));
        // 回拖越过锚点: 选区锁定为「锚点 ↔ 光标」的区间喵
        e.extend_select(5);
        assert_eq!(e.selection(), Some((4, 5)));
        e.end_select();
        e.insert_char('X');
        assert_eq!(e.text, "abcdXf");
    }
}