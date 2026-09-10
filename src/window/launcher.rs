//! 启动器窗口喵~
//!
//! 单窗口架构的启动器核心: 持有应用状态、渲染器、弹簧动画,
//! 实现 `WindowHandler` 把平台事件翻译成业务动作喵。
//!
//! 在多窗口架构下,`Launcher` 兼任「应用控制器」: 它的定时器心跳除了推进动画,
//! 还负责轮询后台任务结果、处理托盘/配置窗口投递的应用命令喵。
//!
//! 状态机(显式):
//! * 呼出(show): 显示窗口 → 弹簧淡入 → 输入即搜喵
//! * 隐藏(hide): 失焦/Esc/启动后 → 清空 → 隐藏窗口喵
//! * 展开: 有结果时面板弹簧展开,无结果时折叠喵

use crate::animation::{clamp_dt, DynamicIsland};
use crate::app::{Command, ListItem, SharedState};
use crate::apps::AppInfo;
use crate::apps::icon::IconExtractor;
use crate::platform::{Key, PlatformWindow, Win32Platform, WindowEvent, WindowHandler};
use crate::render::{layout, FontCache, Layout, Renderer, Theme, paint_scene};
use skia_safe::{Image, Paint};
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 光标闪烁间隔(ms)喵
const CARET_INTERVAL_MS: u32 = 500;
/// 隐藏时心跳(ms): 必须常驻,否则托盘命令永远排不空喵
const HEARTBEAT_MS: u32 = 100;
/// 光标闪烁半周期喵
const CARET_HALF_PERIOD: Duration = Duration::from_millis(500);
/// 拖拽启动阈值(物理 px)喵
const DRAG_SLOP: f32 = 4.0;

/// 后台任务结果喵(异步任务完成后回传主线程)喵
enum BgEvent {
    /// 应用扫描完成喵
    Scanned(Vec<AppInfo>),
    /// 图标提取完成喵
    Icon { name: String, image: Option<Image> },
}

/// 启动器窗口喵
pub struct Launcher {
    /// 共享应用状态喵
    state: SharedState,
    /// 平台句柄喵
    platform: Arc<Win32Platform>,
    /// 窗口句柄喵
    window: PlatformWindow,
    /// 配置窗口句柄喵(打开设置时显示)喵
    settings_window: PlatformWindow,
    /// Skia 渲染器喵
    renderer: Renderer,
    /// 字体缓存喵
    fonts: FontCache,
    /// 灵动岛内核喵
    island: DynamicIsland,
    /// 是否呼出喵
    visible: bool,
    /// 光标是否可见(闪烁)喵
    caret_on: bool,
    /// 光标闪烁累计时间喵
    caret_acc: Duration,
    /// 上一帧时间喵
    last_tick: Option<Instant>,
    /// 结果面板滚动偏移(逻辑像素)喵
    scroll_offset: f32,
    /// BGRA 像素缓冲(复用)喵
    pixels: Vec<u8>,
    /// 后台任务结果发送端喵
    bg_tx: mpsc::Sender<BgEvent>,
    /// 后台任务结果接收端喵
    bg_rx: mpsc::Receiver<BgEvent>,
    /// 图标提取器喵(可移入后台线程)喵
    icon_extractor: IconExtractor,
    /// 正在提取图标的名称集合喵(避免重复提交)喵
    pending_icons: HashSet<String>,
    /// 应用命令队列喵(托盘/配置窗口投递)喵
    commands: Rc<RefCell<VecDeque<Command>>>,
    /// IME 预编辑串喵
    ime_preedit: String,
    /// 拖拽起点喵
    drag: Option<DragState>,
    /// 滚动条拖动状态喵
    drag_scroll: Option<ScrollDrag>,
    /// 刚拖完,吞掉下一次 click 喵
    just_dragged: bool,
    /// 当前生效的定时器间隔(ms,仅变化时重设,避免每帧重置导致抖动)喵
    timer_interval_ms: u32,
    /// 上次设置窗口位置喵(未变时跳过 SetWindowPos,省同步开销)喵
    last_win_pos: Option<(i32, i32)>,
    /// 查询光标位置(字节偏移,UTF-8 边界,输入插入点)喵
    caret: usize,
    /// 查询选中区(字节区间 [lo, hi))喵,None 表示无选中喵
    selection: Option<(usize, usize)>,
    /// 拖选锚点(字节偏移): 按下鼠标时的位置,反向拖选的基准喵
    sel_anchor: usize,
    /// 是否正处于搜索框内拖选喵
    selecting: bool,
    /// 搜索框是否获得点击焦点喵(空文本时也闪烁光标)喵
    search_focused: bool,
}

