//! named-pipe IPC 协议解析喵~ 单元测试喵
//!
//! 协议为「一行一命令」的 UTF-8 文本,解析是纯函数,可以离线验证喵。

use meow_app_launcher::platform::{parse_ipc_command, IpcCommand};

#[test]
fn 解析无参命令喵() {
    assert_eq!(parse_ipc_command("show"), Some(IpcCommand::Show));
    assert_eq!(parse_ipc_command("hide"), Some(IpcCommand::Hide));
    assert_eq!(parse_ipc_command("toggle"), Some(IpcCommand::Toggle));
}

#[test]
fn 解析查询命令喵() {
    assert_eq!(
        parse_ipc_command("query 你好世界"),
        Some(IpcCommand::Query("你好世界".into()))
    );
    // 多词查询保留原样(含内部空格)喵
    assert_eq!(
        parse_ipc_command("query  hello world "),
        Some(IpcCommand::Query("hello world".into()))
    );
}

#[test]
fn 拒绝空查询与未知命令喵() {
    // query 不带文本无效喵
    assert_eq!(parse_ipc_command("query"), None);
    assert_eq!(parse_ipc_command("query   "), None);
    // 未知动词 / 空行无效喵
    assert_eq!(parse_ipc_command("eval rm -rf"), None);
    assert_eq!(parse_ipc_command(""), None);
    assert_eq!(parse_ipc_command("  \n"), None);
}

#[test]
fn 首行截断与首尾空白容忍喵() {
    // 带换行的请求行按首行解析喵
    assert_eq!(parse_ipc_command("show\n"), Some(IpcCommand::Show));
    assert_eq!(parse_ipc_command("  show  "), Some(IpcCommand::Show));
    // 协议动词大小写敏感(保持简单严格)喵
    assert_eq!(parse_ipc_command("SHOW"), None);
}
