//! 应用管理中心窗口喵~
//!
//! 独立窗口陈列**全部**注册应用: 左列表(网格/列表双排版 + 过滤 + 滚动条),
//! 右详情卡可编辑名称/路径/图标/标签/收藏,并提供「打开所在位置/移除」动作喵。
//! 与配置窗口同款心跳模式: 平时隐藏,`manager_visible` 置位时自show喵。
//!
//! 入口: 配置 GUI「应用」页按钮 + 托盘「应用管理」菜单项喵。

use crate::app::SharedState;
use crate::apps::AppInfo;
use crate::apps::icon::IconExtractor;
use crate::platform::{Key, PlatformWindow, Win32Platform, WindowEvent, WindowHandler};
use crate::render::app_manager::{
    paint_app_manager, DetailField, MANAGER_HEIGHT, MANAGER_WIDTH, ManagerHit,
};
use crate::render::edit::{caret_from_x, TextEdit};
use crate::render::font::FontCache;
use crate::render::theme::SettingsTheme;
use crate::render::Renderer;
use skia_safe::{Image, Rect};
use std::collections::{HashMap, HashSet};
use std::sync::{mpsc, Arc};

/// 心跳间隔(ms): 隐藏时低频检查显示请求喵
const HEARTBEAT_MS: u32 = 100;
/// 动效帧间隔(ms)喵
const ANIM_FRAME_MS: u32 = 16;
/// 滚动平滑的趋近系数喵
const SCROLL_EASE: f32 = 0.28;
/// 滚轮一格滚动量喵
const WHEEL_STEP: f32 = 48.0;

/// 后台任务结果喵(完成后回传主线程)喵
enum BgEvent {
    /// 图标提取完成喵
    Icon { name: String, image: Option<Image> },
    /// 应用扫描完成喵
    Scanned(Vec<AppInfo>),
}

/// 应用管理中心窗口喵
pub struct AppManagerWindow {
    /// 共享应用状态喵
    state: SharedState,
    /// 平台句柄喵
    platform: Arc<Win32Platform>,
    /// 窗口句柄喵
    window: PlatformWindow,
    /// Skia 渲染器喵
    renderer: Renderer,
    /// 上次见到的渲染后端配置(变化时重建渲染器)喵
    last_backend: crate::app::config::RenderBackend,
    /// 字体缓存喵
    fonts: FontCache,
    /// 是否可见喵
    visible: bool,
    /// 列表滚动偏移(逻辑像素)喵
    scroll: f32,
    /// 滚动目标喵
    scroll_target: f32,
    /// 最近一次布局结果(供命中测试)喵
    layout: Option<crate::render::app_manager::ManagerLayout>,
    /// DPI 缩放系数喵
    scale: f32,
    /// BGRA 像素缓冲喵
    pixels: Vec<u8>,
    /// 当前选中的应用名喵
    selected: Option<String>,
    /// 过滤词喵(独立持久,点击别处不会丢;编辑态草稿另存 filter_edit)喵
    filter_text: String,
    /// 过滤词草稿喵
    filter_edit: Option<TextEdit>,
    /// 名称编辑草稿喵
    name_edit: Option<TextEdit>,
    /// 路径编辑草稿喵
    path_edit: Option<TextEdit>,
    /// 图标路径编辑草稿喵
    icon_edit: Option<TextEdit>,
    /// 新标签编辑草稿喵
    tag_edit: Option<TextEdit>,
    /// 屏蔽词草稿喵
    block_edit: Option<TextEdit>,
    /// 正在拖选文本的输入框喵
    text_drag: Option<TextDrag>,
    /// 滚动条拖动快照喵
    scroll_drag: Option<ScrollDrag>,
    /// 窗口屏幕位置(物理像素)喵
    win_pos: Option<(i32, i32)>,
    /// 顶栏拖拽状态喵
    win_drag: Option<WinDrag>,
    /// 当前生效的定时器间隔(ms)喵
    timer_interval_ms: u32,
    /// 后台任务结果发送端喵
    bg_tx: mpsc::Sender<BgEvent>,
    /// 后台任务结果接收端喵
    bg_rx: mpsc::Receiver<BgEvent>,
    /// 图标提取器喵
    icon_extractor: IconExtractor,
    /// 正在提取图标的应用名喵
    pending_icons: HashSet<String>,
    /// 是否正在后台扫描(防重复触发)喵
    scanning: bool,
}

/// 正在被鼠标拖选文本的输入框喵
#[derive(Debug, Clone, Copy)]
struct TextDrag {
    rect: Rect,
}

