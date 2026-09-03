//! 文本编辑缓冲喵~ 单元测试喵

use meow_app_launcher::render::edit::TextEdit;

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
