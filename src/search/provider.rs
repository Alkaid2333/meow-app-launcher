//! 内置搜索源喵: 计算器 / 系统命令 / Web 跳转喵。

use super::calc;
use super::fuzzy;
use super::item::{Action, BuiltinIcon, ItemIcon, Scored, SearchItem};
use crate::app::config::{AppConfig, CommandEntry};
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
    let mut best_per_entry: Vec<(usize, f32)> = Vec::new();
    for (index, entry) in ctx.config.search.commands.iter().enumerate() {
        for alias in &entry.aliases {
            let alias = alias.trim().to_lowercase();
            if alias.is_empty() {
                continue;
            }
            let Some(m) = fuzzy::fuzzy_match(&q, &alias) else {
                continue;
            };
            match best_per_entry.iter_mut().find(|(i, _)| *i == index) {
                Some(slot) => slot.1 = slot.1.max(m.score),
                None => best_per_entry.push((index, m.score)),
            }
        }
    }

    best_per_entry
        .into_iter()
        .map(|(index, score)| command_item(&ctx.config.search.commands[index], score))
        .collect()
}

/// 把一条指令配置变成搜索结果条目喵(系统命令与自定义 shell 指令通用)喵
fn command_item(entry: &CommandEntry, score: f32) -> Scored {
    if entry.kind == SystemCommandKind::Custom {
        // 自定义指令: 标题取首个别名(没有就用脚本前段),正文展示完整脚本喵
        let title = entry
            .aliases
            .first()
            .map(|a| a.trim())
            .filter(|a| !a.is_empty())
            .map(|a| a.to_string())
            .unwrap_or_else(|| summarize_script(&entry.script));
        Scored::new(
            score + 2.0,
            SearchItem {
                title,
                subtitle: Some(entry.script.clone()),
                icon: ItemIcon::Builtin(BuiltinIcon::Power),
                action: Action::ShellCommand(entry.script.clone()),
            },
        )
    } else {
        Scored::new(
            score + 2.0,
            SearchItem {
                title: entry.kind.title().to_string(),
                subtitle: Some("系统命令,按 Enter 执行喵".to_string()),
                icon: ItemIcon::Builtin(BuiltinIcon::Power),
                action: Action::SystemCommand(entry.kind),
            },
        )
    }
}

/// 没有别名时给自定义指令起个短标题喵(截前 16 字符)喵
fn summarize_script(script: &str) -> String {
    let script = script.trim();
    let mut head: String = script.chars().take(16).collect();
    if script.chars().count() > 16 {
        head.push('…');
    }
    head.to_string()
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
