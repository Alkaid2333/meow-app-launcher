//! 指令模块的测试喵~ 覆盖自定义指令、shell 选择、`> ` 指令模式与 serde 兼容喵。

use meow_app_launcher::app::config::CommandEntry;
use meow_app_launcher::app::{aggregate_items, command_mode_items, ListItem};
use meow_app_launcher::platform::{ShellKind, SystemCommandKind};
use meow_app_launcher::search::{Action, ItemIcon, Scored, SearchItem};

/// 造一个系统指令条目喵
fn cmd_item(score: f32) -> Scored {
    Scored::new(
        score,
        SearchItem {
            title: "关机".into(),
            subtitle: None,
            icon: ItemIcon::Builtin(meow_app_launcher::search::BuiltinIcon::Power),
            action: Action::SystemCommand(SystemCommandKind::Shutdown),
        },
    )
}

/// 造一个应用条目喵
fn app_item(title: &str, score: f32) -> Scored {
    Scored::new(
        score,
        SearchItem {
            title: title.into(),
            subtitle: None,
            icon: ItemIcon::Builtin(meow_app_launcher::search::BuiltinIcon::Web),
            action: Action::OpenUrl("https://example.com".into()),
        },
    )
}

#[test]
fn 自定义指令也归入指令组置底喵() {
    let shell_item = Scored::new(
        5.0,
        SearchItem {
            title: "清DNS".into(),
            subtitle: None,
            icon: ItemIcon::Builtin(meow_app_launcher::search::BuiltinIcon::Power),
            action: Action::ShellCommand("ipconfig /flushdns".into()),
        },
    );
    let items = aggregate_items(vec![app_item("应用A", 1.0), shell_item, cmd_item(9.0)]);
    // 分组头 + 自定义指令 + 系统指令,应用被压在前面喵
    let sections: Vec<&str> = items
        .iter()
        .filter_map(|i| match i {
            ListItem::Section(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(sections, vec!["指令"], "指令(含自定义)单独成组喵");
}

#[test]
fn 指令模式由大于号前缀触发喵() {
    let none = command_mode_items("普通搜索");
    assert!(none.is_none(), "普通查询不进指令模式喵");

    let items = command_mode_items("> ipconfig /flushdns").unwrap();
    assert_eq!(items.len(), 2, "分组头 + 一条指令条目喵");
    match &items[1] {
        ListItem::Item(item) => match &item.action {
            Action::ShellCommand(script) => {
                assert_eq!(script, "ipconfig /flushdns");
                assert!(item.title.contains("ipconfig"), "标题要能看出要跑什么喵");
            }
            other => panic!("应是 ShellCommand 动作,实际 {other:?} 喵"),
        },
        other => panic!("应是条目,实际 {other:?} 喵"),
    }
}

#[test]
fn 指令模式空脚本给引导文案喵() {
    let items = command_mode_items("> ").unwrap();
    match &items[1] {
        ListItem::Item(item) => {
            assert!(item.title.contains("输入"), "空脚本时提示先输入指令喵");
        }
        _ => panic!("应是条目喵"),
    }
}

#[test]
fn shell档位与切换喵() {
    assert_eq!(ShellKind::default(), ShellKind::Powershell);
    assert_eq!(ShellKind::Powershell.label(), "PowerShell");
    assert_eq!(ShellKind::Cmd.label(), "Cmd");
    assert_eq!(ShellKind::Powershell.cycle(), ShellKind::Cmd);
    assert_eq!(ShellKind::Cmd.cycle(), ShellKind::Powershell);
}

#[test]
fn 系统命令类型五档轮换喵() {
    assert_eq!(SystemCommandKind::ALL.len(), 5);
    assert_eq!(SystemCommandKind::Custom.title(), "自定义");
    assert_eq!(SystemCommandKind::Restart.cycle(), SystemCommandKind::Custom);
    assert_eq!(SystemCommandKind::Custom.cycle(), SystemCommandKind::Lock);
}

#[test]
fn 旧版配置json缺字段可回填喵() {
    // v1.8 的配置里 CommandEntry 只有 aliases + kind,没有 script 喵
    let old = r#"{"aliases":["锁屏","lock"],"kind":"lock"}"#;
    let entry: CommandEntry = serde_json::from_str(old).unwrap();
    assert_eq!(entry.kind, SystemCommandKind::Lock);
    assert!(entry.script.is_empty(), "缺 script 回填空串喵");

    // 自定义指令的序列化往返喵
    let custom = CommandEntry::custom(vec!["清DNS".into()], "ipconfig /flushdns");
    let raw = serde_json::to_string(&custom).unwrap();
    let back: CommandEntry = serde_json::from_str(&raw).unwrap();
    assert_eq!(back, custom);
    assert!(raw.contains("\"custom\""), "kind 序列化为 custom 喵");
}

#[test]
fn 控制台输出解码_utf8直通喵() {
    use meow_app_launcher::platform::win32::decode_console_output;

    assert_eq!(decode_console_output(b""), "");
    assert_eq!(decode_console_output(b"plain ascii"), "plain ascii");
    assert_eq!(decode_console_output("中文输出喵".as_bytes()), "中文输出喵");
}

#[test]
fn 控制台输出解码_gbk不产生替换符喵() {
    use meow_app_launcher::platform::win32::decode_console_output;

    // "配置" 的 GBK 编码(中文系统 ipconfig 首行就是这类字节)喵
    let gbk = [0xC5u8, 0xE4, 0xD6, 0xC3];
    let text = decode_console_output(&gbk);
    assert!(!text.is_empty(), "GBK 字节应能解出内容喵");
    assert!(!text.contains('\u{FFFD}'), "按系统代码页解码不许出现替换符喵");

    // GBK 混合 ASCII(典型输出形态: "Windows IP 配置")喵
    let mixed = b"Windows IP \xC5\xE4\xD6\xC3";
    let text = decode_console_output(mixed);
    assert!(text.starts_with("Windows IP "), "ASCII 段原样保留喵");
    assert!(!text.contains('\u{FFFD}'), "混合编码也不许出现替换符喵");
}