/// 拖拽快照喵
struct DragState {
    origin_x: f64,
    origin_y: f64,
    /// 拖拽起点(屏幕坐标): 窗口会随岛移动,必须锚定屏幕而非客户区,否则反馈抖动喵
    anchor_screen_x: f32,
    anchor_screen_y: f32,
    stage_w: f64,
    stage_h: f64,
    active: bool,
}

/// 滚动条拖动快照喵
struct ScrollDrag {
    /// 拖拽起点(客户区 y,滚动不移动窗口,可用客户区坐标)喵
    anchor_y: f32,
    /// 起点进度(0..1)喵
    anchor_prog: f32,
    /// 滑块可移动距离(物理 px)喵
    travel: f32,
    /// 最大滚动量(逻辑 px)喵
    max_logical: f32,
}

impl Launcher {
    /// 装配启动器窗口喵: 计算尺寸、创建窗口、绑定 handler 喵
    ///
    /// 返回启动器窗口句柄,供调用方(注册热键等)使用喵。
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        platform: Arc<Win32Platform>,
        state: SharedState,
        commands: Rc<RefCell<VecDeque<Command>>>,
        settings_window: PlatformWindow,
    ) -> PlatformWindow {
        // 计算初始窗口尺寸与位置(按岛配置百分比锚点)喵
        let scale = platform.scale_factor();
        let (screen_w, screen_h) = platform.screen_size();
        let island_cfg = state.borrow().config.island.clone();
        let mut island = DynamicIsland::new(island_cfg);
        island.set_stage(screen_w as f64 / scale as f64, screen_h as f64 / scale as f64);
        let frame = island.frame();
        let layout = Layout::from_frame(&state.borrow().config, scale, &frame, &[], 0.0);
        let win_w = layout.window_width.ceil().max(1.0) as i32;
        let win_h = layout.window_height.ceil().max(1.0) as i32;
        let x = (frame.left as f32 * scale - layout::SHADOW_MARGIN * scale).round() as i32;
        let y = (frame.top as f32 * scale - layout::SHADOW_MARGIN * scale).round() as i32;

        let spec = crate::platform::WindowSpec {
            x,
            y,
            width: win_w,
            height: win_h,
        };
        let window = platform.create_window(&spec).unwrap_or_else(|| {
            log::error!("启动器窗口创建失败,即将退出喵~");
            std::process::exit(1);
        });

        let backend = state.borrow().config.render_backend;
        let renderer = Renderer::new(win_w, win_h, backend, &platform).unwrap_or_else(|| {
            log::error!("渲染器初始化失败,即将退出喵~");
            std::process::exit(1);
        });

        let (bg_tx, bg_rx) = mpsc::channel();
        let icon_extractor = state.borrow().icons.extractor();

        let mut launcher = Launcher {
            state: state.clone(),
            platform: platform.clone(),
            window,
            settings_window,
            renderer,
            fonts: FontCache::new(),
            island,
            visible: false,
            caret_on: true,
            caret_acc: Duration::ZERO,
            last_tick: None,
            scroll_offset: 0.0,
            pixels: Vec::new(),
            bg_tx,
            bg_rx,
            icon_extractor,
            pending_icons: HashSet::new(),
            commands,
            ime_preedit: String::new(),
            drag: None,
            drag_scroll: None,
            just_dragged: false,
            timer_interval_ms: HEARTBEAT_MS,
            last_win_pos: None,
            caret: 0,
            selection: None,
            sel_anchor: 0,
            selecting: false,
            search_focused: false,
        };

        // 首次启动后台异步扫描系统应用喵(不阻塞窗口创建)喵
        if launcher.state.borrow().registry.apps.is_empty() {
            log::info!("首次启动,后台异步扫描系统应用喵~");
            launcher.spawn_scan();
        }

        // 绑定事件处理器喵
        platform.set_window_handler(&window, Box::new(launcher));
        // 常驻心跳: 隐藏时也要排空托盘命令喵(字段初始值已对齐到此间隔)喵
        platform.set_timer(&window, HEARTBEAT_MS);

        window
    }

    // ---------------------------------------------------------------------
    // 显示 / 隐藏
    // ---------------------------------------------------------------------

    /// 呼出搜索框喵~
    fn show(&mut self) {
        log::info!("呼出搜索框喵~");
        self.visible = true;
        self.scroll_offset = 0.0;
        self.caret_on = true;
        self.caret_acc = Duration::ZERO;
        self.caret = 0;
        self.selection = None;
        self.sel_anchor = 0;
        self.selecting = false;
        self.search_focused = false;
        {
            let mut state = self.state.borrow_mut();
            state.reset_search();
            state.launcher_visible = true;
        }
        self.ime_preedit.clear();
        self.sync_island_from_config(false);
        self.island.go(true, self.want_expanded());
        self.platform.show_window(&self.window, true);
        self.platform.focus_window(&self.window);
        self.start_animation();
        self.request_render();
    }

    /// 隐藏搜索框喵~
    fn hide(&mut self) {
        log::info!("隐藏搜索框喵~");
        self.visible = false;
        self.ime_preedit.clear();
        self.caret = 0;
        self.selection = None;
        self.sel_anchor = 0;
        self.selecting = false;
        self.search_focused = false;
        {
            let mut state = self.state.borrow_mut();
            state.reset_search();
            state.launcher_visible = false;
        }
        self.island.go(false, false);
        self.start_animation();
        self.request_render();
    }

    /// 切换呼出/隐藏喵
    fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    // ---------------------------------------------------------------------
    // 输入处理
    // ---------------------------------------------------------------------

    /// 字符输入喵(支持: 插入光标处 / 替换选中区)喵
    fn on_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        self.ime_preedit.clear();
        let mut q = self.state.borrow_mut();
        let caret = self.caret.min(q.query.len());
        if let Some((lo, hi)) = self.selection {
            let (lo, hi) = (lo.min(q.query.len()), hi.min(q.query.len()));
            q.query.replace_range(lo..hi, &ch.to_string());
            self.caret = lo + ch.len_utf8();
        } else {
            q.query.insert(caret, ch);
            self.caret = caret + ch.len_utf8();
        }
        self.selection = None;
        drop(q);
        self.refresh_results();
    }

    /// 导航键按下喵(网格走几何导航,列表走线性跳步)喵
    fn on_key(&mut self, key: Key) {
        match key {
            Key::Enter => self.execute_selected(),
            Key::Escape => {
                if self.island.state == crate::animation::IslandState::Expanded {
                    self.island.go(true, false);
                    self.start_animation();
                    self.request_render();
                } else {
                    self.hide();
                }
            }
            Key::Backspace => {
                let mut q = self.state.borrow_mut();
                let caret = self.caret.min(q.query.len());
                if let Some((lo, hi)) = self.selection {
                    let (lo, hi) = (lo.min(q.query.len()), hi.min(q.query.len()));
                    q.query.replace_range(lo..hi, "");
                    self.caret = lo;
                } else if caret > 0 {
                    let before = q.query[..caret]
                        .chars()
                        .next_back()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    q.query.replace_range((caret - before)..caret, "");
                    self.caret = caret - before;
                }
                self.selection = None;
                drop(q);
                self.refresh_results();
            }
            Key::Delete => {
                let mut q = self.state.borrow_mut();
                let caret = self.caret.min(q.query.len());
                if let Some((lo, hi)) = self.selection {
                    let (lo, hi) = (lo.min(q.query.len()), hi.min(q.query.len()));
                    q.query.replace_range(lo..hi, "");
                    self.caret = lo;
                } else if caret < q.query.len() {
                    let next = q.query[caret..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    q.query.replace_range(caret..(caret + next), "");
                }
                self.selection = None;
                drop(q);
                self.refresh_results();
            }
            Key::Up => self.nudge_grid(-1, 0),
            Key::Down => self.nudge_grid(1, 0),
            // 网格排版: 左右键切换列喵
            Key::Left if self.grid_mode() => self.nudge_grid(0, -1),
            Key::Right if self.grid_mode() => self.nudge_grid(0, 1),
            _ => {}
        }
    }

    /// IME 预览串喵
    fn on_ime_preedit(&mut self, text: String) {
        self.ime_preedit = text;
        self.request_render();
    }

    /// 鼠标按下(搜索框设光标 / 点击条目启动 / 滚动条拖滑块 / 岛外拖拽)喵
    fn on_mouse_down(&mut self, x: f32, y: f32) {
        if self.just_dragged {
            self.just_dragged = false;
            return;
        }
        // 点击即抢回键盘焦点: 呼出时若被前台锁拒绝,靠点击自救喵
        self.platform.focus_window(&self.window);
        let scale = self.platform.scale_factor();
        let layout = self.current_layout(scale);

        // 滚动条命中: 只有按住「滑块」才进入拖动,点轨道空白不跳转不触发喵
        if let Some((track, thumb)) = layout.scrollbar {
            let on_track = x >= track.left - 4.0
                && x <= track.right + 4.0
                && y >= track.top - 4.0
                && y <= track.bottom + 4.0;
            let on_thumb = x >= thumb.left - 4.0
                && x <= thumb.right + 4.0
                && y >= thumb.top - 4.0
                && y <= thumb.bottom + 4.0;
            if on_track {
                if on_thumb {
                    let max_px = (layout.content_height - layout.panel_rect.height()).max(0.5);
                    let prog = (self.scroll_offset * scale / max_px).clamp(0.0, 1.0);
                    self.drag_scroll = Some(ScrollDrag {
                        anchor_y: y,
                        anchor_prog: prog,
                        travel: (track.height() - thumb.height()).max(1.0),
                        max_logical: max_px / scale,
                    });
                }
                return;
            }
        }

        // 搜索框命中(任意状态): 单击聚焦并设定光标位置,为拖选锚定起点喵。
        // 收起态用户点了搜索框就说明想编辑文本,只有胶囊四周的留白仍可拖岛喵。
        {
            let search = layout.search_rect;
            if search.left <= x && x <= search.right && search.top <= y && y <= search.bottom {
                let caret = self.caret_at(&layout, x);
                self.caret = caret;
                self.sel_anchor = caret;
                self.selection = Some((caret, caret));
                self.selecting = true;
                self.search_focused = true;
                self.request_render();
                return;
            }
        }

        for (i, rect) in layout.item_rects.iter().enumerate() {
            if rect.left <= x && x <= rect.right && rect.top <= y && y <= rect.bottom {
                if matches!(self.state.borrow().results.get(i), Some(ListItem::Section(_))) {
                    return;
                }
                self.state.borrow_mut().selected = i;
                self.execute_selected();
                return;
            }
        }
        if self.state.borrow().config.island.draggable {
            let (sw, sh) = self.platform.screen_size();
            let cfg = &self.state.borrow().config.island;
            // 锚定屏幕坐标: 岛随鼠标移动时窗口也在动,客户区坐标会漂移,
            // 只有屏幕坐标差值才是真实位移喵
            let (wx, wy) = self.last_win_pos.unwrap_or((0, 0));
            self.drag = Some(DragState {
                origin_x: cfg.x,
                origin_y: cfg.y,
                anchor_screen_x: wx as f32 + x,
                anchor_screen_y: wy as f32 + y,
                stage_w: sw as f64 / scale as f64,
                stage_h: sh as f64 / scale as f64,
                active: false,
            });
        }
    }

    fn on_mouse_move(&mut self, x: f32, y: f32) {
        // 搜索框内拖选: 以按下时的锚点为基准,正向/反向拖选都可靠喵
        if self.selecting {
            let layout = self.current_layout(self.platform.scale_factor().max(0.01));
            let caret = self.caret_at(&layout, x);
            let anchor = self.sel_anchor;
            self.selection = Some((anchor.min(caret), anchor.max(caret)));
            self.request_render();
            return;
        }

        // 拖动滚动条(仅滑块): 滚动不移动窗口,客户区坐标稳定可用喵
        if let Some(ref ds) = self.drag_scroll {
            let dy = y - ds.anchor_y;
            let prog = (ds.anchor_prog + dy / ds.travel).clamp(0.0, 1.0);
            self.scroll_offset = prog * ds.max_logical;
            self.request_render();
            return;
        }

        // 悬停吸附: 指针进入扩展面板时,焦点跟随指针吸附最合适的应用喵
        if self.drag.is_none() && self.visible && self.want_expanded() {
            let layout = self.current_layout(self.platform.scale_factor().max(0.01));
            let panel = layout.panel_rect;
            if panel.left <= x && x <= panel.right && panel.top <= y && y <= panel.bottom {
                if let Some((i, _)) = layout
                    .item_rects
                    .iter()
                    .enumerate()
                    .find(|(_, r)| r.left <= x && x <= r.right && r.top <= y && y <= r.bottom)
                {
                    let is_app =
                        matches!(self.state.borrow().results.get(i), Some(ListItem::Item(_)));
                    if is_app && self.state.borrow().selected != i {
                        self.state.borrow_mut().selected = i;
                        log::debug!("悬停吸附选中: {} 喵", i);
                        self.request_render();
                    }
                }
                return; // 指针在面板内,不再进入岛拖拽喵
            }
        }

        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        // 当前屏幕坐标 = 窗口屏幕位置 + 客户区坐标喵
        let (wx, wy) = self.last_win_pos.unwrap_or((0, 0));
        let dx = (wx as f32 + x) - drag.anchor_screen_x;
        let dy = (wy as f32 + y) - drag.anchor_screen_y;
        if !drag.active && dx.abs() < DRAG_SLOP && dy.abs() < DRAG_SLOP {
            return;
        }
        drag.active = true;
        let scale = self.platform.scale_factor().max(0.01);
        let nx = (drag.origin_x + (dx / scale) as f64 / drag.stage_w * 100.0).clamp(2.0, 98.0);
        let ny = (drag.origin_y + (dy / scale) as f64 / drag.stage_h * 100.0).clamp(2.0, 98.0);
        let mut cfg = self.state.borrow().config.island.clone();
        cfg.x = nx;
        cfg.y = ny;
        self.island.set_config(cfg.clone(), false);
        {
            let mut state = self.state.borrow_mut();
            state.config.island = cfg;
        }
        self.request_render();
    }

    fn on_mouse_up(&mut self) {
        self.selecting = false;
        self.drag_scroll = None;
        if let Some(drag) = self.drag.take()
            && drag.active
        {
            self.just_dragged = true;
            self.state.borrow_mut().persist();
            log::info!("灵动岛位置已保存: ({:.1}%, {:.1}%) 喵", self.island.config.x, self.island.config.y);
        }
    }

    /// 刷新搜索结果并展开/折叠面板喵
    fn refresh_results(&mut self) {
        let count = self.state.borrow_mut().refresh_results();
        // 查询变化,滚动回到顶部喵
        self.scroll_offset = 0.0;
        log::debug!("查询更新: {:?}, 结果 {count} 条喵", self.state.borrow().query);
        // 为缺图标的条目发起异步提取喵
        self.request_missing_icons();
        if self.state.borrow().config.island.auto_morph {
            self.island.go(true, self.want_expanded());
        }
        self.start_animation();
        self.request_render();
    }

    /// 线性移动选中(为列表模式服务,跳过分组头)喵
    fn nudge_selection(&mut self, delta: isize) {
        if self.state.borrow().results.is_empty() {
            return;
        }
        self.state.borrow_mut().move_selection(delta);
        self.ensure_selected_visible();
        log::debug!("选中: {} 喵", self.state.borrow().selected);
        self.request_render();
    }

    /// 是否网格排版喵
    fn grid_mode(&self) -> bool {
        self.state.borrow().config.window.layout == crate::app::config::AppLayout::Grid
    }

    /// 方向键导航喵: 网格走几何定位(同列/同行),列表走线性跳步喵
    fn nudge_grid(&mut self, vy: isize, hx: isize) {
        if self.state.borrow().results.is_empty() {
            return;
        }
        if !self.grid_mode() {
            if vy != 0 {
                self.nudge_selection(vy);
            }
            return;
        }
        let scale = self.platform.scale_factor().max(0.01);
        let layout = self.current_layout(scale);
        let sections: Vec<bool> = self
            .state
            .borrow()
            .results
            .iter()
            .map(|i| matches!(i, ListItem::Section(_)))
            .collect();
        let cur = self.state.borrow().selected;
        let target = if vy != 0 {
            layout::grid_vertical_target(&layout.item_rects, &sections, cur, vy)
        } else if hx != 0 {
            layout::grid_horizontal_target(&layout.item_rects, &sections, cur, hx)
        } else {
            None
        };
        if let Some(target) = target
            && target != cur
        {
            self.state.borrow_mut().selected = target;
            self.ensure_selected_visible();
            log::debug!("网格导航: {cur} → {target} 喵");
            self.request_render();
        }
    }

    /// 由鼠标 x 坐标换算查询光标位置(字节偏移,与绘制共用同一套几何)喵
    fn caret_at(&self, layout: &Layout, x: f32) -> usize {
        let rect = layout.search_rect;
        let text_x = (rect.left + rect.height() * 0.95).round();
        let font = self.fonts.font(rect.height() * 0.38);
        let paint = Paint::default();
        let query = &self.state.borrow().query;
        let total = font.measure_str(query, Some(&paint)).0;
        if query.is_empty() || x >= text_x + total {
            return query.len();
        }
        if x <= text_x {
            return 0;
        }
        // 线性扫描各字符边界,取视觉最近的一个(与绘制像素对齐)喵
        let mut best = 0usize;
        let mut best_dx = (x - text_x).abs();
        for (i, _) in query.char_indices() {
            let w = font.measure_str(&query[..i], Some(&paint)).0;
            let dx = (x - (text_x + w)).abs();
            if dx < best_dx {
                best_dx = dx;
                best = i;
            }
        }
        best
    }

    /// 保证选中项在结果面板可视区内,必要时滚动喵
    ///
    /// 布局几何统一用 `current_layout` 的物理像素矩形计算(网格/列表都适用)。
    /// 滚动量必须是**增量**拼接: 旧实现把「超过视口的越界量」直接当成新的绝对滚动,
    /// 导致每次按键都把内容猛然拽回顶部再落下,表现成窗口/内容纵向反复跳跃喵。
    fn ensure_selected_visible(&mut self) {
        if self.state.borrow().results.is_empty() {
            self.scroll_offset = 0.0;
            return;
        }
        let scale = self.platform.scale_factor().max(0.01);
        let layout = self.current_layout(scale);
        let panel = layout.panel_rect;
        let pad = 8.0 * scale;

        let Some(r) = layout.item_rects.get(self.state.borrow().selected) else {
            return;
        };
        let max_px = (layout.content_height - panel.height()).max(0.0);
        let mut scroll_px = self.scroll_offset * scale;
        if r.top < panel.top + pad {
            // 选中跑到视口上方 → 上滚补齐越界量喵
            scroll_px += r.top - (panel.top + pad);
        } else if r.bottom > panel.bottom - pad {
            // 选中跑到视口下方 → 下滚补齐越界量喵
            scroll_px += r.bottom - (panel.bottom - pad);
        }
        self.scroll_offset = scroll_px.clamp(0.0, max_px) / scale;
    }

    /// 滚轮滚动结果面板喵
    fn on_wheel(&mut self, delta: f32) {
        if !self.visible || !self.want_expanded() {
            return;
        }
        let scale = self.platform.scale_factor().max(0.01);
        let layout = self.current_layout(scale);
        let max_px = (layout.content_height - layout.panel_rect.height()).max(0.0);
        if max_px <= 0.0 {
            return;
        }
        let cur = self.scroll_offset * scale;
        let next = (cur - delta / 120.0 * 48.0 * scale).clamp(0.0, max_px);
        self.scroll_offset = next / scale;
        self.request_render();
    }

    /// 执行选中条目的动作并隐藏喵
    fn execute_selected(&mut self) {
        let executed = self.state.borrow_mut().execute_selected();
        if executed.is_none() {
            log::debug!("没有可执行的条目喵");
        }
        self.hide();
    }

    // ---------------------------------------------------------------------
    // 应用命令(托盘/配置窗口投递)喵
    // ---------------------------------------------------------------------

    /// 打开配置窗口喵
    fn open_settings(&mut self) {
        log::info!("打开配置窗口喵~");
        self.state.borrow_mut().settings_visible = true;
        self.platform.show_window(&self.settings_window, true);
        self.platform.focus_window(&self.settings_window);
    }

    /// 重启应用喵
    fn restart(&mut self) {
        log::info!("重启应用喵~");
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).spawn();
        }
        self.platform.quit();
    }

    /// 退出应用喵
    fn quit(&mut self) {
        log::info!("退出应用喵~");
        self.platform.quit();
    }

    /// 处理应用命令队列喵
    fn drain_commands(&mut self) {
        let commands: Vec<Command> = self.commands.borrow_mut().drain(..).collect();
        for cmd in commands {
            match cmd {
                Command::ToggleLauncher => self.toggle(),
                Command::OpenSettings => self.open_settings(),
                Command::Rescan => self.spawn_scan(),
                Command::Restart => self.restart(),
                Command::ReapplyHotkey => {
                    self.platform.unregister_global_hotkey();
                    let hotkey = self.state.borrow().config.hotkey.clone();
                    if hotkey.enabled {
                        let ok = self
                            .platform
                            .register_global_hotkey(&hotkey.modifiers, &hotkey.key, self.window);
                        log::info!(
                            "全局热键已重注册: {}+{} 成功={ok} 喵",
                            hotkey.modifiers,
                            hotkey.key
                        );
                    } else {
                        log::info!("全局热键已禁用喵");
                    }
                }
                Command::RecreateRenderer => self.recreate_renderer(),
                Command::Quit => self.quit(),
            }
        }
    }

    // ---------------------------------------------------------------------
    // 后台异步任务
    // ---------------------------------------------------------------------

    /// 后台扫描系统应用喵(不阻塞消息循环)喵
    fn spawn_scan(&mut self) {
        let tx = self.bg_tx.clone();
        std::thread::spawn(move || {
            let apps = crate::apps::scanner::scan_installed_apps();
            let _ = tx.send(BgEvent::Scanned(apps));
        });
    }

    /// 为缺图标的条目发起异步图标提取喵
    fn request_missing_icons(&mut self) {
        let extractor = self.icon_extractor.clone();
        // 克隆结果列表,避免迭代期间借用 self.state 喵
        let apps: Vec<AppInfo> = self
            .state
            .borrow()
            .results
            .iter()
            .filter_map(|i| match i {
                ListItem::Item(item) => match &item.icon {
                    crate::search::ItemIcon::App(app) => Some(app.clone()),
                    crate::search::ItemIcon::Builtin(_) => None,
                },
                ListItem::Section(_) => None,
            })
            .collect();
        for app in apps {
            if self.pending_icons.contains(&app.name) {
                continue;
            }
            if self.state.borrow_mut().icons.cached_image(&app).is_some() {
                continue;
            }
            self.pending_icons.insert(app.name.clone());
            let tx = self.bg_tx.clone();
            let extractor = extractor.clone();
            std::thread::spawn(move || {
                let (name, image) = extractor.extract(app);
                let _ = tx.send(BgEvent::Icon { name, image });
            });
        }
    }

    /// 处理后台任务结果喵(主线程轮询)喵
    fn drain_bg_events(&mut self) {
        while let Ok(event) = self.bg_rx.try_recv() {
            match event {
                BgEvent::Scanned(apps) => {
                    self.state.borrow_mut().merge_scanned(apps);
                }
                BgEvent::Icon { name, image } => {
                    self.pending_icons.remove(&name);
                    self.state.borrow_mut().icons.cache_image(name, image);
                }
            }
        }
    }

    // ---------------------------------------------------------------------
    // 动画与渲染
    // ---------------------------------------------------------------------

    /// 启动动画帧定时器喵
    fn start_animation(&mut self) {
        self.last_tick = None;
        self.set_timer_interval(self.anim_interval());
    }

    /// 动画帧间隔: 优先取配置帧率,0 = 跟随显示器刷新率喵
    fn anim_interval(&self) -> u32 {
        let cfg_fps = self.state.borrow().config.island.anim_fps;
        let fps = if cfg_fps == 0 {
            self.platform.display_refresh_rate().max(30)
        } else {
            cfg_fps.clamp(30, 144)
        };
        (1000 / fps).clamp(6, 16)
    }

    /// 设置定时器间隔,仅在实际变化时才重设喵
    ///
    /// 每次重置会让 WM_TIMER 相位归零,若每帧都重设,实际周期会拖成
    /// 「间隔 + 渲染耗时」,节奏忽快忽慢喵。因此只在切换档位时重设喵。
    fn set_timer_interval(&mut self, ms: u32) {
        if self.timer_interval_ms != ms {
            self.timer_interval_ms = ms;
            self.platform.set_timer(&self.window, ms);
        }
    }

    /// 动画帧推进喵
    fn tick(&mut self) {
        let now = Instant::now();
        let elapsed = self.last_tick.map(|t| now - t).unwrap_or(Duration::ZERO);
        self.last_tick = Some(now);

        self.drain_bg_events();
        self.drain_commands();
        self.sync_island_from_config(true);

        let hidden_done = !self.visible && self.island.settled();
        if hidden_done {
            self.platform.show_window(&self.window, false);
            self.set_timer_interval(HEARTBEAT_MS);
            return;
        }

        if self.visible {
            self.caret_acc += elapsed;
            if self.caret_acc >= CARET_HALF_PERIOD {
                self.caret_acc = Duration::ZERO;
                self.caret_on = !self.caret_on;
            }
            self.island.go(true, self.want_expanded());
        }

        let (sw, sh) = self.platform.screen_size();
        let scale = self.platform.scale_factor().max(0.01);
        self.island
            .set_stage(sw as f64 / scale as f64, sh as f64 / scale as f64);
        let animating = {
            self.island.step(clamp_dt(elapsed.as_secs_f32()) as f64);
            !self.island.settled()
        };

        self.request_render();

        let interval = if animating || self.visible {
            if animating {
                self.anim_interval()
            } else {
                CARET_INTERVAL_MS
            }
        } else {
            HEARTBEAT_MS
        };
        self.set_timer_interval(interval);
    }

    fn want_expanded(&self) -> bool {
        let state = self.state.borrow();
        state.results_visible && !state.results.is_empty()
    }

    fn sync_island_from_config(&mut self, animate: bool) {
        let cfg = self.state.borrow().config.island.clone();
        if cfg != self.island.config {
            log::debug!("灵动岛配置热更新喵");
            self.island.set_config(cfg, animate);
            self.start_animation();
        }
    }

    /// 按配置重建渲染器喵(切换 CPU/GPU 时调用,失败保留原渲染器)喵
    fn recreate_renderer(&mut self) {
        let backend = self.state.borrow().config.render_backend;
        let (w, h) = (self.renderer.width(), self.renderer.height());
        if let Some(renderer) = Renderer::new(w, h, backend, &self.platform) {
            log::info!("启动器渲染器已重建 → {} 喵", renderer.mode().label());
            self.renderer = renderer;
            self.request_render();
        }
    }

    fn current_layout(&mut self, scale: f32) -> Layout {
        let (sw, sh) = self.platform.screen_size();
        self.island.set_stage(
            sw as f64 / scale.max(0.01) as f64,
            sh as f64 / scale.max(0.01) as f64,
        );
        let frame = self.island.frame();
        let state = self.state.borrow();
        let sections: Vec<bool> = state
            .results
            .iter()
            .map(|i| matches!(i, ListItem::Section(_)))
            .collect();
        Layout::from_frame(&state.config, scale, &frame, &sections, self.scroll_offset)
    }

    /// 渲染一帧并呈现喵
    fn request_render(&mut self) {
        let scale = self.platform.scale_factor().max(0.01);
        let (sw, sh) = self.platform.screen_size();
        self.island
            .set_stage(sw as f64 / scale as f64, sh as f64 / scale as f64);
        let frame = self.island.frame();
        let layout = {
            let state = self.state.borrow();
            let sections: Vec<bool> = state
                .results
                .iter()
                .map(|i| matches!(i, ListItem::Section(_)))
                .collect();
            Layout::from_frame(&state.config, scale, &frame, &sections, self.scroll_offset)
        };

        let w = layout.window_width.ceil().max(1.0) as i32;
        let h = layout.window_height.ceil().max(1.0) as i32;
        if w != self.renderer.width() || h != self.renderer.height() {
            self.renderer.resize(w, h);
            self.platform.resize_window(&self.window, w, h);
        }
        let win_x = (frame.left as f32 * scale - layout::SHADOW_MARGIN * scale).round() as i32;
        let win_y = (frame.top as f32 * scale - layout::SHADOW_MARGIN * scale).round() as i32;
        // 位置未变时跳过 SetWindowPos,避免每帧向 DWM 发起同步重排(拖累帧率)喵
        let pos = (win_x, win_y);
        if self.last_win_pos != Some(pos) {
            self.last_win_pos = Some(pos);
            self.platform.move_window(&self.window, win_x, win_y);
        }

        let theme = {
            let state = self.state.borrow();
            Theme::resolve(state.config.theme.preset)
        };
        let caret_on = self.caret_on
            && self.visible
            && (self.search_focused
                || !self.state.borrow().query.is_empty()
                || !self.ime_preedit.is_empty());
        {
            let canvas = self.renderer.canvas();
            let mut state = self.state.borrow_mut();
            paint_scene(
                canvas,
                &theme,
                &layout,
                &mut state,
                &self.fonts,
                caret_on,
                &self.ime_preedit,
                self.caret,
                self.selection,
            );
        }

        self.renderer.read_bgra(&mut self.pixels);
        self.platform.present(&self.window, w, h, &self.pixels);
    }
}

impl WindowHandler for Launcher {
    fn on_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::Hotkey => self.toggle(),
            WindowEvent::KeyDown(key) => self.on_key(key),
            WindowEvent::Char(ch) => self.on_char(ch),
            WindowEvent::ImePreedit(text) => self.on_ime_preedit(text),
            WindowEvent::MouseDown(x, y) => self.on_mouse_down(x, y),
            WindowEvent::MouseMove(x, y) => self.on_mouse_move(x, y),
            WindowEvent::MouseUp => self.on_mouse_up(),
            WindowEvent::MouseWheel(delta) => self.on_wheel(delta),
            WindowEvent::LostFocus => {
                if self.visible {
                    self.hide();
                }
            }
            WindowEvent::Timer => self.tick(),
            WindowEvent::HotkeyChord { .. } => {}
            WindowEvent::Close => {}
            WindowEvent::FilesDropped(_) => {}
        }
    }
}
