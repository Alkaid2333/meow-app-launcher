//! 内置搜索源喵: 计算器 / 系统命令 / Web 跳转喵。

use super::calc;
use super::fuzzy;
use super::item::{Action, BuiltinIcon, ItemIcon, Scored, SearchItem};
use crate::app::config::AppConfig;
use crate::platform::SystemCommandKind;

/// 提供者查询上下文喵
pub struct ProviderContext<'a> {
    /// 去空白后的原始查询喵
    pub query: &'a str,
    /// 全局配置喵
    pub config: &'a AppConfig,
}

/// 名称模式下注入的内置结果喵(计算器置顶、命令次之、Web 垫底)喵
pub fn extra_results(ctx: &ProviderContext) -> Vec<Scored> {
    let mut out = calc_query(ctx);
    out.extend(cmd_query(ctx));
    out.extend(web_query(ctx));
    out
}

fn calc_query(ctx: &ProviderContext) -> Vec<Scored> {
    let q = ctx.query;
    let Some(value) = calc::try_eval(q) else {
        return Vec::new();
    };
    let result = calc::format_number(value);
    log::debug!("计算器命中: {q} = {result} 喵");
    vec![Scored::new(
        3.0,
        SearchItem {
            title: result.clone(),
            subtitle: Some(format!("{q} = {result}")),
            icon: ItemIcon::Builtin(BuiltinIcon::Calculator),
            action: Action::CopyText(result),
        },
    )]
}

fn cmd_query(ctx: &ProviderContext) -> Vec<Scored> {
    let q = ctx.query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
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
            Scored::new(
                score + 2.0,
                SearchItem {
                    title: kind.title().to_string(),
                    subtitle: Some("系统命令,按 Enter 执行喵".to_string()),
                    icon: ItemIcon::Builtin(BuiltinIcon::Power),
                    action: Action::SystemCommand(kind),
                },
            )
        })
        .collect()
}

fn web_query(ctx: &ProviderContext) -> Vec<Scored> {
    let q = ctx.query.trim();
    if q.is_empty() {
        return Vec::new();
    }
    let engine = ctx.config.search.web_engine;
    let url = engine.search_url(q);
    vec![Scored::new(
        0.0,
        SearchItem {
            title: format!("用{}搜索「{q}」", engine.label()),
            subtitle: Some(url.clone()),
            icon: ItemIcon::Builtin(BuiltinIcon::Web),
            action: Action::OpenUrl(url),
        },
    )]
}
