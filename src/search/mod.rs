//! 搜索逻辑喵~
//!
//! 把模糊匹配、拼音索引、模式解析组合成完整搜索能力喵。
//! 支持三种模式喵:
//! * 名称模式(默认): 模糊匹配名称/拼音喵
//! * Tag 模式: `t: xxx` 按标签搜索喵
//! * 首字母模式: `i: a b c` 按首字母搜索喵

pub mod calc;
pub mod fuzzy;
pub mod item;
pub mod provider;

pub use item::{Action, BuiltinIcon, ItemIcon, Scored, SearchItem};
pub use provider::{builtin_providers, ProviderContext, SearchProvider};

use crate::apps::AppInfo;
use pinyin::ToPinyin;
use std::collections::HashMap;

/// 搜索结果条数上限喵
pub const MAX_RESULTS: usize = 30;

/// 搜索模式喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// 按名称模糊搜索喵
    Name,
    /// 按 Tag 搜索喵
    Tag,
    /// 按首字母搜索喵
    Initial,
}

/// 判定查询属于哪种模式喵(多源聚合只在名称模式下注入计算器/Web/命令)喵
pub fn query_mode(input: &str) -> SearchMode {
    ParsedQuery::parse(input).mode
}

/// 解析后的查询喵
#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub mode: SearchMode,
    /// 去掉前缀后的关键字喵
    pub keywords: String,
    /// 首字母模式的多个字母喵
    pub initials: Vec<char>,
}

impl ParsedQuery {
    /// 解析用户输入喵,支持 `t:` / `i:` 前缀喵
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        if let Some(tag) = trimmed.strip_prefix("t:").or_else(|| trimmed.strip_prefix("T:")) {
            return Self {
                mode: SearchMode::Tag,
                keywords: tag.trim().to_string(),
                initials: Vec::new(),
            };
        }
        if let Some(init) = trimmed.strip_prefix("i:").or_else(|| trimmed.strip_prefix("I:")) {
            let initials: Vec<char> = init
                .split_whitespace()
                .filter_map(|w| w.chars().next())
                .collect();
            return Self {
                mode: SearchMode::Initial,
                keywords: init.trim().to_string(),
                initials,
            };
        }
        Self {
            mode: SearchMode::Name,
            keywords: trimmed.to_string(),
            initials: Vec::new(),
        }
    }

    /// 查询是否为空喵
    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty()
    }
}

/// 拼音索引缓存喵(应用名 → 拼音数据)
#[derive(Debug, Default)]
pub struct PinyinIndex {
    /// 全拼索引: 应用名 → 不带声调的全拼喵
    full: HashMap<String, String>,
    /// 首字母索引: 应用名 → 各字首字母喵
    initials: HashMap<String, String>,
}

impl PinyinIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// 为注册表建立/更新拼音索引喵
    pub fn rebuild(&mut self, apps: &[AppInfo]) {
        self.full.clear();
        self.initials.clear();
        for app in apps {
            self.full.insert(app.name.clone(), to_pinyin_full(&app.name));
            self.initials.insert(app.name.clone(), to_pinyin_initials(&app.name));
        }
        log::debug!("拼音索引重建完成: {} 个应用喵", self.full.len());
    }

    fn get_full(&self, name: &str) -> &str {
        self.full.get(name).map(|s| s.as_str()).unwrap_or("")
    }

    fn get_initials(&self, name: &str) -> &str {
        self.initials.get(name).map(|s| s.as_str()).unwrap_or("")
    }
}

/// 计算名称的全拼(不带声调)喵,非汉字原样保留喵
pub fn to_pinyin_full(name: &str) -> String {
    name.chars()
        .map(|c| match c.to_pinyin() {
            Some(p) => p.plain().to_string(),
            None => c.to_string(),
        })
        .collect()
}

