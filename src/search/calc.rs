//! 算式求值器喵~
//!
//! 输入形如 `1 + 2 * 3` 的查询,直接给出结果条目(Enter 复制)喵。
//! 实现: 词法分析 → 调度场算法(shunting-yard) → 逆波兰求值喵。
//! 纯 Rust 零依赖,只接受白名单字符,拒绝一切花哨输入喵。

/// 尝试把整条查询当作算式求值喵。
///
/// 触发条件(全部满足才动手,避免干扰普通搜索):
/// * 只含 数字 / `+ - * / % ^ ( )` / 空格 / 小数点喵
/// * 至少含一个运算符或括号(纯数字是普通查询,不算式)喵
/// * 语法合法且结果有限(除零/溢出返回 None)喵
pub fn try_eval(input: &str) -> Option<f64> {
    let tokens = tokenize(input)?;
    // 纯数字(或纯括号)不算算式,交给普通搜索喵
    if tokens
        .iter()
        .all(|t| matches!(t, Tok::Num(_) | Tok::LParen | Tok::RParen))
    {
        return None;
    }
    let rpn = to_rpn(tokens)?;
    eval_rpn(&rpn).filter(|v| v.is_finite())
}

/// 把结果格式化成人类友好的短文本喵:
/// 整数不带小数点,小数最多保留 10 位并裁掉尾零喵
pub fn format_number(v: f64) -> String {
    if v.is_nan() {
        return "不是数字".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "∞".into() } else { "-∞".into() };
    }
    let s = format!("{v:.10}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    // "-0" 可爱但多余,统一归 0 喵
    if s == "-0" { "0".into() } else { s.to_string() }
}

// ---------------------------------------------------------------------------
// 词法分析
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    /// 二元运算符喵(逆波兰里一元负号用 'n' 表示)
    Op(char),
    LParen,
    RParen,
}

/// 词法分析喵: 出现白名单外字符直接判负,交给普通搜索喵
fn tokenize(input: &str) -> Option<Vec<Tok>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' => {
                chars.next();
            }
            '0'..='9' | '.' => {
                let mut num = String::new();
                let mut dots = 0;
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() {
                        num.push(c);
                        chars.next();
                    } else if c == '.' {
                        dots += 1;
                        if dots > 1 {
                            return None; // 多个小数点,语法错误喵
                        }
                        num.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                // 孤立小数点不算数喵
                if num == "." {
                    return None;
                }
                tokens.push(Tok::Num(num.parse().ok()?));
            }
            '+' | '-' | '*' | '/' | '%' | '^' => {
                tokens.push(Tok::Op(c));
                chars.next();
            }
            '(' => {
                tokens.push(Tok::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Tok::RParen);
                chars.next();
            }
            _ => return None,
        }
    }
    if tokens.is_empty() { None } else { Some(tokens) }
}

// ---------------------------------------------------------------------------
// 调度场: 中缀 → 逆波兰
// ---------------------------------------------------------------------------

/// 运算符栈项喵(一元负号独立建模,语义才清爽)喵
#[derive(Debug, Clone, Copy, PartialEq)]
enum StackItem {
    Bin(char),
    Neg,
    LParen,
}

/// 二元运算符 → (优先级, 是否右结合)喵
fn op_spec(op: char) -> Option<(u8, bool)> {
    match op {
        '+' | '-' => Some((1, false)),
        '*' | '/' | '%' => Some((2, false)),
        '^' => Some((3, true)),
        _ => None,
    }
}

/// 一元负号优先级最高且右结合喵
const UNARY_PREC: u8 = 4;

fn to_rpn(tokens: Vec<Tok>) -> Option<Vec<Tok>> {
    let mut out: Vec<Tok> = Vec::new();
    let mut ops: Vec<StackItem> = Vec::new();
    // 上一个词是否「产出了值」: 用于判定 +/- 是一元还是二元喵
    let mut prev_value = false;
    let mut depth = 0usize;

    for tok in tokens {
        match tok {
            Tok::Num(_) => {
                out.push(tok);
                prev_value = true;
            }
            Tok::LParen => {
                depth += 1;
                ops.push(StackItem::LParen);
                prev_value = false;
            }
            Tok::RParen => {
                depth = depth.checked_sub(1)?; // 括号不配对喵
                loop {
                    match ops.pop()? {
                        StackItem::LParen => break,
                        other => push_op(other, &mut out),
                    }
                }
                prev_value = true;
            }
            Tok::Op(op) => {
                // 一元 +/-: 出现在开头 / 运算符后 / 左括号后喵
                let unary = !prev_value && matches!(op, '+' | '-');
                let item = if unary { StackItem::Neg } else { StackItem::Bin(op) };
                let (prec, right_assoc) = if unary {
                    (UNARY_PREC, true)
                } else {
                    op_spec(op)?
                };
                while let Some(&top) = ops.last() {
                    let top_prec = match top {
                        StackItem::LParen => break,
                        StackItem::Neg => UNARY_PREC,
                        StackItem::Bin(o) => op_spec(o)?.0,
                    };
                    if top_prec > prec || (top_prec == prec && !right_assoc) {
                        push_op(ops.pop().unwrap(), &mut out);
                    } else {
                        break;
                    }
                }
                ops.push(item);
                prev_value = false;
            }
        }
    }
    if depth != 0 {
        return None; // 括号没闭合喵
    }
    while let Some(item) = ops.pop() {
        if item == StackItem::LParen {
            return None;
        }
        push_op(item, &mut out);
    }
    if out.is_empty() { None } else { Some(out) }
}

/// 把栈顶运算符写进输出队列喵(一元负号用专用记号 'n')喵
fn push_op(item: StackItem, out: &mut Vec<Tok>) {
    match item {
        StackItem::Bin(op) => out.push(Tok::Op(op)),
        StackItem::Neg => out.push(Tok::Op('n')),
        StackItem::LParen => {}
    }
}

// ---------------------------------------------------------------------------
// 逆波兰求值
// ---------------------------------------------------------------------------

fn eval_rpn(rpn: &[Tok]) -> Option<f64> {
    let mut stack: Vec<f64> = Vec::new();
    for tok in rpn {
        match tok {
            Tok::Num(v) => stack.push(*v),
            Tok::Op('n') => {
                let a = stack.pop()?;
                stack.push(-a);
            }
            Tok::Op(op) => {
                let b = stack.pop()?;
                let a = stack.pop()?;
                let v = match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' => a / b,
                    '%' => a % b,
                    '^' => a.powf(b),
                    _ => return None,
                };
                stack.push(v);
            }
            _ => return None,
        }
    }
    // 正常算式最终恰好剩一个值喵
    if stack.len() == 1 { stack.pop() } else { None }
}