/// 滚动条拖动快照喵
struct ScrollDrag {
    anchor_y: f32,
    anchor_prog: f32,
    travel: f32,
    max_logical: f32,
}

/// 顶栏拖拽快照喵
struct WinDrag {
    anchor_screen: (i32, i32),
    anchor_pos: (i32, i32),
}

impl AppManagerWindow {
    /// 装配应用管理中心窗口喵: 创建窗口(初始隐藏)、渲染器、启动心跳喵
    pub fn spawn(platform: Arc<Win32Platform>, state: SharedState) -> PlatformWindow {
        let scale = platform.scale_factor();
        let win_w = (MANAGER_WIDTH * scale).ceil() as i32;
        let win_h = (MANAGER_HEIGHT * scale).ceil() as i32;
        let (screen_w, screen_h) = platform.screen_size();
        let x = (screen_w - win_w) / 2;
        let y = (screen_h - win_h) / 2;

        let spec = crate::platform::WindowSpec {
            x,
            y,
            width: win_w,
            height: win_h,
        };
        let window = platform.create_window(&spec).unwrap_or_else(|| {
            log::error!("应用管理中心窗口创建失败,即将退出喵~");
            std::process::exit(1);
        });

        let backend = state.borrow().config.render_backend;
        let renderer = Renderer::new(win_w, win_h, backend, &platform).unwrap_or_else(|| {
            log::error!("应用管理中心渲染器初始化失败,即将退出喵~");
            std::process::exit(1);
        });

        let (bg_tx, bg_rx) = mpsc::channel();
        let icon_extractor = state.borrow().icons.extractor();

        let manager = AppManagerWindow {
            state,
            platform: platform.clone(),
            window,
            renderer,
            last_backend: backend,
            fonts: FontCache::new(),
            visible: false,
            scroll: 0.0,
            scroll_target: 0.0,
            layout: None,
            scale,
            pixels: Vec::new(),
            selected: None,
            filter_text: String::new(),
            filter_edit: None,
            name_edit: None,
            path_edit: None,
            icon_edit: None,
            tag_edit: None,
            text_drag: None,
            scroll_drag: None,
            win_pos: Some((x, y)),
            win_drag: None,
            timer_interval_ms: HEARTBEAT_MS,
            bg_tx,
            bg_rx,
            icon_extractor,
            pending_icons: HashSet::new(),
            block_edit: None,
            scanning: false,
        };

        platform.set_window_handler(&window, Box::new(manager));
        platform.enable_file_drop(&window);
        platform.set_timer(&window, HEARTBEAT_MS);

        window
    }

    // ---------------------------------------------------------------------
    // 显示 / 隐藏
    // ---------------------------------------------------------------------

    /// 显示管理窗口喵
    fn show(&mut self) {
        log::info!("显示应用管理中心喵~");
        self.visible = true;
        self.scroll = 0.0;
        self.scroll_target = 0.0;
        self.platform.show_window(&self.window, true);
        self.platform.focus_window(&self.window);
        self.render();
    }

    /// 隐藏管理窗口喵(先提交挂起的编辑,避免丢改动)喵
    fn hide(&mut self) {
        log::info!("隐藏应用管理中心喵~");
        self.commit_name_edit();
        self.commit_path_edit();
        self.commit_icon_edit();
        self.commit_tag_edit();
        self.commit_filter_edit();
        self.visible = false;
        self.state.borrow_mut().manager_visible = false;
        self.platform.show_window(&self.window, false);
    }

    // ---------------------------------------------------------------------
    // 数据
    // ---------------------------------------------------------------------

    /// 过滤后的应用列表喵(按拼音首字母排序,便于翻找)喵
    fn filtered_apps(&self) -> Vec<AppInfo> {
        // 编辑中优先取草稿,否则用已提交的过滤词喵
        let kw = self
            .filter_edit
            .as_ref()
            .map(|t| t.text.trim())
            .unwrap_or(self.filter_text.trim());
        let kw_lower = kw.to_lowercase();
        let state = self.state.borrow();
        let mut apps: Vec<AppInfo> = state
            .registry
            .apps
            .iter()
            .filter(|a| {
                kw_lower.is_empty()
                    || a.name.to_lowercase().contains(&kw_lower)
                    || a.tags.iter().any(|t| t.to_lowercase().contains(&kw_lower))
            })
            .cloned()
            .collect();
        apps.sort_by_key(|a| crate::search::to_pinyin_initials(&a.name).to_uppercase());
        apps
    }

