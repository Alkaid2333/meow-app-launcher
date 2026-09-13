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
use crate::platform::Win32Platform;
use crate::search::{
    Action, ItemIcon, MAX_RESULTS, ParsedQuery, Scored, SearchEngine, SearchItem,
    extra_results, to_pinyin_initials, ProviderContext,
};

/// 空查询时每类推荐最多条数喵
const QUICK_LIMIT: usize = 8;

/// 结果列表条目喵(分组头不可激活)喵
#[derive(Debug, Clone)]
pub enum ListItem {
    /// 分组标题喵
    Section(String),
    /// 统一搜索条目(应用/计算器/命令/Web 均走这里)喵
    Item(SearchItem),
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
    /// 打开应用管理中心窗口喵
    OpenAppManager,
    /// 重新扫描系统应用喵
    Rescan,
    /// 重启应用喵
    Restart,
    /// 热键配置变更,重新注册全局热键喵
    ReapplyHotkey,
    /// 渲染后端变更,重建渲染器喵
    RecreateRenderer,
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
    pub platform: Arc<Win32Platform>,

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
    /// 应用管理中心是否应显示喵(供管理窗口心跳检测)喵
    pub manager_visible: bool,
}

impl AppState {
    /// 初始化应用状态喵,并确保数据目录存在喵~
    pub fn new(platform: Arc<Win32Platform>, data_dir: PathBuf) -> Self {
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
            manager_visible: false,
        }
    }

    /// 保存全部持久化数据喵(配置 + 注册表)喵
    pub fn persist(&mut self) {
        self.config.save(&self.data_dir);
        self.registry.save(&self.data_dir);
    }

    /// 合并后台扫描结果进注册表喵(由主线程在收到异步扫描结果后调用)喵
    ///
    /// 命中关键词过滤规则的启动项在入库前剔除,保持注册表干净喵。
    pub fn merge_scanned(&mut self, scanned: Vec<AppInfo>) -> usize {
        let total = scanned.len();
        let visible: Vec<AppInfo> = scanned
            .into_iter()
            .filter(|a| !self.config.is_app_filtered(&a.name))
            .collect();
        let skipped = total.saturating_sub(visible.len());
        let added = self.registry.merge_scanned(visible);
        self.search.sync(&self.registry.apps);
        self.persist();
        if skipped > 0 {
            log::info!("扫描完成,过滤排除 {skipped} 个应用喵");
        }
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
        let q = self.query.trim().to_string();
        self.results = if q.is_empty() {
            browse_list(&self.registry, &self.config)
        } else {
            self.aggregated_results(&q)
        };
        self.results_visible = !self.results.is_empty();
        self.selected = first_selectable_index(&self.results).unwrap_or(0);
        self.results.len()
    }

    /// 非空查询的多源聚合喵: 应用源 + 内置 Provider(计算器/命令/Web)混排喵
    ///
    /// 分数域约定: 计算器 3.0 置顶、命令 = 模糊分 + 2.0、应用为原始模糊分、
    /// Web 固定 0.0 垫底;t:/i: 定向模式不注入多源结果喵。
    /// **系统指令单独成组置底**——与应用彻底隔离,避免方向键/点击误触关机类指令喵。
    /// 排序后统一截断到 [`MAX_RESULTS`],保证交互预算喵。
    fn aggregated_results(&self, query: &str) -> Vec<ListItem> {
        let mut scored: Vec<Scored> = Vec::new();

        if ParsedQuery::parse(query).mode == crate::search::SearchMode::Name {
            scored.extend(extra_results(&ProviderContext {
                query,
                config: &self.config,
            }));
        }

        for (app, score) in self.search.search_scored(&self.registry, &self.config, query) {
            scored.push(Scored::new(score, SearchItem::from_app(app)));
        }

        aggregate_items(scored)
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
            if matches!(self.results.get(idx as usize), Some(ListItem::Item(_))) {
                self.selected = idx as usize;
                return;
            }
        }
    }

    /// 当前选中的条目喵
    pub fn selected_item(&self) -> Option<&SearchItem> {
        match self.results.get(self.selected) {
            Some(ListItem::Item(item)) => Some(item),
            _ => None,
        }
    }

    /// 执行当前选中条目的动作喵,返回人类可读的结果描述(失败为 None)喵
    ///
    /// 应用条目顺带记录使用统计(次数 + 最近时间)喵。
    pub fn execute_selected(&mut self) -> Option<String> {
        let item = self.selected_item()?.clone();
        self.run_item(&item)
    }

    /// 执行任意条目的动作喵(搜索结果与右键菜单共用)喵
    ///
    /// 应用条目顺带记录使用统计(次数 + 最近时间)喵。
    pub fn run_item(&mut self, item: &SearchItem) -> Option<String> {
        match &item.action {
            Action::Launch(path) => {
                if !self.platform.launch(path) {
                    log::warn!("启动失败: {} ({}) 喵", item.title, path);
                    return None;
                }
                if let ItemIcon::App(app) = &item.icon {
                    log::info!("启动应用: {} ({}) 喵", app.name, path);
                    self.record_launch(&app.name);
                } else {
                    log::info!("启动: {path} 喵");
                }
                Some(item.title.clone())
            }
            Action::OpenUrl(url) => {
                // 空浏览器 = 系统默认;配置了就走自定义浏览器喵
                let browser = self.config.search.web_browser.clone();
                if !self.platform.open_url(url, &browser) {
                    log::warn!("打开链接失败: {url} 喵");
                    return None;
                }
                log::info!("打开链接: {url} 喵");
                Some(item.title.clone())
            }
            Action::CopyText(text) => {
                if !self.platform.copy_to_clipboard(text) {
                    return None;
                }
                Some(format!("已复制 {text}"))
            }
            Action::SystemCommand(kind) => {
                if !self.platform.execute_system_command(*kind) {
                    return None;
                }
                Some(item.title.clone())
            }
        }
    }

    /// 原地修改一个注册应用喵(改名/改路径/改图标等统一入口)
    ///
    /// 修改后重同步搜索索引并持久化,返回是否修改成功喵。
    /// 改名时若新名与现有应用冲突则拒绝喵。
    pub fn mutate_app(&mut self, name: &str, f: impl FnOnce(&mut AppInfo) -> bool) -> bool {
        let old_name = name.to_string();
        // 先在克隆体上试改: 改名校验不过就不落地,避免半途污染索引喵
        let Some(mut draft) = self.registry.apps.iter().find(|a| a.name == old_name).cloned() else {
            return false;
        };
        if !f(&mut draft) {
            return false;
        }
        if draft.name != old_name && self.registry.find(&draft.name).is_some() {
            log::warn!("应用改名冲突: {} 已存在,已拒绝喵", draft.name);
            return false;
        }
        if let Some(app) = self.registry.apps.iter_mut().find(|a| a.name == old_name) {
            *app = draft;
        } else {
            return false;
        }
        self.search.sync(&self.registry.apps);
        self.persist();
        true
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

/// 把多源聚合的带分条目整理成最终列表喵(纯函数,便于单测)喵
///
/// 规则: 应用等条目按分数降序(同分按标题)截断到 [`MAX_RESULTS`];
/// **系统指令单独成组置底**——与应用彻底隔离,避免误触关机类指令喵。
pub fn aggregate_items(scored: Vec<Scored>) -> Vec<ListItem> {
    let (mut commands, mut rest): (Vec<Scored>, Vec<Scored>) = scored
        .into_iter()
        .partition(|s| matches!(s.item.action, Action::SystemCommand(_)));
    let by_rank = |a: &Scored, b: &Scored| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.item.title.cmp(&b.item.title))
    };
    rest.sort_by(by_rank);
    commands.sort_by(by_rank);
    rest.truncate(MAX_RESULTS);

    let mut items: Vec<ListItem> = rest.into_iter().map(|s| ListItem::Item(s.item)).collect();
    if !commands.is_empty() {
        items.push(ListItem::Section("指令".into()));
        items.extend(commands.into_iter().map(|s| ListItem::Item(s.item)));
    }
    items
}

