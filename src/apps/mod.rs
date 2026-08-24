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
    /// 新建一个手动注册的应用喵(供 CLI/配置 GUI 使用)喵
    #[allow(dead_code)]
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

#[allow(dead_code)] // 注册表操作 API 供 CLI/配置 GUI 使用(第二阶段)喵
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

    /// 手动注册一个应用喵(存在同名则更新)喵
    pub fn register_manual(&mut self, name: &str, path: &str) -> bool {
        if name.trim().is_empty() || path.trim().is_empty() {
            log::warn!("手动注册参数不完整: name={name:?} path={path:?}");
            return false;
        }
        self.upsert(AppInfo::manual(name, path))
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

    /// 收藏的应用喵
    pub fn favorites(&self) -> Vec<&AppInfo> {
        self.apps.iter().filter(|a| a.favorite).collect()
    }

    /// 最近使用的应用(按时间倒序,最多 n 个)喵
    pub fn recent(&self, n: usize) -> Vec<&AppInfo> {
        let mut apps: Vec<&AppInfo> = self
            .apps
            .iter()
            .filter(|a| a.last_used > 0)
            .collect();
        apps.sort_by_key(|a| std::cmp::Reverse(a.last_used));
        apps.truncate(n);
        apps
    }

    /// 最常用的应用(按次数倒序,最多 n 个)喵
    pub fn frequent(&self, n: usize) -> Vec<&AppInfo> {
        let mut apps: Vec<&AppInfo> = self.apps.iter().filter(|a| a.launch_count > 0).collect();
        apps.sort_by_key(|a| std::cmp::Reverse(a.launch_count));
        apps.truncate(n);
        apps
    }

    /// 合并扫描到的应用喵(保留已有的手动信息: tag/收藏/计数)喵
    pub fn merge_scanned(&mut self, scanned: Vec<AppInfo>) -> usize {
        let mut added = 0;
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
                added += 1;
            }
        }
        log::debug!("扫描合并完成: 新增 {added} 个应用喵");
        added
    }
}

/// 清理文件名里的非法字符(用于图标快照文件名)喵
fn sanitize_file_name(name: &str) -> String {
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