    /// 提交过滤词草稿喵(写回持久字段,文本不因点击别处而丢)喵
    ///
    /// 过滤词变化时把列表滚回顶部——结果集变了,旧视口位置多半已悬空喵。
    /// 返回过滤词是否发生变化喵。
    fn commit_filter_edit(&mut self) -> bool {
        let Some(te) = self.filter_edit.take() else {
            return false;
        };
        let text = te.text.trim().to_string();
        let changed = text != self.filter_text;
        if changed {
            log::debug!("过滤词更新: {:?} → {:?} 喵", self.filter_text, text);
            self.filter_text = text;
            // 视口回到顶部: 搜索结果的排列高度未必够到旧滚动位置喵
            self.scroll = 0.0;
            self.scroll_target = 0.0;
        }
        changed
    }

    /// 取已缓存的图标喵(缺失时后台提取,不阻塞渲染)喵
    fn cached_icon(&mut self, app: &AppInfo) -> Option<Image> {
        if let Some(img) = self.state.borrow_mut().icons.cached_image(app) {
            return Some(img);
        }
        if self.pending_icons.insert(app.name.clone()) {
            let tx = self.bg_tx.clone();
            let extractor = self.icon_extractor.clone();
            let app = app.clone();
            std::thread::spawn(move || {
                let (name, image) = extractor.extract(app);
                let _ = tx.send(BgEvent::Icon { name, image });
            });
        }
        None
    }

    // ---------------------------------------------------------------------
    // 渲染
    // ---------------------------------------------------------------------

    /// 渲染一帧喵
    fn render(&mut self) {
        let apps = self.filtered_apps();
        let total = self.state.borrow().registry.apps.len();
        let theme = {
            let state = self.state.borrow();
            SettingsTheme::for_preset(state.config.theme.preset)
        };
        let grid = self.state.borrow().config.window.layout == crate::app::config::AppLayout::Grid;
        let scale = self.scale;
        let selected = self.selected.clone();

        // 渲染前批量预取图标喵(缺失的顺手排进后台提取,不阻塞)喵
        let mut icon_table: HashMap<String, Option<Image>> = HashMap::new();
        for app in &apps {
            if !icon_table.contains_key(&app.name) {
                let img = self.cached_icon(app);
                icon_table.insert(app.name.clone(), img);
            }
        }

        let edits = crate::render::app_manager::ManagerEdits {
            filter: self.filter_edit.as_ref(),
            name: self.name_edit.as_ref(),
            path: self.path_edit.as_ref(),
            icon: self.icon_edit.as_ref(),
            tag: self.tag_edit.as_ref(),
            block: self.block_edit.as_ref(),
        };
        // 过滤框静态展示文本喵(非编辑态显示已提交的过滤词)喵
        let filter_shown = self.filter_text.clone();
        // 屏蔽词展示串喵(与配置 GUI 同一份规则,这里只读关键词)喵
        let block_words: Vec<String> = self
            .state
            .borrow()
            .config
            .search
            .filters
            .iter()
            .map(|f| f.keyword.clone())
            .collect();

        // 图标回调只查预取表,不借用整个 self 喵
        let mut icon_fn = |app: &AppInfo| -> Option<Image> {
            icon_table.get(&app.name).cloned().flatten()
        };

        let canvas = self.renderer.canvas();
        canvas.save();
        canvas.scale((scale, scale));
        let layout = paint_app_manager(
            canvas,
            &theme,
            &self.fonts,
            &apps,
            total,
            selected.as_deref(),
            grid,
            self.scroll,
            edits,
            &filter_shown,
            &block_words,
            &mut icon_fn,
            MANAGER_WIDTH,
            MANAGER_HEIGHT,
        );
        canvas.restore();
        self.layout = Some(layout);

        self.renderer.read_bgra(&mut self.pixels);
        self.platform.present(
            &self.window,
            self.renderer.width(),
            self.renderer.height(),
            &self.pixels,
        );
    }

    /// 按配置重建渲染器喵(切换 CPU/GPU 时调用)喵
    fn recreate_renderer(&mut self) {
        let backend = self.state.borrow().config.render_backend;
        let (w, h) = (self.renderer.width(), self.renderer.height());
        if let Some(renderer) = Renderer::new(w, h, backend, &self.platform) {
            log::info!("应用管理中心渲染器已重建 → {} 喵", renderer.mode().label());
            self.renderer = renderer;
        }
    }

    // ---------------------------------------------------------------------
    // 编辑
    // ---------------------------------------------------------------------

    /// 选中应用切换喵(先提交旧选中挂起的编辑,避免丢改动)喵
    fn select_app(&mut self, name: Option<String>) {
        self.commit_name_edit();
        self.commit_path_edit();
        self.commit_icon_edit();
        self.selected = name;
        self.tag_edit = None;
        self.text_drag = None;
        self.render();
    }

