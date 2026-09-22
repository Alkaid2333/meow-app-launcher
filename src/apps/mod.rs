//! 应用注册表喵~
//!
//! 负责: 应用的注册、扫描、持久化、查询喵。
//! 注册的应用列表持久化到 `./.datas/apps.json` 喵。

pub mod icon;
pub mod scanner;

use crate::app::config::APPS_FILE_NAME;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 应用来源喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppSource {
    /// 自动扫描注册喵
    Scanned,
    /// 手动注册喵
    Manual,
}

/// 注册的应用喵
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppInfo {
    /// 应用名称喵
    pub name: String,
    /// 应用路径(可执行文件/快捷方式/链接)喵
    pub path: String,
    /// 自定义图标路径(可选,为空则自动解析)喵
    pub icon_path: Option<String>,
    /// Tag 分类喵
    #[serde(default)]
    pub tags: Vec<String>,
    /// 是否收藏喵
    #[serde(default)]
    pub favorite: bool,
    /// 启动次数(用于最常用统计)喵
    #[serde(default)]
    pub launch_count: u32,
    /// 最近使用时间戳(Unix 秒,0 表示从未)喵
    #[serde(default)]
    pub last_used: u64,
    /// 来源喵
    pub source: AppSource,
}

impl AppInfo {
    /// 新建一个手动注册的应用喵
    pub fn manual(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            icon_path: None,
            tags: Vec::new(),
            favorite: false,
            launch_count: 0,
            last_used: 0,
            source: AppSource::Manual,
        }
    }

    /// 图标快照文件名(数据目录 icons 下的文件名)喵
    pub fn icon_snapshot_name(&self) -> String {
        format!("{}.png", sanitize_file_name(&self.name))
    }

    /// 记录一次启动喵,返回更新后的计数器喵
    pub fn bump_launch(&mut self, now_secs: u64) {
        self.launch_count = self.launch_count.saturating_add(1);
        self.last_used = now_secs;
    }
}

/// 应用注册表喵
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppRegistry {
    /// 全部注册应用喵
    pub apps: Vec<AppInfo>,
}

/// 一轮扫描的增减量统计喵
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScanDelta {
    /// 新注册的应用数喵
    pub added: usize,
    /// 因消失而移除的应用数喵(卸载/快捷方式被删)喵
    pub removed: usize,
    /// 被移除的应用名喵(供调用方清理图标快照)喵
    pub removed_names: Vec<String>,
}

impl ScanDelta {
    /// 有任何变化吗喵
    pub fn any(&self) -> bool {
        self.added > 0 || self.removed > 0
    }

    /// 给通知看的一句话摘要喵
    pub fn summary(&self) -> String {
        format!("新增 {} 个,移除 {} 个应用喵", self.added, self.removed)
    }
}

impl AppRegistry {
    /// 从磁盘加载应用注册表喵,失败则空表喵~
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join(APPS_FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<AppRegistry>(&raw) {
                Ok(reg) => {
                    log::debug!("应用注册表加载成功: {} 个应用喵", reg.apps.len());
                    reg
                }
                Err(e) => {
                    log::warn!("应用注册表解析失败({e}),使用空注册表喵~");
                    Self::default()
                }
            },
            Err(_) => {
                log::info!("应用注册表不存在,首次启动喵~");
                Self::default()
            }
        }
    }

    /// 保存应用注册表到磁盘喵~
    pub fn save(&self, data_dir: &Path) {
        let path = data_dir.join(APPS_FILE_NAME);
        match serde_json::to_string_pretty(self) {
            Ok(raw) => match std::fs::write(&path, raw) {
                Ok(_) => log::debug!("应用注册表已保存: {} 个应用喵", self.apps.len()),
                Err(e) => log::error!("应用注册表保存失败: {e}"),
            },
            Err(e) => log::error!("应用注册表序列化失败: {e}"),
        }
    }

    /// 注册(或更新同名)一个应用喵,返回是否新增喵
    pub fn upsert(&mut self, app: AppInfo) -> bool {
        let same_name = self.apps.iter().position(|a| a.name == app.name);
        match same_name {
            Some(idx) => {
                log::debug!("更新应用: {} 喵", app.name);
                self.apps[idx] = app;
                false
            }
            None => {
                log::debug!("注册新应用: {} 喵", app.name);
                self.apps.push(app);
                true
            }
        }
    }

    /// 移除一个应用喵,返回是否成功喵
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.apps.len();
        self.apps.retain(|a| a.name != name);
        let removed = self.apps.len() < before;
        if removed {
            log::debug!("移除应用: {name} 喵");
        }
        removed
    }

    /// 按名称查找应用喵
    pub fn find(&self, name: &str) -> Option<&AppInfo> {
        self.apps.iter().find(|a| a.name == name)
    }

    /// 合并扫描结果进注册表喵(增量注册 + 减量移除,双向同步)喵
    ///
    /// * 增量: 扫描集里出现的新名字 → 注册喵
    /// * 更新: 同名应用的路径变了 → 跟随更新(保留 tag/收藏/计数等用户信息)喵
    /// * 减量: 已注册的扫描来源应用,**名字不在本轮扫描集里** → 移除
    ///   (应用被卸载或快捷方式被手动删除后,注册表不再留孤儿喵)
    ///
    /// 减量比较基于传入的**完整扫描集**: 上层因屏蔽词拦下的条目不算消失,
    /// 否则加个屏蔽词就会把正常应用从注册表里误删喵。
    /// 返回增减量统计;调用方负责对被移除的应用做图标清理喵。
    pub fn merge_scanned(&mut self, scanned: Vec<AppInfo>) -> ScanDelta {
        // 减量: 拿「扫描集里的全部名字」当存活名单喵
        let alive: std::collections::HashSet<&str> =
            scanned.iter().map(|a| a.name.as_str()).collect();
        let gone: Vec<String> = self
            .apps
            .iter()
            .filter(|a| a.source == AppSource::Scanned && !alive.contains(a.name.as_str()))
            .map(|a| a.name.clone())
            .collect();
        for name in &gone {
            self.remove(name);
        }

        // 增量 + 更新喵
        let mut delta = ScanDelta { removed: gone.len(), removed_names: gone, ..Default::default() };
        for app in scanned {
            let existing = self.find(&app.name);
            if let Some(existing) = existing {
                // 只补来源为扫描的,不覆盖手动注册喵
                if existing.source == AppSource::Manual {
                    continue;
                }
                // 路径变了就更新,保留用户附加信息喵
                if existing.path != app.path
                    && let Some(slot) = self.apps.iter_mut().find(|a| a.name == app.name) {
                        slot.path = app.path;
                    }
            } else {
                self.apps.push(app);
                delta.added += 1;
            }
        }
        log::debug!("扫描合并完成: 新增 {} 个,移除 {} 个应用喵", delta.added, delta.removed);
        delta
    }
}

/// 清理文件名里的非法字符(用于图标快照文件名)喵
pub(crate) fn sanitize_file_name(name: &str) -> String {
    let invalid = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut out: String = name
        .chars()
        .map(|c| if invalid.contains(&c) { '_' } else { c })
        .collect();
    // 太长的名字截断,避免路径过深喵
    if out.chars().count() > 60 {
        out = out.chars().take(60).collect();
    }
    out
}
