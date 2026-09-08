//! 搜索源提供者喵~
//!
//! 按 D3 拍板「内置 Provider trait 先行」: 计算器/系统命令/Web 跳转
//! 全部以内部插件形式接入,把 trait 接口打磨稳,
//! 未来外部插件(Rhai/WASM)直接复用同一道接口喵。

use super::calc;
use super::fuzzy;
use super::item::{Action, BuiltinIcon, ItemIcon, Scored, SearchItem};
use crate::app::config::AppConfig;
use crate::platform::SystemCommandKind;

/// 提供者查询上下文喵(字段只增不改,保证后续兼容)喵
pub struct ProviderContext<'a> {
    /// 去空白后的原始查询喵
    pub query: &'a str,
    /// 全局配置喵
    pub config: &'a AppConfig,
}

/// 搜索源接口喵: 同步返回结果,必须毫秒级完成(慢源将来走异步流)喵
pub trait SearchProvider: Send + Sync {
    /// 来源名喵(调试/日志用)
    fn name(&self) -> &'static str;

    /// 对查询给出带分数的结果喵
    fn query(&self, ctx: &ProviderContext) -> Vec<Scored>;
}

/// 全部内置搜索源喵(顺序即默认同分顺序)喵
pub fn builtin_providers() -> Vec<Box<dyn SearchProvider>> {
    vec![
        Box::new(CalcProvider),
        Box::new(CmdProvider),
        Box::new(WebProvider),
    ]
}

// ---------------------------------------------------------------------------
// 计算器源喵
// ---------------------------------------------------------------------------

/// 算式求值源喵: 查询是合法算式时给出结果(Enter 复制)喵
pub struct CalcProvider;

impl SearchProvider for CalcProvider {
    fn name(&self) -> &'static str {
        "calculator"
    }

    fn query(&self, ctx: &ProviderContext) -> Vec<Scored> {
        let q = ctx.query;
        let Some(value) = calc::try_eval(q) else {
            return Vec::new();
        };
        let result = calc::format_number(value);
        log::debug!("计算器命中: {q} = {result} 喵");
        let item = SearchItem {
            title: result.clone(),
            subtitle: Some(format!("{q} = {result}")),
            icon: ItemIcon::Builtin(BuiltinIcon::Calculator),
            action: Action::CopyText(result),
        };
        // 算式意图极其明确,分数给足保证置顶喵
        vec![Scored::new(3.0, item)]
    }
}

// ---------------------------------------------------------------------------
// 系统命令源喵
// ---------------------------------------------------------------------------

/// 系统命令源喵: 按配置的指令别名表匹配(中英文/自定义 alias 皆可)喵
pub struct CmdProvider;

impl SearchProvider for CmdProvider {
    fn name(&self) -> &'static str {
        "system-command"
    }

    fn query(&self, ctx: &ProviderContext) -> Vec<Scored> {
        let q = ctx.query.trim().to_lowercase();
        if q.is_empty() {
            return Vec::new();
        }
        // 每种命令只留最高分命中(同命令的多个别名互相竞争)喵
        let mut best_per_kind: Vec<(SystemCommandKind, f32)> = Vec::new();
        for entry in &ctx.config.search.commands {
            for alias in &entry.aliases {
                let alias = alias.trim().to_lowercase();
                if alias.is_empty() {
                    continue;
                }
                let Some(m) = fuzzy::fuzzy_match(&q, &alias) else {
                    continue;
                };
                match best_per_kind.iter_mut().find(|(k, _)| *k == entry.kind) {
                    Some(slot) => slot.1 = slot.1.max(m.score),
                    None => best_per_kind.push((entry.kind, m.score)),
                }
            }
        }

        best_per_kind
            .into_iter()
            .map(|(kind, score)| {
                let item = SearchItem {
                    title: kind.title().to_string(),
                    subtitle: Some("系统命令,按 Enter 执行喵".to_string()),
                    icon: ItemIcon::Builtin(BuiltinIcon::Power),
                    action: Action::SystemCommand(kind),
                };
                // 加成保证压过普通应用名命中喵
                Scored::new(score + 2.0, item)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Web 搜索跳转源喵
// ---------------------------------------------------------------------------

/// Web 搜索跳转源喵: 任意非空查询都给一条「用 XX 搜索」垫底喵
pub struct WebProvider;

impl SearchProvider for WebProvider {
    fn name(&self) -> &'static str {
        "web"
    }

    fn query(&self, ctx: &ProviderContext) -> Vec<Scored> {
        let q = ctx.query.trim();
        if q.is_empty() {
            return Vec::new();
        }
        let engine = ctx.config.search.web_engine;
        let url = engine.search_url(q);
        let item = SearchItem {
            title: format!("用{}搜索「{q}」", engine.label()),
            subtitle: Some(url.clone()),
            icon: ItemIcon::Builtin(BuiltinIcon::Web),
            action: Action::OpenUrl(url),
        };
        // 垫底分: 有应用命中就不抢位,没有也能兜住喵
        vec![Scored::new(0.0, item)]
    }
}