    /// 点击详情输入框喵: 进入编辑态(全选当前值)或定位光标喵
    fn click_detail_input(&mut self, field: DetailField, rect: Rect, lx: f32) {
        // 输入槽互斥: 点谁聚焦谁,其余草稿先提交喵
        self.commit_name_edit();
        self.commit_path_edit();
        self.commit_icon_edit();
        let value = match field {
            DetailField::Name => self.selected.clone().unwrap_or_default(),
            DetailField::Path => {
                let path = self.selected.as_ref().and_then(|n| {
                    let state = self.state.borrow();
                    state.registry.find(n).map(|a| a.path.clone())
                });
                path.unwrap_or_default()
            }
            DetailField::Icon => {
                let icon = self.selected.as_ref().and_then(|n| {
                    let state = self.state.borrow();
                    state.registry.find(n).and_then(|a| a.icon_path.clone())
                });
                icon.unwrap_or_default()
            }
            DetailField::TagAdd => String::new(),
        };
        let is_new = field == DetailField::TagAdd;
        // 画笔与字体先取好,再拿编辑槽的可变借用喵
        let paint = self.box_paint();
        let font = self.fonts.font(12.0);

        let slot = match field {
            DetailField::Name => &mut self.name_edit,
            DetailField::Path => &mut self.path_edit,
            DetailField::Icon => &mut self.icon_edit,
            DetailField::TagAdd => &mut self.tag_edit,
        };
        if slot.is_some() {
            if let Some(te) = slot.as_mut() {
                let caret = caret_from_x(&font, &paint, &te.text, rect.left + 8.0, lx);
                te.begin_select(caret);
            }
        } else {
            let mut te = TextEdit::new(value);
            if !is_new {
                te.select_all();
            }
            *slot = Some(te);
        }
        self.filter_edit = None;
        self.text_drag = Some(TextDrag { rect });
        self.render();
    }

    /// 输入框通用取色画笔喵
    fn box_paint(&self) -> skia_safe::Paint {
        let mut p = skia_safe::Paint::default();
        p.set_color(skia_safe::Color::BLACK);
        p
    }

    /// 提交名称编辑喵(改名,冲突时拒绝)喵
    fn commit_name_edit(&mut self) {
        let Some(te) = self.name_edit.take() else {
            return;
        };
        let Some(old) = self.selected.clone() else {
            return;
        };
        let new = te.text.trim().to_string();
        if new.is_empty() || new == old {
            return;
        }
        let mut state = self.state.borrow_mut();
        if state.mutate_app(&old, |a| {
            a.name = new.clone();
            true
        }) {
            log::info!("应用改名: {old} → {new} 喵");
            self.selected = Some(new);
        }
    }

    /// 提交路径编辑喵
    fn commit_path_edit(&mut self) {
        let Some(te) = self.path_edit.take() else {
            return;
        };
        let Some(name) = self.selected.clone() else {
            return;
        };
        let path = te.text.trim().to_string();
        if path.is_empty() {
            return;
        }
        let mut state = self.state.borrow_mut();
        if state.mutate_app(&name, |a| {
            if a.path == path {
                return false;
            }
            a.path = path.clone();
            true
        }) {
            log::info!("应用路径已更新: {name} → {path} 喵");
        }
    }

    /// 提交图标路径编辑喵(空 = 清除自定义图标)喵
    fn commit_icon_edit(&mut self) {
        let Some(te) = self.icon_edit.take() else {
            return;
        };
        let Some(name) = self.selected.clone() else {
            return;
        };
        let path = te.text.trim().to_string();
        let mut state = self.state.borrow_mut();
        if state.mutate_app(&name, |a| {
            let next = if path.is_empty() { None } else { Some(path.clone()) };
            if a.icon_path == next {
                return false;
            }
            a.icon_path = next;
            true
        }) {
            log::info!("应用自定义图标已更新: {name} 喵");
        }
    }

    /// 提交新标签喵
    fn commit_tag_edit(&mut self) {
        let Some(te) = self.tag_edit.take() else {
            return;
        };
        let draft = te.text.trim().to_string();
        if draft.is_empty() {
            return;
        }
        if let Some(name) = self.selected.clone() {
            self.state.borrow_mut().add_tag(&name, &draft);
            log::info!("已为 {name} 添加标签: {draft} 喵");
        }
    }

    /// 当前是否有输入框处于编辑态喵
    fn editing(&self) -> bool {
        self.filter_edit.is_some()
            || self.name_edit.is_some()
            || self.path_edit.is_some()
            || self.icon_edit.is_some()
            || self.tag_edit.is_some()
            || self.block_edit.is_some()
    }

