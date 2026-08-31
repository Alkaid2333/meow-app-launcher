//! 应用级状态与生命周期喵~
//!
//! `AppState` 是应用唯一的状态中枢,由窗口层(单线程消息循环)持有喵。
//! 包含: 配置、应用注册表、搜索引擎、图标管理器、平台句柄,
//! 以及搜索交互的瞬时状态(查询/结果/选中)喵。
//!
//! 与上一版(GPUI)相比: 不再挂载为 `Global`,而是普通结构;
//! 状态更新全部发生在单线程内,天然避免跨线程同步问题喵。

pub mod config;

use crate::apps::{AppInfo, AppRegistry, icon::IconManager};
use crate::app::config::AppConfig;
use crate::platform::Platform;
use crate::search::SearchEngine;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// 共享应用状态喵(单线程消息循环,用 Rc<RefCell> 跨窗口共享)喵
pub type SharedState = Rc<RefCell<AppState>>;

/// 应用级命令喵(托盘/配置窗口触发,主窗口消息循环里执行)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// 呼出/隐藏启动器喵
    ToggleLauncher,
    /// 打开配置窗口喵
    OpenSettings,
    /// 重新扫描系统应用喵
    Rescan,
    /// 重启应用喵
    Restart,
    /// 退出应用喵
    Quit,
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
    pub platform: Arc<dyn Platform>,

    // ---------- 搜索交互状态喵 ----------
    /// 当前查询喵
    pub query: String,
    /// 搜索结果(按分数降序)喵
    pub results: Vec<AppInfo>,
    /// 选中索引喵
    pub selected: usize,
    /// 是否有结果需要展开面板喵
    pub results_visible: bool,
    /// 启动器是否可见喵(供托盘菜单动态文本)喵
    pub launcher_visible: bool,
    /// 配置窗口是否应显示喵(供配置窗口心跳检测)喵
    pub settings_visible: bool,
}

impl AppState {
    /// 初始化应用状态喵,并确保数据目录存在喵~
    pub fn new(platform: Arc<dyn Platform>, data_dir: PathBuf) -> Self {
        // 确保数据目录存在喵
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            log::error!("创建数据目录失败: {e}");
        }

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
            launcher_visible: false,
            settings_visible: false,
        }
    }

    /// 保存全部持久化数据喵(配置 + 注册表)喵
    pub fn persist(&self) {
        self.config.save(&self.data_dir);
        self.registry.save(&self.data_dir);
    }

    /// 合并后台扫描结果进注册表喵(由主线程在收到异步扫描结果后调用)喵
    pub fn merge_scanned(&mut self, scanned: Vec<AppInfo>) -> usize {
        let added = self.registry.merge_scanned(scanned);
        self.search.sync(&self.registry.apps);
        self.persist();
        log::info!("应用扫描完成,新增 {added} 个喵");
        added
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

    /// 清空搜索状态喵(隐藏窗口 / 启动应用后调用,避免面板残留)喵
    pub fn reset_search(&mut self) {
        self.query.clear();
        self.results.clear();
        self.results_visible = false;
        self.selected = 0;
    }

    /// 根据当前查询重新计算搜索结果喵(更新 results/results_visible/selected)喵
    ///
    /// 返回结果数量,便于调用方判断是否需要展开面板喵。
    pub fn refresh_results(&mut self) -> usize {
        let q = self.query.trim();
        if q.is_empty() {
            self.results.clear();
            self.results_visible = false;
            self.selected = 0;
            return 0;
        }
        // 搜索结果按分数降序,克隆为自有数据喵
        self.results = self
            .search
            .search(&self.registry, q)
            .into_iter()
            .cloned()
            .collect();
        self.results_visible = !self.results.is_empty();
        self.selected = 0;
        self.results.len()
    }

    /// 启动当前选中的应用喵,返回成功启动的应用名喵~
    pub fn launch_selected(&mut self) -> Option<String> {
        let app = self.results.get(self.selected)?.clone();
        let launched = self.platform.launch(&app.path);
        if launched {
            log::info!("启动应用: {} ({}) 喵", app.name, app.path);
            self.record_launch(&app.name);
            Some(app.name)
        } else {
            None
        }
    }
}
