//! 模糊搜索逻辑喵~
//!
//! 纯 Rust 实现,零 GPUI 依赖,可单测喵!
//! 支持: 英文、中文、拼音的子序列模糊匹配,并带打分排序喵。
//!
//! 匹配思想: 用户输入的关键字按顺序出现在目标字符串中即可命中,
//! 越靠前、越连续、完全包含的分数越高,用来自动聚焦「最匹配」的喵。

/// 一次模糊匹配的结果喵
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchResult {
    /// 匹配分数,越高越匹配(0.0 = 不匹配)喵
    pub score: f32,
}

/// 判断 `query` 是否是 `target` 的子序列,并计算匹配分数喵~
///
/// 打分规则:
/// * 完全包含(子串): 基础分 + 大额奖励
/// * 从头开始匹配: 加分
/// * 匹配到的字符越紧凑(跳跃越少): 分越高
/// * 命中的字符越靠前: 分越高
pub fn fuzzy_match(query: &str, target: &str) -> Option<MatchResult> {
    let query: Vec<char> = query.chars().collect();
    let target: Vec<char> = target.chars().collect();

    // 空查询谁都匹配,但分数极低(不干扰排序喵)
    if query.is_empty() {
        return Some(MatchResult { score: 0.01 });
    }
    // 查询比目标还长,肯定没戏喵
    if query.len() > target.len() {
        return None;
    }

    let mut score = 0.0_f32;
    let mut qi = 0;
    let mut last_match_idx = None::<usize>;
    let mut matches = Vec::with_capacity(query.len());

    for (ti, &tc) in target.iter().enumerate() {
        if qi < query.len() && tc == query[qi] {
            matches.push(ti);
            // 连续匹配奖励: 紧挨着上一个命中,分数高喵
            if let Some(prev) = last_match_idx
                && ti == prev + 1 {
                    score += 3.0;
                }
            // 开头匹配奖励: 命中越靠前越好喵
            if ti < 3 {
                score += 2.0;
            }
            last_match_idx = Some(ti);
            qi += 1;
            if qi == query.len() {
                break;
            }
        }
    }

    // 没匹配完所有查询字符,失败喵
    if qi < query.len() {
        return None;
    }

    // 完全包含(子串)大奖励: query 在 target 里连续出现喵
    let sub = query.iter().collect::<String>();
    if target.iter().collect::<String>().contains(&sub) {
        score += 50.0;
    }
    // 从头开始匹配奖励喵
    if matches[0] == 0 {
        score += 10.0;
    }
    // 命中紧凑度: 总跨度越小分越高喵
    let span = matches.last().unwrap() - matches[0];
    score += 20.0 / (1.0 + span as f32);

    Some(MatchResult { score })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