    /// 编辑按键喵(Enter 提交 / Esc 取消)喵
    fn edit_key(&mut self, key: Key) {
        match key {
            Key::Backspace => {
                if let Some(te) = &mut self.filter_edit {
                    te.backspace();
                } else if let Some(te) = &mut self.name_edit {
                    te.backspace();
                } else if let Some(te) = &mut self.path_edit {
                    te.backspace();
                } else if let Some(te) = &mut self.icon_edit {
                    te.backspace();
                } else if let Some(te) = &mut self.tag_edit {
                    te.backspace();
                } else if let Some(te) = &mut self.block_edit {
                    te.backspace();
                }
                self.render();
            }
            Key::Delete => {
                if let Some(te) = &mut self.filter_edit {
                    te.delete();
                } else if let Some(te) = &mut self.name_edit {
                    te.delete();
                } else if let Some(te) = &mut self.path_edit {
                    te.delete();
                } else if let Some(te) = &mut self.icon_edit {
                    te.delete();
                } else if let Some(te) = &mut self.tag_edit {
                    te.delete();
                } else if let Some(te) = &mut self.block_edit {
                    te.delete();
                }
                self.render();
            }
            Key::Enter => {
                // 过滤词回车 = 提交并收起光标,其余提交喵
                self.commit_filter_edit();
                if self.tag_edit.is_some() {
                    self.commit_tag_edit();
                }
                if self.block_edit.is_some() {
                    self.commit_block_edit();
                }
                self.commit_name_edit();
                self.commit_path_edit();
                self.commit_icon_edit();
                self.render();
            }
            Key::Escape => {
                if self.editing() {
                    // 过滤词草稿丢弃回已提交值;其余草稿直接放弃喵
                    self.filter_edit = None;
                    self.name_edit = None;
                    self.path_edit = None;
                    self.icon_edit = None;
                    self.tag_edit = None;
                    self.block_edit = None;
                    self.render();
                } else {
                    self.hide();
                }
            }
            _ => {}
        }
    }

