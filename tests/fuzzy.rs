//! 模糊搜索逻辑喵~ 单元测试喵

use meow_app_launcher::search::fuzzy::fuzzy_match;

#[test]
fn exact_substring_scores_highest() {
    // 完全包含一定比零散命中分高喵!
    let sub = fuzzy_match("fire", "firefox").unwrap();
    let scattered = fuzzy_match("fix", "firefox").unwrap();
    assert!(sub.score > scattered.score);
}

#[test]
fn prefix_match_scores_high() {
    let prefix = fuzzy_match("chr", "chrome").unwrap();
    let later = fuzzy_match("ome", "chrome").unwrap();
    assert!(prefix.score > later.score);
}

#[test]
fn no_match_returns_none() {
    assert!(fuzzy_match("xyz", "chrome").is_none());
}

#[test]
fn chinese_match_works() {
    // 中文子序列匹配也要能用喵!
    assert!(fuzzy_match("浏览器", "谷歌浏览器").is_some());
    assert!(fuzzy_match("谷歌", "谷歌浏览器").is_some());
}

#[test]
fn out_of_order_fails() {
    // 顺序错乱就不该匹配喵
    assert!(fuzzy_match("meh", "chrome").is_none());
}

#[test]
fn empty_query_matches_low() {
    // 空查询谁都匹配但分数极低喵
    let r = fuzzy_match("", "anything").unwrap();
    assert!(r.score < 0.1);
}
