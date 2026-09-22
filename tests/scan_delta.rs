//! 应用扫描增减量的测试喵~ 验证注册表与磁盘扫描结果的双向同步喵。

use meow_app_launcher::apps::{AppInfo, AppRegistry, AppSource, ScanDelta};

/// 造一个扫描来源的应用喵
fn scanned(name: &str, path: &str) -> AppInfo {
    AppInfo {
        name: name.into(),
        path: path.into(),
        icon_path: None,
        tags: Vec::new(),
        favorite: false,
        launch_count: 0,
        last_used: 0,
        source: AppSource::Scanned,
    }
}

/// 造一个手动注册的应用喵
fn manual(name: &str) -> AppInfo {
    AppInfo::manual(name, format!("C:/manual/{name}.exe"))
}

#[test]
fn 首轮扫描全部算新增喵() {
    let mut reg = AppRegistry::default();
    let delta = reg.merge_scanned(vec![scanned("微信", "C:/w.lnk"), scanned("QQ", "C:/q.lnk")]);
    assert_eq!(delta.added, 2);
    assert_eq!(delta.removed, 0);
    assert_eq!(reg.apps.len(), 2);
}

#[test]
fn 卸载的应用会被减量移除喵() {
    let mut reg = AppRegistry::default();
    reg.merge_scanned(vec![scanned("微信", "C:/w.lnk"), scanned("QQ", "C:/q.lnk")]);

    // 微信被卸载: 本轮扫描集里只剩 QQ 喵
    let delta = reg.merge_scanned(vec![scanned("QQ", "C:/q.lnk")]);
    assert_eq!(delta.added, 0);
    assert_eq!(delta.removed, 1);
    assert_eq!(delta.removed_names, vec!["微信".to_string()]);
    assert!(reg.find("微信").is_none(), "卸载的应用必须从注册表消失喵");
    assert!(reg.find("QQ").is_some());
}

#[test]
fn 手动注册的应用不受减量影响喵() {
    let mut reg = AppRegistry::default();
    reg.merge_scanned(vec![scanned("微信", "C:/w.lnk")]);
    reg.upsert(manual("我的工具"));

    // 手动应用不在扫描集里也不许被删喵
    let delta = reg.merge_scanned(Vec::new());
    assert_eq!(delta.removed, 1, "只有扫描来源的微信被移除喵");
    assert_eq!(delta.removed_names, vec!["微信".to_string()]);
    assert!(reg.find("我的工具").is_some(), "手动注册的应用必须保留喵");
}

#[test]
fn 同名应用路径更新时保留用户信息喵() {
    let mut reg = AppRegistry::default();
    reg.merge_scanned(vec![scanned("微信", "C:/old/WeChat.lnk")]);
    // 用户给微信打了收藏喵
    if let Some(app) = reg.apps.first_mut() {
        app.favorite = true;
    }

    reg.merge_scanned(vec![scanned("微信", "C:/new/WeChat.lnk")]);
    let app = reg.find("微信").unwrap();
    assert_eq!(app.path, "C:/new/WeChat.lnk", "路径要跟随扫描更新喵");
    assert!(app.favorite, "用户附加信息(收藏)不许丢喵");
}

#[test]
fn 增减统计与摘要喵() {
    let empty = ScanDelta::default();
    assert!(!empty.any());
    assert_eq!(empty.summary(), "新增 0 个,移除 0 个应用喵");

    let both = ScanDelta {
        added: 3,
        removed: 1,
        removed_names: vec!["旧应用".into()],
    };
    assert!(both.any());
    assert_eq!(both.summary(), "新增 3 个,移除 1 个应用喵");
}

#[test]
fn 重复扫描无变化喵() {
    let mut reg = AppRegistry::default();
    let first = vec![scanned("微信", "C:/w.lnk"), scanned("QQ", "C:/q.lnk")];
    reg.merge_scanned(first.clone());
    let delta = reg.merge_scanned(first);
    assert_eq!(delta.added, 0, "重复扫描不该重复注册喵");
    assert_eq!(delta.removed, 0);
    assert!(!delta.any());
}