    fn on_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        if let Some(te) = &mut self.filter_edit {
            te.insert_char(ch);
            // 过滤词实时生效: 列表变了,视口回顶,免得结果被旧滚动位置甩出视野喵
            self.scroll = 0.0;
            self.scroll_target = 0.0;
        } else if let Some(te) = &mut self.name_edit {
            te.insert_char(ch);
        } else if let Some(te) = &mut self.path_edit {
            te.insert_char(ch);
        } else if let Some(te) = &mut self.icon_edit {
            te.insert_char(ch);
        } else if let Some(te) = &mut self.tag_edit {
            te.insert_char(ch);
        } else if let Some(te) = &mut self.block_edit {
            te.insert_char(ch);
        } else {
            return;
        }
        self.render();
    }

    // ---------------------------------------------------------------------
    // 交互
    // ---------------------------------------------------------------------

    /// 命中测试并处理点击喵
    fn handle_click(&mut self, x: f32, y: f32) {
        let lx = x / self.scale;
        let ly = y / self.scale;

        let Some(layout) = &self.layout else {
            return;
        };
        let hit = layout
            .hits
            .iter()
            .find(|(rect, _)| rect.left <= lx && lx <= rect.right && rect.top <= ly && ly <= rect.bottom)
            .map(|(r, h)| (*r, *h));

        // 与配置 GUI 统一: 点在输入框外,挂起的草稿立即提交,焦点随点击离开喵
        // (过滤词提交后持久保留,列表保持过滤结果——就是搜索体验的另一半喵)
        let on_input = matches!(
            &hit,
            Some((_, ManagerHit::FilterInput))
                | Some((_, ManagerHit::DetailInput(_)))
                | Some((_, ManagerHit::Block(crate::render::app_manager::BlockPart::Input)))
        );
        if !on_input {
            self.commit_name_edit();
            self.commit_path_edit();
            self.commit_icon_edit();
            self.commit_tag_edit();
            self.commit_filter_edit();
            self.block_edit = None;
        }

        if let Some((rect, hit)) = hit {
            match hit {
                ManagerHit::TitleBar => {
                    let (wx, wy) = self.win_pos.unwrap_or((0, 0));
                    self.win_drag = Some(WinDrag {
                        anchor_screen: (wx + x as i32, wy + y as i32),
                        anchor_pos: (wx, wy),
                    });
                }
                ManagerHit::Close => self.hide(),
                ManagerHit::FilterInput => self.click_filter_input(rect, lx),
                ManagerHit::LayoutToggle => {
                    let mut state = self.state.borrow_mut();
                    state.config.window.layout = state.config.window.layout.cycle();
                    state.persist();
                    log::info!("管理中心排版 → {} 喵", state.config.window.layout.label());
                    drop(state);
                    self.render();
                }
                ManagerHit::Rescan => self.spawn_scan(),
                ManagerHit::ListItem(i) => {
                    let name = self.filtered_apps().get(i).map(|a| a.name.clone());
                    self.select_app(name);
                }
                ManagerHit::FavStar => {
                    if let Some(name) = self.selected.clone() {
                        self.state.borrow_mut().toggle_favorite(&name);
                    }
                    self.render();
                }
                ManagerHit::DetailInput(field) => self.click_detail_input(field, rect, lx),
                ManagerHit::TagChip(i) => self.remove_tag(i),
                ManagerHit::Block(part) => match part {
                    crate::render::app_manager::BlockPart::Input => self.click_block_input(rect, lx),
                    crate::render::app_manager::BlockPart::Chip(i) => self.remove_block_word(i),
                },
                ManagerHit::Reveal => self.reveal_selected(),
                ManagerHit::RemoveApp => self.remove_selected(),
                ManagerHit::ScrollThumb => self.begin_scroll_drag(rect, ly),
            }
        }
    }

    /// 点击屏蔽词输入框喵
    fn click_block_input(&mut self, rect: Rect, lx: f32) {
        if self.block_edit.is_some() {
            let paint = self.box_paint();
            let font = self.fonts.font(12.0);
            if let Some(te) = self.block_edit.as_mut() {
                let caret = caret_from_x(&font, &paint, &te.text, rect.left + 8.0, lx);
                te.begin_select(caret);
            }
        } else {
            self.block_edit = Some(TextEdit::default());
        }
        self.name_edit = None;
        self.path_edit = None;
        self.icon_edit = None;
        self.tag_edit = None;
        self.filter_edit = None;
        self.text_drag = Some(TextDrag { rect });
        self.render();
    }

    /// 添加屏蔽词规则喵(回车提交;与配置 GUI 同一份规则表)喵
    fn commit_block_edit(&mut self) {
        let Some(te) = self.block_edit.take() else {
            return;
        };
        let keyword = te.text.trim().to_string();
        if keyword.is_empty() {
            return;
        }
        let mut state = self.state.borrow_mut();
        let exists = state
            .config
            .search
            .filters
            .iter()
            .any(|f| f.keyword.eq_ignore_ascii_case(&keyword));
        if exists {
            log::debug!("屏蔽词已存在,忽略: {keyword} 喵");
            return;
        }
        state
            .config
            .search
            .filters
            .push(crate::app::config::FilterRule {
                keyword,
                case_sensitive: false,
            });
        let count = state.config.search.filters.len();
        state.persist();
        log::info!("已添加屏蔽词,共 {count} 条规则喵");
    }

    /// 删除屏蔽词规则喵
    fn remove_block_word(&mut self, index: usize) {
        let mut state = self.state.borrow_mut();
        if index < state.config.search.filters.len() {
            let removed = state.config.search.filters.remove(index);
            state.persist();
            log::info!("已移除屏蔽词: {} 喵", removed.keyword);
        }
        drop(state);
        self.render();
    }

    /// 后台重新扫描系统应用喵(结果合并回注册表)喵
    fn spawn_scan(&mut self) {
        if self.scanning {
            log::debug!("扫描进行中,忽略重复请求喵");
            return;
        }
        self.scanning = true;
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let apps = crate::apps::scanner::scan_installed_apps();
            let _ = tx.send(BgEvent::Scanned(apps));
        });
        log::info!("管理中心已发起应用重扫描喵~");
    }

    /// 点击过滤框喵(从已提交的过滤词继续编辑,点击别处不丢文本)喵
    fn click_filter_input(&mut self, rect: Rect, lx: f32) {
        if self.filter_edit.is_some() {
            let paint = self.box_paint();
            let font = self.fonts.font(12.0);
            if let Some(te) = self.filter_edit.as_mut() {
                let caret = caret_from_x(&font, &paint, &te.text, rect.left + 8.0, lx);
                te.begin_select(caret);
            }
        } else {
            self.filter_edit = Some(TextEdit::new(self.filter_text.clone()));
        }
        // 详情编辑槽让位喵(先提交,避免丢改动)喵
        self.commit_name_edit();
        self.commit_path_edit();
        self.commit_icon_edit();
        self.name_edit = None;
        self.path_edit = None;
        self.icon_edit = None;
        self.tag_edit = None;
        self.text_drag = Some(TextDrag { rect });
        self.render();
    }

    /// 删除标签芯片喵
    fn remove_tag(&mut self, index: usize) {
        let Some(name) = self.selected.clone() else {
            return;
        };
        let tag = self.state.borrow().registry.find(&name).and_then(|a| a.tags.get(index).cloned());
        if let Some(tag) = tag {
            self.state.borrow_mut().remove_tag(&name, &tag);
            log::info!("已移除标签: {name} × {tag} 喵");
            self.render();
        }
    }

    /// 打开选中应用所在位置喵
    fn reveal_selected(&mut self) {
        let Some(name) = self.selected.clone() else {
            return;
        };
        let path = self.state.borrow().registry.find(&name).map(|a| a.path.clone());
        if let Some(path) = path {
            let arg = format!("/select,\"{path}\"");
            if self.platform.launch_args("explorer.exe", &arg) {
                log::info!("已打开所在位置: {name} 喵");
            } else {
                log::warn!("打开所在位置失败: {name} 喵");
            }
        }
    }

    /// 移除选中应用喵
    fn remove_selected(&mut self) {
        if let Some(name) = self.selected.take() {
            self.state.borrow_mut().remove_app(&name);
            log::info!("已移除应用 {name} 喵");
            self.render();
        }
    }

    /// 开始拖动滚动条喵
    fn begin_scroll_drag(&mut self, _thumb: Rect, y: f32) {
        let Some(layout) = self.layout.as_ref() else {
            return;
        };
        let Some((track, thumb)) = layout.scrollbar else {
            return;
        };
        let max = (layout.list_content_height - layout.list_rect.height()).max(0.0);
        if max <= 0.0 {
            return;
        }
        self.scroll_drag = Some(ScrollDrag {
            anchor_y: y,
            anchor_prog: (self.scroll / max).clamp(0.0, 1.0),
            travel: (track.height() - thumb.height()).max(1.0),
            max_logical: max,
        });
        self.scroll_target = self.scroll;
    }

    /// 鼠标移动喵(拖选文本 / 拖滚动条 / 拖窗口)喵
    fn on_mouse_move(&mut self, x: f32, y: f32) {
        let ly = y / self.scale;
        if let Some(drag) = self.text_drag {
            let lx = x / self.scale;
            let paint = self.box_paint();
            let font = self.fonts.font(12.0);
            if let Some(te) = &mut self.filter_edit {
                let caret = caret_from_x(&font, &paint, &te.text, drag.rect.left + 8.0, lx);
                te.extend_select(caret);
            } else if let Some(te) = &mut self.name_edit {
                let caret = caret_from_x(&font, &paint, &te.text, drag.rect.left + 8.0, lx);
                te.extend_select(caret);
            } else if let Some(te) = &mut self.path_edit {
                let caret = caret_from_x(&font, &paint, &te.text, drag.rect.left + 8.0, lx);
                te.extend_select(caret);
            } else if let Some(te) = &mut self.icon_edit {
                let caret = caret_from_x(&font, &paint, &te.text, drag.rect.left + 8.0, lx);
                te.extend_select(caret);
            } else if let Some(te) = &mut self.tag_edit {
                let caret = caret_from_x(&font, &paint, &te.text, drag.rect.left + 8.0, lx);
                te.extend_select(caret);
            } else if let Some(te) = &mut self.block_edit {
                let caret = caret_from_x(&font, &paint, &te.text, drag.rect.left + 8.0, lx);
                te.extend_select(caret);
            } else {
                return;
            }
            self.render();
            return;
        }
        if let Some(d) = &self.scroll_drag {
            let dy = ly - d.anchor_y;
            let prog = (d.anchor_prog + dy / d.travel).clamp(0.0, 1.0);
            self.scroll = prog * d.max_logical;
            self.scroll_target = self.scroll;
            self.render();
            return;
        }
        if let Some(d) = &self.win_drag {
            let (wx, wy) = self.win_pos.unwrap_or((0, 0));
            let nx = d.anchor_pos.0 + (wx + x as i32 - d.anchor_screen.0);
            let ny = d.anchor_pos.1 + (wy + y as i32 - d.anchor_screen.1);
            self.platform.move_window(&self.window, nx, ny);
            self.win_pos = Some((nx, ny));
        }
    }

    /// 推进滚动平滑动效,返回是否仍在动喵
    ///
    /// 每帧把目标钳回内容范围内: 过滤/删除让内容变矮时,
    /// 旧的滚动目标会悬空,表现为「看不到结果」,这里统一兜底喵。
    fn step_anim(&mut self) -> bool {
        let max = self
            .layout
            .as_ref()
            .map(|l| (l.list_content_height - l.list_rect.height()).max(0.0))
            .unwrap_or(0.0);
        self.scroll_target = self.scroll_target.clamp(0.0, max);
        let delta = self.scroll_target - self.scroll;
        if delta.abs() > 0.5 {
            self.scroll += delta * SCROLL_EASE;
            true
        } else {
            self.scroll = self.scroll_target;
            false
        }
    }

    /// 处理拖入文件注册喵
    fn drop_files(&mut self, paths: Vec<String>) {
        let mut added = 0usize;
        for path in paths {
            let p = std::path::Path::new(&path);
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(ext.as_str(), "lnk" | "exe" | "bat" | "cmd" | "url") {
                log::warn!("不支持的拖入类型: {path}");
                continue;
            }
            let name = p
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("未命名应用")
                .to_string();
            if self.state.borrow_mut().register_app(&name, &path, None) {
                added += 1;
            }
            log::info!("拖入注册应用: {name} ({path}) 喵");
        }
        if added > 0 {
            log::info!("管理中心拖入注册完成,新增 {added} 个应用喵");
        }
        self.render();
    }

    /// 后台任务结果回填喵
    fn drain_bg_events(&mut self) {
        while let Ok(event) = self.bg_rx.try_recv() {
            match event {
                BgEvent::Icon { name, image } => {
                    self.pending_icons.remove(&name);
                    self.state.borrow_mut().icons.cache_image(name, image);
                }
                BgEvent::Scanned(apps) => {
                    self.scanning = false;
                    let delta = self.state.borrow_mut().merge_scanned(apps);
                    log::info!("管理中心重扫描完成,{} 喵", delta.summary());
                    // 增减有变化就系统通知一声(与热键/托盘触发的扫描同一反馈通道)喵
                    if delta.any() {
                        self.platform
                            .show_notification("应用扫描完成", &delta.summary());
                    }
                    if self.visible {
                        self.render();
                    }
                }
            }
        }
    }

    /// 心跳喵: 检查显示请求、跟进渲染后端、推进动效并渲染喵
    fn tick(&mut self) {
        let should_show = self.state.borrow().manager_visible;
        if should_show && !self.visible {
            self.show();
        }
        self.drain_bg_events();
        // 渲染后端配置变化时重建渲染器(与配置窗口同步)喵
        let backend = self.state.borrow().config.render_backend;
        if backend != self.last_backend {
            self.last_backend = backend;
            self.recreate_renderer();
        }
        if self.visible {
            let animating = self.step_anim();
            self.render();
            let next = if animating { ANIM_FRAME_MS } else { HEARTBEAT_MS };
            if self.timer_interval_ms != next {
                self.timer_interval_ms = next;
                self.platform.set_timer(&self.window, next);
            }
        }
    }
}

