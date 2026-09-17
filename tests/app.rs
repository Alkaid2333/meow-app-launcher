//! 应用级状态与列表逻辑喵~ 单元测试喵

use meow_app_launcher::app::config::{AppConfig, ElevateModifier, FilterRule, WindowConfig};
use meow_app_launcher::app::{browse_list, ListItem};
use meow_app_launcher::apps::{AppInfo, AppRegistry, AppSource};
use meow_app_launcher::platform::{Modifier, ModifierSet};

fn app(name: &str, fav: bool, count: u32, last: u64) -> AppInfo {
    AppInfo {
        name: name.into(),
        path: format!("{name}.exe"),
        icon_path: None,
        tags: Vec::new(),
        favorite: fav,
        launch_count: count,
        last_used: last,
        source: AppSource::Manual,
    }
}

#[test]
fn browse_list_respects_toggles() {
    let mut registry = AppRegistry::default();
    registry.apps = vec![app("Alpha", true, 5, 100), app("Beta", false, 1, 50)];
    let mut config = AppConfig::default();
    config.window = WindowConfig {
        show_recent: true,
        show_favorites: true,
        show_frequent: false,
        show_all: false,
        ..config.window
    };
    let items = browse_list(&registry, &config);
    let titles: Vec<_> = items
        .iter()
        .filter_map(|i| match i {
            ListItem::Section(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(titles, ["最近打开", "收藏"]);
}

#[test]
fn browse_list_filters_uninstall_keywords() {
    // 默认过滤规则应把「卸载 / uninstall」类启动项排除,不进入任何分组喵
    let mut registry = AppRegistry::default();
    registry.apps = vec![
        app("卸载助手", false, 0, 0),
        app("Firefox Uninstall", false, 0, 0),
        app("Firefox", false, 0, 0),
    ];
    let config = AppConfig::default();
    let items = browse_list(&registry, &config);
    let apps: Vec<&str> = items
        .iter()
        .filter_map(|i| match i {
            ListItem::Item(item) => Some(item.title.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(apps, vec!["Firefox"], "应只剩未被过滤的应用喵");
}

#[test]
fn filter_rule_case_sensitive() {
    // 大小写不敏感: 任意大小写都命中喵
    let loose = FilterRule {
        keyword: "uninstall".into(),
        case_sensitive: false,
    };
    assert!(loose.matches("Firefox Uninstall"));
    assert!(loose.matches("UNINSTALL_工具"));
    // 大小写敏感: 仅精确大小写命中喵
    let strict = FilterRule {
        keyword: "Uninstall".into(),
        case_sensitive: true,
    };
    assert!(strict.matches("Uninstall"));
    assert!(!strict.matches("uninstall"));
    // 空关键词永不命中喵
    assert!(!FilterRule::default().matches("任意应用"));
}

#[test]
fn 提权修饰键循环回到起点() {
    let mut m = ElevateModifier::default();
    assert_eq!(m, ElevateModifier::Shift, "默认可用的档位应是 Shift 喵");
    for _ in 0..ElevateModifier::ALL.len() {
        m = m.cycle();
    }
    assert_eq!(m, ElevateModifier::Shift, "循环一圈应回到起点喵");
}

#[test]
fn 提权修饰键档位与标签一一对应() {
    assert_eq!(
        ElevateModifier::Disabled.modifier(),
        None,
        "关闭档位不该绑定任何修饰键喵"
    );
    for m in ElevateModifier::ALL {
        assert!(!m.label().is_empty());
        assert_eq!(m.modifier().is_some(), m != ElevateModifier::Disabled);
    }
    /* 除关闭外每档都该落在不同修饰键上,不然切换毫无意义喵 */
    let keys: Vec<Modifier> = ElevateModifier::ALL
        .iter()
        .filter_map(|m| m.modifier())
        .collect();
    for (i, k) in keys.iter().enumerate() {
        assert!(!keys[i + 1..].contains(k), "{k:?} 重复绑定了喵");
    }
}

#[test]
fn 修饰键集合按位判定() {
    let mut set = ModifierSet::empty();
    assert!(set.is_empty());
    set.insert(Modifier::Shift);
    assert!(set.contains(Modifier::Shift));
    assert!(!set.contains(Modifier::Ctrl), "没按的键不能误判喵");
    set.insert(Modifier::Ctrl);
    assert!(set.contains(Modifier::Shift) && set.contains(Modifier::Ctrl));
    assert!(!set.is_empty());
}
