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
use crate::app::config::{AppConfig, SearchMode};
use crate::platform::Platform;
use crate::search::{to_pinyin_initials, SearchEngine};

/// 空查询时每类推荐最多条数喵
const QUICK_LIMIT: usize = 8;

/// 结果列表条目喵(分组头不可启动)喵
#[derive(Debug, Clone)]
pub enum ListItem {
    /// 分组标题喵
    Section(String),
    /// 应用喵
    App(AppInfo),
}
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
    /// 热键配置变更,重新注册全局热键喵
    ReapplyHotkey,
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
    /// 当前列表(搜索结果或推荐区,含分组头)喵
    pub results: Vec<ListItem>,
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
    pub fn persist(&mut self) {
        self.config.sync_island_size();
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
        self.apply_default_prefix();
        self.refresh_results();
    }

    /// 呼出时按默认搜索模式垫上前缀喵
    fn apply_default_prefix(&mut self) {
        self.query = match self.config.search.default_mode {
            SearchMode::Tag => "t: ".into(),
            SearchMode::Initial => "i: ".into(),
            SearchMode::Name => String::new(),
        };
    }

    /// 根据当前查询重新计算列表喵,返回条目数喵
    pub fn refresh_results(&mut self) -> usize {
        let q = self.query.trim();
        self.results = if q.is_empty() {
            browse_list(&self.registry, &self.config)
        } else {
            self.search
                .search(&self.registry, q)
                .into_iter()
                .cloned()
                .map(ListItem::App)
                .collect()
        };
        self.results_visible = !self.results.is_empty();
        self.selected = first_app_index(&self.results).unwrap_or(0);
        self.results.len()
    }

    /// 移动选中,跳过分组头喵
    pub fn move_selection(&mut self, delta: isize) {
        let len = self.results.len() as isize;
        if len == 0 {
            return;
        }
        let mut idx = self.selected as isize;
        for _ in 0..len {
            idx = (idx + delta).rem_euclid(len);
            if matches!(self.results.get(idx as usize), Some(ListItem::App(_))) {
                self.selected = idx as usize;
                return;
            }
        }
    }

    /// 当前选中的应用喵
    pub fn selected_app(&self) -> Option<&AppInfo> {
        match self.results.get(self.selected) {
            Some(ListItem::App(app)) => Some(app),
            _ => None,
        }
    }

    /// 启动当前选中的应用喵,返回成功启动的应用名喵~
    pub fn launch_selected(&mut self) -> Option<String> {
        let app = self.selected_app()?.clone();
        let launched = self.platform.launch(&app.path);
        if launched {
            log::info!("启动应用: {} ({}) 喵", app.name, app.path);
            self.record_launch(&app.name);
            Some(app.name)
        } else {
            None
        }
    }

    /// 手动注册应用喵
    pub fn register_app(&mut self, name: &str, path: &str, icon: Option<String>) -> bool {
        let mut app = AppInfo::manual(name, path);
        app.icon_path = icon;
        let added = self.registry.upsert(app);
        self.search.sync(&self.registry.apps);
        self.persist();
        log::info!("手动注册应用: {name} ({path}) 喵");
        added
    }

    /// 切换收藏喵
    pub fn toggle_favorite(&mut self, name: &str) {
        if let Some(app) = self.registry.apps.iter_mut().find(|a| a.name == name) {
            app.favorite = !app.favorite;
            log::info!("收藏 {} → {} 喵", name, app.favorite);
        }
        self.persist();
    }

    /// 给应用加 tag喵
    pub fn add_tag(&mut self, name: &str, tag: &str) {
        let tag = tag.trim();
        if tag.is_empty() {
            return;
        }
        if let Some(app) = self.registry.apps.iter_mut().find(|a| a.name == name)
            && !app.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
        {
            app.tags.push(tag.to_string());
        }
        self.persist();
    }

    /// 去掉应用的 tag喵
    pub fn remove_tag(&mut self, name: &str, tag: &str) {
        if let Some(app) = self.registry.apps.iter_mut().find(|a| a.name == name) {
            app.tags.retain(|t| t != tag);
        }
        self.persist();
    }

    /// 移除应用喵
    pub fn remove_app(&mut self, name: &str) {
        self.registry.remove(name);
        self.search.sync(&self.registry.apps);
        self.persist();
    }
}

/// 空查询推荐列表喵
pub fn browse_list(registry: &AppRegistry, config: &AppConfig) -> Vec<ListItem> {
    let mut items = Vec::new();
    let win = &config.window;
    if win.show_recent {
        push_section(&mut items, "最近打开", registry.recent(QUICK_LIMIT));
    }
    if win.show_favorites {
        push_section(&mut items, "收藏", registry.favorites());
    }
    if win.show_frequent {
        push_section(&mut items, "最常用", registry.frequent(QUICK_LIMIT));
    }
    if win.show_all {
        push_initial_groups(&mut items, &registry.apps);
    }
    items
}

fn push_section(items: &mut Vec<ListItem>, title: &str, apps: Vec<&AppInfo>) {
    if apps.is_empty() {
        return;
    }
    items.push(ListItem::Section(title.into()));
    items.extend(apps.into_iter().cloned().map(ListItem::App));
}

fn push_initial_groups(items: &mut Vec<ListItem>, apps: &[AppInfo]) {
    if apps.is_empty() {
        return;
    }
    let mut sorted = apps.to_vec();
    sorted.sort_by_key(|a| to_pinyin_initials(&a.name).to_uppercase());
    let mut current = '\0';
    for app in sorted {
        let ch = to_pinyin_initials(&app.name)
            .chars()
            .next()
            .unwrap_or('#')
            .to_ascii_uppercase();
        let letter = if ch.is_ascii_alphanumeric() { ch } else { '#' };
        if letter != current {
            current = letter;
            items.push(ListItem::Section(letter.to_string()));
        }
        items.push(ListItem::App(app));
    }
}

fn first_app_index(items: &[ListItem]) -> Option<usize> {
    items.iter().position(|i| matches!(i, ListItem::App(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::{AppInfo, AppRegistry, AppSource};
    use crate::app::config::WindowConfig;

    fn app(name: &str, fav: bool, count: u32, last: u64) -> AppInfo {
        AppInfo {
            name: name.into(),
            path: format!("{name}.exe"),
            icon_path: None,
            tags: Vec::new(),
            favorite: fav,
            launch_count: count,
            last_used: last,
            source: AppSource::Manual,
        }
    }

    #[test]
    fn browse_list_respects_toggles() {
        let mut registry = AppRegistry::default();
        registry.apps = vec![
            app("Alpha", true, 5, 100),
            app("Beta", false, 1, 50),
        ];
        let mut config = AppConfig::default();
        config.window = WindowConfig {
            show_recent: true,
            show_favorites: true,
            show_frequent: false,
            show_all: false,
            ..config.window
        };
        let items = browse_list(&registry, &config);
        let titles: Vec<_> = items
            .iter()
            .filter_map(|i| match i {
                ListItem::Section(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(titles, ["最近打开", "收藏"]);
    }
}