impl WindowHandler for AppManagerWindow {
    fn on_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::MouseDown(x, y) => self.handle_click(x, y),
            WindowEvent::MouseMove(x, y) => self.on_mouse_move(x, y),
            WindowEvent::MouseUp => {
                self.win_drag = None;
                self.text_drag = None;
                self.scroll_drag = None;
                // 结束拖选模态(保留选中区,清锚点)喵
                if let Some(te) = &mut self.filter_edit {
                    te.end_select();
                } else if let Some(te) = &mut self.name_edit {
                    te.end_select();
                } else if let Some(te) = &mut self.path_edit {
                    te.end_select();
                } else if let Some(te) = &mut self.icon_edit {
                    te.end_select();
                } else if let Some(te) = &mut self.tag_edit {
                    te.end_select();
                } else if let Some(te) = &mut self.block_edit {
                    te.end_select();
                }
            }
            WindowEvent::Char(ch) => self.on_char(ch),
            WindowEvent::KeyDown(key) => self.edit_key(key),
            WindowEvent::MouseWheel(delta) => {
                if self.visible {
                    let max = self
                        .layout
                        .as_ref()
                        .map(|l| (l.list_content_height - l.list_rect.height()).max(0.0))
                        .unwrap_or(0.0);
                    self.scroll_target = (self.scroll_target - delta / 120.0 * WHEEL_STEP).clamp(0.0, max);
                    self.render();
                }
            }
            WindowEvent::FilesDropped(paths) => self.drop_files(paths),
            WindowEvent::LostFocus => {
                // 失焦不关闭(管理窗口常驻,便于来回对照)喵
            }
            WindowEvent::Timer => self.tick(),
            WindowEvent::Close => self.hide(),
            WindowEvent::Hotkey
            | WindowEvent::ScanHotkey
            | WindowEvent::HotkeyChord { .. }
            | WindowEvent::ImePreedit(_)
            | WindowEvent::ContextMenu(..)
            | WindowEvent::IpcCommand(_) => {}
        }
    }
}
