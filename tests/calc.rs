//! 算式求值器测试喵~
//!
//! 覆盖: 四则/优先级/括号/幂/一元负号/取模/格式化,
//! 以及「非算式一律放行给普通搜索」的守门逻辑喵。

use meow_app_launcher::search::calc::{format_number, try_eval};

fn eval(s: &str) -> String {
    try_eval(s).map(format_number).unwrap_or_else(|| "<None>".into())
}

#[test]
fn basic_arithmetic() {
    assert_eq!(eval("1+2"), "3");
    assert_eq!(eval("10 - 4"), "6");
    assert_eq!(eval("3*7"), "21");
    assert_eq!(eval("9 / 2"), "4.5");
    assert_eq!(eval("7%3"), "1");
}

#[test]
fn precedence_and_parens() {
    assert_eq!(eval("1+2*3"), "7");
    assert_eq!(eval("(1+2)*3"), "9");
    assert_eq!(eval("2^3^2"), "512", "幂运算右结合喵");
    assert_eq!(eval("2*(3+4)^2"), "98");
    assert_eq!(eval("10/4*2"), "5", "同级从左到右喵");
}

#[test]
fn unary_signs() {
    assert_eq!(eval("-5+2"), "-3");
    assert_eq!(eval("3*-2"), "-6");
    assert_eq!(eval("-(2+3)"), "-5");
    assert_eq!(eval("--3"), "3", "双重负号喵");
}

#[test]
fn decimals() {
    assert_eq!(eval("0.5+0.25"), "0.75");
    assert_eq!(eval("1/3"), "0.3333333333", "最多 10 位小数喵");
    assert_eq!(eval("0.1+0.2"), "0.3", "浮点误差按 10 位截断喵");
}

#[test]
fn non_finite_or_invalid_returns_none() {
    // 除零喵
    assert!(try_eval("1/0").is_none());
    // 语法不完整喵
    assert!(try_eval("1+").is_none());
    assert!(try_eval("(1+2").is_none());
    assert!(try_eval("()").is_none());
    // 孤立小数点喵
    assert!(try_eval("1..2").is_none());
    assert!(try_eval("1 . 2").is_none());
    // 括号不配对喵
    assert!(try_eval(")1+2(").is_none());
}

#[test]
fn non_expressions_pass_through() {
    // 纯数字是普通查询,不是算式喵
    assert!(try_eval("123").is_none());
    assert!(try_eval("3.14").is_none());
    // 含白名单外字符直接放行喵
    assert!(try_eval("v2.1+").is_none());
    assert!(try_eval("chrome").is_none());
    // 空查询喵
    assert!(try_eval("").is_none());
    assert!(try_eval("   ").is_none());
}

#[test]
fn format_number_style() {
    assert_eq!(format_number(3.0), "3");
    assert_eq!(format_number(-0.0), "0");
    assert_eq!(format_number(2.5), "2.5");
    assert_eq!(format_number(1.0 / 0.0), "∞");
    assert_eq!(format_number(-1.0 / 0.0), "-∞");
    assert_eq!(format_number(0.0 / 0.0), "不是数字");
}
