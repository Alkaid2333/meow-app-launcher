//! 搜索逻辑喵~ 单元测试喵

use meow_app_launcher::app::config::AppConfig;
use meow_app_launcher::apps::{AppInfo, AppRegistry, AppSource};
use meow_app_launcher::search::{ParsedQuery, SearchEngine, SearchMode};

fn make_registry() -> AppRegistry {
    let mut reg = AppRegistry::default();
    reg.upsert(AppInfo {
        name: "谷歌浏览器".into(),
        path: "C:/chrome.exe".into(),
        icon_path: None,
        tags: vec!["浏览器".into(), "常用".into()],
        favorite: true,
        launch_count: 3,
        last_used: 100,
        source: AppSource::Manual,
    });
    reg.upsert(AppInfo {
        name: "Firefox".into(),
        path: "C:/firefox.exe".into(),
        icon_path: None,
        tags: vec!["浏览器".into()],
        favorite: false,
        launch_count: 1,
        last_used: 50,
        source: AppSource::Scanned,
    });
    reg.upsert(AppInfo {
        name: "Google Chrome".into(),
        path: "C:/chrome.exe".into(),
        icon_path: None,
        tags: vec!["浏览器".into()],
        favorite: false,
        launch_count: 2,
        last_used: 80,
        source: AppSource::Scanned,
    });
    reg.upsert(AppInfo {
        name: "终端".into(),
        path: "C:/terminal.exe".into(),
        icon_path: None,
        tags: vec!["开发".into()],
        favorite: false,
        launch_count: 0,
        last_used: 0,
        source: AppSource::Scanned,
    });
    reg
}

#[test]
fn search_by_english_name() {
    let reg = make_registry();
    let engine = SearchEngine::new();
    let cfg = AppConfig::default();
    let results = engine.search(&reg, &cfg, "fire");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Firefox");
}

#[test]
fn search_by_chinese_pinyin() {
    let reg = make_registry();
    let mut engine = SearchEngine::new();
    // 同步拼音索引(实际运行时 AppState 会做,测试里手动做喵)
    engine.sync(&reg.apps);
    let cfg = AppConfig::default();
    // "guge" 应命中「谷歌浏览器」喵
    let results = engine.search(&reg, &cfg, "guge");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "谷歌浏览器");
}

#[test]
fn search_by_initial_letters() {
    let reg = make_registry();
    let mut engine = SearchEngine::new();
    engine.sync(&reg.apps);
    let cfg = AppConfig::default();
    // "gg" 首字母 → 谷歌浏览器(ggllq)喵
    let results = engine.search(&reg, &cfg, "gg");
    assert!(results.iter().any(|a| a.name == "谷歌浏览器"));
}

#[test]
fn search_by_tag() {
    let reg = make_registry();
    let mut engine = SearchEngine::new();
    engine.sync(&reg.apps);
    let cfg = AppConfig::default();
    let results = engine.search(&reg, &cfg, "t: 浏览器");
    assert_eq!(results.len(), 3);
}

#[test]
fn search_by_initial_mode() {
    let reg = make_registry();
    let mut engine = SearchEngine::new();
    engine.sync(&reg.apps);
    let cfg = AppConfig::default();
    let results = engine.search(&reg, &cfg, "i: z");
    assert!(results.iter().any(|a| a.name == "终端"));
}

#[test]
fn parse_prefixes() {
    let t = ParsedQuery::parse("t: 浏览器");
    assert_eq!(t.mode, SearchMode::Tag);
    let i = ParsedQuery::parse("i: z");
    assert_eq!(i.mode, SearchMode::Initial);
    assert_eq!(i.initials, vec!['z']);
}
