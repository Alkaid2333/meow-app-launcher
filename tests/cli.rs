//! 命令行注册参数解析喵~ 单元测试喵

use meow_app_launcher::cli::parse_register_args;

#[test]
fn parse_register_basic() {
    let args = ["记事本".into(), "C:/Windows/notepad.exe".into()];
    let (name, path, ico) = parse_register_args(&args).unwrap();
    assert_eq!(name, "记事本");
    assert_eq!(path, "C:/Windows/notepad.exe");
    assert!(ico.is_none());
}

#[test]
fn parse_register_with_ico() {
    let args = [
        "Chrome".into(),
        "C:/chrome.exe".into(),
        "-ico".into(),
        "C:/icon.png".into(),
    ];
    let (_, _, ico) = parse_register_args(&args).unwrap();
    assert_eq!(ico.as_deref(), Some("C:/icon.png"));
}

#[test]
fn parse_register_too_few_args() {
    assert!(parse_register_args(&[]).is_none());
    assert!(parse_register_args(&["Chrome".into()]).is_none());
}