/// 空查询推荐列表喵(过滤规则命中的应用先行剔除)喵
pub fn browse_list(registry: &AppRegistry, config: &AppConfig) -> Vec<ListItem> {
    // 先过滤出可见应用,所有分组都基于这份可见集喵
    let visible: Vec<&AppInfo> = registry
        .apps
        .iter()
        .filter(|a| !config.is_app_filtered(&a.name))
        .collect();
    let mut items = Vec::new();
    let win = &config.window;
    if win.show_recent {
        let mut v: Vec<&AppInfo> = visible.iter().copied().filter(|a| a.last_used > 0).collect();
        v.sort_by_key(|a| std::cmp::Reverse(a.last_used));
        v.truncate(QUICK_LIMIT);
        push_section(&mut items, "最近打开", v);
    }
    if win.show_favorites {
        push_section(&mut items, "收藏", visible.iter().copied().filter(|a| a.favorite).collect());
    }
    if win.show_frequent {
        let mut v: Vec<&AppInfo> = visible.iter().copied().filter(|a| a.launch_count > 0).collect();
        v.sort_by_key(|a| std::cmp::Reverse(a.launch_count));
        v.truncate(QUICK_LIMIT);
        push_section(&mut items, "最常用", v);
    }
    if win.show_all {
        push_initial_groups(&mut items, &visible);
    }
    items
}

fn push_section(items: &mut Vec<ListItem>, title: &str, apps: Vec<&AppInfo>) {
    if apps.is_empty() {
        return;
    }
    items.push(ListItem::Section(title.into()));
    items.extend(apps.into_iter().map(|a| ListItem::Item(SearchItem::from_app(a))));
}

fn push_initial_groups(items: &mut Vec<ListItem>, apps: &[&AppInfo]) {
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
        items.push(ListItem::Item(SearchItem::from_app(app)));
    }
}

/// 第一个可激活条目的索引喵(跳过分组头)喵
fn first_selectable_index(items: &[ListItem]) -> Option<usize> {
    items.iter().position(|i| matches!(i, ListItem::Item(_)))
}
