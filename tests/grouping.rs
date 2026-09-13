//! 搜索结果分组的纯函数测试喵~ 验证「指令与应用分离」的分组规则喵。

use meow_app_launcher::app::aggregate_items;
use meow_app_launcher::app::ListItem;
use meow_app_launcher::platform::SystemCommandKind;
use meow_app_launcher::search::{Action, Scored, SearchItem};

/// 造一个应用条目喵
fn app_item(title: &str, score: f32) -> Scored {
    Scored::new(
        score,
        SearchItem {
            title: title.into(),
            subtitle: None,
            icon: meow_app_launcher::search::ItemIcon::Builtin(
                meow_app_launcher::search::BuiltinIcon::Web,
            ),
            action: Action::OpenUrl("https://example.com".into()),
        },
    )
}

/// 造一个系统指令条目喵
fn cmd_item(title: &str, score: f32) -> Scored {
    Scored::new(
        score,
        SearchItem {
            title: title.into(),
            subtitle: None,
            icon: meow_app_launcher::search::ItemIcon::Builtin(
                meow_app_launcher::search::BuiltinIcon::Power,
            ),
            action: Action::SystemCommand(SystemCommandKind::Shutdown),
        },
    )
}

#[test]
fn 指令单独成组置底喵() {
    let items = aggregate_items(vec![
        app_item("应用A", 1.0),
        cmd_item("关机", 99.0), // 指令分再高也不许混进应用区喵
        app_item("应用B", 2.0),
        cmd_item("重启", 50.0),
    ]);
    let titles: Vec<&str> = items
        .iter()
        .map(|i| match i {
            ListItem::Section(s) => s.as_str(),
            ListItem::Item(it) => it.title.as_str(),
        })
        .collect();
    assert_eq!(
        titles,
        vec!["应用B", "应用A", "指令", "关机", "重启"],
        "应用按分排序在前,指令独立成组垫底(组内按分排序)喵"
    );
}

#[test]
fn 只有指令时也存在分组头喵() {
    let items = aggregate_items(vec![cmd_item("锁屏", 3.0)]);
    assert!(matches!(items.first(), Some(ListItem::Section(s)) if s == "指令"));
    assert_eq!(items.len(), 2);
}

#[test]
fn 无指令时不产生空分组喵() {
    let items = aggregate_items(vec![app_item("应用A", 1.0)]);
    assert_eq!(items.len(), 1);
    assert!(matches!(items.first(), Some(ListItem::Item(_))));
}
