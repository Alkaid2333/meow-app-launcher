//! 环境变量同步的纯逻辑喵~
//!
//! 常驻进程的环境块是「启动那一刻」的快照,用户之后改的环境变量不会自动进来喵。
//! 平台实现只负责两件有副作用的事——读注册表、写进程环境;
//! 而合并 / 展开 / 差量这三件纯计算放在这里,可以脱开 Windows 直接单测喵。

use std::collections::{HashMap, HashSet};

/// 展开 `%引用%` 的最大递归深度喵(注册表被玩坏时也不能把进程转死)喵
const MAX_EXPAND_DEPTH: usize = 8;

/// 系统环境键里这些变量属于「会话级」变量喵
///
/// 机器键里的 `TEMP`/`TMP` 指向 `%SystemRoot%\TEMP`(给服务用的),
/// 和当前用户会话里的临时目录不是一回事,用户键没显式覆盖时就不采用喵。
const SESSION_OWNED_VARS: [&str; 2] = ["TEMP", "TMP"];

/// 注册表里读到的一条环境变量喵
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvEntry {
    /// 变量名(保持注册表原样)喵
    pub name: String,
    /// 变量值(可能仍含 `%引用%`)喵
    pub value: String,
    /// 是否需要展开 `%引用%`(即注册表类型为 REG_EXPAND_SZ)喵
    pub expand: bool,
}

impl EnvEntry {
    /// 造一条可展开的变量喵(测试与外部构造用)喵
    pub fn expanding(name: &str, value: &str) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            expand: true,
        }
    }
}

/// 变量名归一化喵(Windows 变量名大小写不敏感,统一大写才能正确合并)喵
pub fn normalize(name: &str) -> String {
    name.to_ascii_uppercase()
}

/// 环境变量查找表喵(键已归一化)喵
pub type EnvMap = HashMap<String, String>;

/// 系统 / 用户两级环境变量合并喵
///
/// * 同名时用户值覆盖系统值(与登录环境的两级覆盖一致)喵;
/// * `PATH` 是唯一例外: Windows 把用户 PATH **拼接**在系统 PATH 之后,
///   若按覆盖处理,系统 PATH 会整段丢失,子进程将找不到任何系统命令喵;
/// * 会话级变量(临时目录等)只在用户键给出时才采用,细节见 [`SESSION_OWNED_VARS`] 喵。
pub fn merge(system: &[EnvEntry], user: &[EnvEntry]) -> Vec<EnvEntry> {
    let mut merged: Vec<EnvEntry> = Vec::with_capacity(system.len() + user.len());
    let mut index: HashMap<String, usize> = HashMap::new();

    for entry in system {
        let key = normalize(&entry.name);
        if SESSION_OWNED_VARS.contains(&key.as_str()) {
            continue;
        }
        match index.get(&key).copied() {
            Some(i) => merged[i] = entry.clone(),
            None => {
                index.insert(key, merged.len());
                merged.push(entry.clone());
            }
        }
    }

    for entry in user {
        let key = normalize(&entry.name);
        let Some(i) = index.get(&key).copied() else {
            index.insert(key, merged.len());
            merged.push(entry.clone());
            continue;
        };
        let prev = &merged[i];
        // PATH 拼接,其余用户值直接顶掉系统值喵
        let value = if key == "PATH" && !prev.value.trim().is_empty() {
            join_path(&prev.value, &entry.value)
        } else {
            entry.value.clone()
        };
        merged[i] = EnvEntry {
            name: entry.name.clone(),
            value,
            expand: prev.expand || entry.expand,
        };
    }

    merged
}

/// 用 `;` 拼接两段 PATH,顺手消化重复的分隔符喵
fn join_path(system: &str, user: &str) -> String {
    let head = system.trim_end_matches(';');
    let tail = user.trim_start_matches(';');
    match (head.is_empty(), tail.is_empty()) {
        (true, _) => tail.to_string(),
        (_, true) => head.to_string(),
        _ => format!("{head};{tail}"),
    }
}

/// 把 `%VAR%` 全部替换掉喵
///
/// 找不到的变量保留字面量——与 Windows 展开失败时的行为一致,
/// 半个 `%` 也原样吐回去,绝不吞字符喵。
pub fn expand(value: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match lookup(name) {
                    Some(resolved) => out.push_str(&resolved),
                    None => out.push_str(&rest[start..start + 1 + end + 1]),
                }
                rest = &after[end + 1..];
            }
            // 半个 % 或 %%(空变量名): 原样收下,继续往后找喵
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// 把带 `%引用%` 的变量按需展开,得到最终的可写值表喵
///
/// 只有 `expand` 标记为真的变量会被展开(REG_SZ 的 `%foo%` 在 Windows 里也是字面量)喵;
/// 展开时递归解析引用,`fallback` 用于兜住注册表里没写、但当前进程已存在的变量喵。
pub fn expand_all(entries: &[EnvEntry], fallback: impl Fn(&str) -> Option<String>) -> EnvMap {
    let raw: EnvMap = entries
        .iter()
        .map(|e| (normalize(&e.name), e.value.clone()))
        .collect();
    let resolve = |name: &str| resolve_ref(name, &raw, &fallback, MAX_EXPAND_DEPTH);
    entries
        .iter()
        .map(|e| {
            let value = if e.expand {
                expand(&e.value, resolve)
            } else {
                e.value.clone()
            };
            (normalize(&e.name), value)
        })
        .collect()
}

/// 解析一次 `%引用%`: 注册表优先,找不到再问进程环境喵
fn resolve_ref(
    name: &str,
    raw: &EnvMap,
    fallback: &impl Fn(&str) -> Option<String>,
    depth: usize,
) -> Option<String> {
    if depth == 0 {
        return None; // 自我引用兜底: 到顶就退化成字面量喵
    }
    if let Some(value) = raw.get(&normalize(name)) {
        let nested = |n: &str| resolve_ref(n, raw, fallback, depth - 1);
        return Some(expand(value, nested));
    }
    fallback(name)
}

/// 写入计划喵: 要落地的变量 + 要从进程环境里抹掉的变量喵
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EnvPlan {
    /// 变量名已归一化,值可直接写进进程环境喵
    pub set: Vec<(String, String)>,
    /// 已归一化的变量名喵
    pub remove: Vec<String>,
}

impl EnvPlan {
    /// 没有任何变更喵
    pub fn is_empty(&self) -> bool {
        self.set.is_empty() && self.remove.is_empty()
    }
}

/// 增量同步状态喵
///
/// 记住上一轮同步过的变量名,才能识别「用户把某个变量删了」——
/// 少了这份记忆,删除操作永远传不到子进程喵。
#[derive(Debug, Default)]
pub struct EnvSync {
    synced: HashSet<String>,
}

impl EnvSync {
    /// 空状态: 一次都没同步过喵
    pub fn new() -> Self {
        Self {
            synced: HashSet::new(),
        }
    }

    /// 算出把进程环境对齐到 `fresh` 需要做的改动,并记住这一轮的名字喵
    pub fn plan(&mut self, fresh: &EnvMap) -> EnvPlan {
        let mut set: Vec<(String, String)> =
            fresh.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        // 顺序稳定,日志与排障都可复现喵
        set.sort();

        let mut remove: Vec<String> = self
            .synced
            .iter()
            .filter(|k| !fresh.contains_key(*k))
            .cloned()
            .collect();
        remove.sort();

        self.synced = fresh.keys().cloned().collect();
        EnvPlan { set, remove }
    }
}
