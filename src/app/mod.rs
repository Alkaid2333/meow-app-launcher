//! 应用级状态与生命周期喵~
//!
//! AppState 是全局唯一状态,作为 GPUI Global 挂载喵。
//! 包含: 配置、应用注册表、搜索引擎、图标管理器、平台句柄喵。

pub mod actions;
pub mod config;

use crate::apps::{AppInfo, AppRegistry, icon::IconManager};
use crate::platform::PlatformCapabilities;
use crate::search::SearchEngine;
use crate::app::config::AppConfig;
use gpui::Global;
use std::path::PathBuf;
use std::sync::Arc;

/// 搜索结果条目喵(供选择窗口渲染)喵
#[derive(Clone)]
pub struct ResultEntry {
    pub app: AppInfo,
    pub icon: Option<PathBuf>,
}

/// 全局应用状态喵
pub struct AppState {
    /// 数据目录(./.datas)喵
    pub data_dir: PathBuf,
    /// 全局配置喵
    pub config: AppConfig,
    /// 应用注册表喵
    pub registry: AppRegistry,
    /// 搜索引擎喵
    pub search: SearchEngine,
    /// 图标管理器喵
    pub icons: IconManager,
    /// 平台能力句柄喵
    pub platform: Arc<dyn PlatformCapabilities>,
    // ---------- 搜索交互状态(双窗口共享)喵 ----------
    /// 当前查询喵
    pub query: String,
    /// 搜索结果喵
    pub results: Vec<ResultEntry>,
    /// 选中索引喵
    pub selected: usize,
    /// 选择窗口是否可见喵
    pub results_visible: bool,
    /// 搜索框窗口位置 (x, y, w, h),供选择窗口对齐喵
    pub searchbar_bounds: Option<(f32, f32, f32, f32)>,
}

impl Global for AppState {}

impl AppState {
    /// 初始化全局状态喵,并把数据落到磁盘喵
    pub fn new(data_dir: PathBuf) -> Self {
        // 确保数据目录存在喵
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            log::error!("创建数据目录失败: {e}");
        }

        let platform = crate::platform::platform();
        let config = AppConfig::load(&data_dir);
        let registry = AppRegistry::load(&data_dir);
        let mut search = SearchEngine::new();
        search.sync(&registry.apps);
        let icons = IconManager::new(data_dir.clone(), platform.clone());

        log::info!(
            "AppState 初始化完成喵 ~ 平台: {}, 注册应用: {} 个",
            platform.platform_name(),
            registry.apps.len()
        );

        Self {
            data_dir,
            config,
            registry,
            search,
            icons,
            platform,
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            results_visible: false,
            searchbar_bounds: None,
        }
    }

    /// 保存全部持久化数据喵(配置 + 注册表)喵
    pub fn persist(&self) {
        self.config.save(&self.data_dir);
        self.registry.save(&self.data_dir);
    }

    /// 重新扫描系统应用并合并进注册表喵
    pub fn rescan_apps(&mut self) {
        let scanned = crate::apps::scanner::scan_installed_apps();
        let added = self.registry.merge_scanned(scanned);
        self.search.sync(&self.registry.apps);
        self.persist();
        log::info!("应用扫描完成,新增 {added} 个喵");
    }

    /// 记录一次应用启动喵(次数 + 最近时间)喵
    pub fn record_launch(&mut self, name: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if let Some(app) = self.registry.apps.iter_mut().find(|a| a.name == name) {
            app.bump_launch(now);
        }
        self.persist();
    }

    /// 清空搜索状态喵(隐藏窗口 / 启动应用后调用,避免选择窗被重新拉出来)喵
    pub fn reset_search(&mut self) {
        self.query.clear();
        self.results.clear();
        self.results_visible = false;
        self.selected = 0;
    }

    /// 根据当前查询重新计算搜索结果喵(更新 results/results_visible/selected)喵
    pub fn refresh_results(&mut self) {
        let q = self.query.trim();
        if q.is_empty() {
            self.results.clear();
            self.results_visible = false;
            self.selected = 0;
            return;
        }
        let apps = self.search.search(&self.registry, q);
        let mut results = Vec::with_capacity(apps.len());
        for app in apps {
            let icon = self.icons.icon_path(app);
            results.push(ResultEntry {
                app: app.clone(),
                icon,
            });
        }
        self.results = results;
        self.results_visible = !self.results.is_empty();
        self.selected = 0;
    }

    /// 启动当前选中的应用喵,返回是否成功喵
    pub fn launch_selected(&self) -> Option<String> {
        let app = self.results.get(self.selected).map(|r| r.app.clone())?;
        let launched = self.platform.launch(&app.path);
        if launched {
            log::info!("启动应用: {} ({}) 喵", app.name, app.path);
            Some(app.name)
        } else {
            None
        }
    }
}