/// 计算名称的拼音首字母喵,非汉字取字符本身喵
pub fn to_pinyin_initials(name: &str) -> String {
    name.chars()
        .map(|c| match c.to_pinyin() {
            Some(p) => p.first_letter().to_string(),
            None => c.to_ascii_lowercase().to_string(),
        })
        .collect()
}

/// 搜索引擎喵
pub struct SearchEngine {
    pinyin: PinyinIndex,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            pinyin: PinyinIndex::new(),
        }
    }

    /// 同步拼音索引喵(注册表变化后调用)喵
    pub fn sync(&mut self, apps: &[AppInfo]) {
        self.pinyin.rebuild(apps);
    }

    /// 执行搜索喵,返回按分数降序的应用引用喵
    ///
    /// * 空查询: 按配置返回推荐区(最近/收藏/最常用/全部)——由视图层决定,这里返回空喵
    /// * 非空查询: 模糊匹配名称、全拼、首字母,取最高分喵
    /// * 命中过滤规则的应用一律排除喵
    pub fn search<'a>(
        &self,
        registry: &'a crate::apps::AppRegistry,
        config: &crate::app::config::AppConfig,
        input: &str,
    ) -> Vec<&'a AppInfo> {
        self.search_scored(registry, config, input)
            .into_iter()
            .map(|(app, _)| app)
            .collect()
    }

    /// 带分数版搜索喵(多源聚合需要分数混排;不做截断,由聚合层统一收口)喵
    pub fn search_scored<'a>(
        &self,
        registry: &'a crate::apps::AppRegistry,
        config: &crate::app::config::AppConfig,
        input: &str,
    ) -> Vec<(&'a AppInfo, f32)> {
        let parsed = ParsedQuery::parse(input);
        if parsed.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(&AppInfo, f32)> = Vec::new();
        for app in &registry.apps {
            if config.is_app_filtered(&app.name) {
                continue;
            }
            if let Some(score) = self.score_app(app, &parsed) {
                scored.push((app, score));
            }
        }
        // 按分数降序,分数相同按名称排序保证稳定喵
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.name.cmp(&b.0.name))
        });
        scored
    }

    /// 计算单个应用对查询的匹配分数喵
    fn score_app(&self, app: &AppInfo, parsed: &ParsedQuery) -> Option<f32> {
        match parsed.mode {
            SearchMode::Name => {
                let q = parsed.keywords.trim().to_lowercase();
                if q.is_empty() {
                    return None;
                }
                let mut best = 0.0_f32;

                // 1. 名称直接匹配喵
                if let Some(m) = fuzzy::fuzzy_match(&q, &app.name.to_lowercase()) {
                    best = best.max(m.score);
                }
                // 2. 全拼匹配喵
                let full = self.pinyin.get_full(&app.name).to_lowercase();
                if !full.is_empty()
                    && let Some(m) = fuzzy::fuzzy_match(&q, &full) {
                        best = best.max(m.score);
                    }
                // 3. 首字母匹配喵
                let initials = self.pinyin.get_initials(&app.name).to_lowercase();
                if !initials.is_empty() && !q.is_empty()
                    && let Some(m) = fuzzy::fuzzy_match(&q, &initials) {
                        best = best.max(m.score);
                    }

                if best > 0.0 {
                    Some(best)
                } else {
                    None
                }
            }
            SearchMode::Tag => {
                let q = parsed.keywords.trim().to_lowercase();
                if q.is_empty() {
                    return None;
                }
                // Tag 支持模糊匹配喵
                let matched = app
                    .tags
                    .iter()
                    .any(|tag| fuzzy::fuzzy_match(&q, &tag.to_lowercase()).is_some());
                matched.then_some(60.0)
            }
            SearchMode::Initial => {
                if parsed.initials.is_empty() {
                    return None;
                }
                let initials = self.pinyin.get_initials(&app.name).to_lowercase();
                // 每个首字母都要在索引里命中喵
                let all = parsed
                    .initials
                    .iter()
                    .all(|ch| initials.contains(ch.to_ascii_lowercase()));
                all.then_some(70.0)
            }
        }
    }
}
