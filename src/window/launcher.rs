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
use crate::platform::{Key, Platform, PlatformWindow, WindowEvent, WindowHandler};
use crate::render::{layout, FontCache, Layout, Renderer, Theme, paint_scene};
use skia_safe::Image;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 动画帧间隔(ms)喵
const ANIM_INTERVAL_MS: u32 = 16;
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
    platform: Arc<dyn Platform>,
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
    /// 异步运行时喵(慢操作后台线程池)喵
    runtime: tokio::runtime::Runtime,
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
    /// 刚拖完,吞掉下一次 click 喵
    just_dragged: bool,
}

/// 拖拽快照喵
struct DragState {
    origin_x: f64,
    origin_y: f64,
    client_x: f32,
    client_y: f32,
    stage_w: f64,
    stage_h: f64,
    active: bool,
}

impl Launcher {
    /// 装配启动器窗口喵: 计算尺寸、创建窗口、绑定 handler 喵
    ///
    /// 返回启动器窗口句柄,供调用方(注册热键等)使用喵。
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        platform: Arc<dyn Platform>,
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
        let layout = Layout::from_frame(&state.borrow().config, scale, &frame, 0, 0.0);
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

        let renderer = Renderer::new(win_w, win_h).unwrap_or_else(|| {
            log::error!("渲染器初始化失败,即将退出喵~");
            std::process::exit(1);
        });

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("异步运行时初始化失败喵~");
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
            runtime,
            bg_tx,
            bg_rx,
            icon_extractor,
            pending_icons: HashSet::new(),
            commands,
            ime_preedit: String::new(),
            drag: None,
            just_dragged: false,
        };

        // 首次启动后台异步扫描系统应用喵(不阻塞窗口创建)喵
        if launcher.state.borrow().registry.apps.is_empty() {
            log::info!("首次启动,后台异步扫描系统应用喵~");
            launcher.spawn_scan();
        }

        // 绑定事件处理器喵
        platform.set_window_handler(&window, Box::new(launcher));
        // 常驻心跳: 隐藏时也要排空托盘命令喵
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

    /// 字符输入喵
    fn on_char(&mut self, ch: char) {
        if !ch.is_control() {
            self.ime_preedit.clear();
            self.state.borrow_mut().query.push(ch);
            self.refresh_results();
        }
    }

    /// 导航键按下喵
    fn on_key(&mut self, key: Key) {
        match key {
            Key::Enter => self.launch_selected(),
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
                self.state.borrow_mut().query.pop();
                self.refresh_results();
            }
            Key::Up => self.nudge_selection(-1),
            Key::Down => self.nudge_selection(1),
            _ => {}
        }
    }

    /// IME 预览串喵
    fn on_ime_preedit(&mut self, text: String) {
        self.ime_preedit = text;
        self.request_render();
    }

    /// 鼠标按下(点击条目启动 / 开始拖岛)喵
    fn on_mouse_down(&mut self, x: f32, y: f32) {
        if self.just_dragged {
            self.just_dragged = false;
            return;
        }
        let scale = self.platform.scale_factor();
        let layout = self.current_layout(scale);
        for (i, rect) in layout.item_rects.iter().enumerate() {
            if rect.left <= x && x <= rect.right && rect.top <= y && y <= rect.bottom {
                if matches!(self.state.borrow().results.get(i), Some(ListItem::Section(_))) {
                    return;
                }
                self.state.borrow_mut().selected = i;
                self.launch_selected();
                return;
            }
        }
        if self.state.borrow().config.island.draggable {
            let (sw, sh) = self.platform.screen_size();
            let cfg = &self.state.borrow().config.island;
            self.drag = Some(DragState {
                origin_x: cfg.x,
                origin_y: cfg.y,
                client_x: x,
                client_y: y,
                stage_w: sw as f64 / scale as f64,
                stage_h: sh as f64 / scale as f64,
                active: false,
            });
        }
    }

    fn on_mouse_move(&mut self, x: f32, y: f32) {
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let dx = x - drag.client_x;
        let dy = y - drag.client_y;
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

    /// 移动选中(环绕,跳过分组头)喵
    fn nudge_selection(&mut self, delta: isize) {
        if self.state.borrow().results.is_empty() {
            return;
        }
        self.state.borrow_mut().move_selection(delta);
        self.ensure_selected_visible();
        log::debug!("选中: {} 喵", self.state.borrow().selected);
        self.request_render();
    }

    /// 保证选中项在结果面板可视区内,必要时滚动喵
    fn ensure_selected_visible(&mut self) {
        let (len, selected, max_h) = {
            let state = self.state.borrow();
            (
                state.results.len(),
                state.selected as f32,
                state.config.window.height.max(80.0) as f32,
            )
        };
        if len == 0 {
            self.scroll_offset = 0.0;
            return;
        }

        let content_h = len as f32 * layout::ITEM_HEIGHT;
        let panel_h = content_h.min(max_h);

        let item_top = selected * layout::ITEM_HEIGHT;
        let item_bottom = item_top + layout::ITEM_HEIGHT;
        let view_bottom = self.scroll_offset + panel_h;

        if item_top < self.scroll_offset {
            self.scroll_offset = item_top;
        } else if item_bottom > view_bottom {
            self.scroll_offset = item_bottom - panel_h;
        }

        let max_scroll = (content_h - panel_h).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);
    }

    /// 启动选中的应用并隐藏喵
    fn launch_selected(&mut self) {
        let launched = self.state.borrow_mut().launch_selected();
        if launched.is_none() {
            log::debug!("没有可启动的应用喵");
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
        self.runtime.spawn_blocking(move || {
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
                ListItem::App(app) => Some(app.clone()),
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
            self.runtime.spawn_blocking(move || {
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
        self.platform.set_timer(&self.window, ANIM_INTERVAL_MS);
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
            self.platform.set_timer(&self.window, HEARTBEAT_MS);
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
                ANIM_INTERVAL_MS
            } else {
                CARET_INTERVAL_MS
            }
        } else {
            HEARTBEAT_MS
        };
        self.platform.set_timer(&self.window, interval);
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

    fn current_layout(&mut self, scale: f32) -> Layout {
        let (sw, sh) = self.platform.screen_size();
        self.island.set_stage(
            sw as f64 / scale.max(0.01) as f64,
            sh as f64 / scale.max(0.01) as f64,
        );
        let frame = self.island.frame();
        let state = self.state.borrow();
        Layout::from_frame(
            &state.config,
            scale,
            &frame,
            state.results.len(),
            self.scroll_offset,
        )
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
            Layout::from_frame(
                &state.config,
                scale,
                &frame,
                state.results.len(),
                self.scroll_offset,
            )
        };

        let w = layout.window_width.ceil().max(1.0) as i32;
        let h = layout.window_height.ceil().max(1.0) as i32;
        if w != self.renderer.width() || h != self.renderer.height() {
            self.renderer.resize(w, h);
            self.platform.resize_window(&self.window, w, h);
        }
        let win_x = (frame.left as f32 * scale - layout::SHADOW_MARGIN * scale).round() as i32;
        let win_y = (frame.top as f32 * scale - layout::SHADOW_MARGIN * scale).round() as i32;
        self.platform.move_window(&self.window, win_x, win_y);

        let theme = {
            let state = self.state.borrow();
            Theme::resolve(
                state.config.theme.mode,
                state.config.theme.backdrop,
                state.config.island.visual,
            )
        };
        let caret_on = self.caret_on
            && self.visible
            && (!self.state.borrow().query.is_empty() || !self.ime_preedit.is_empty());
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
            WindowEvent::MouseWheel(_) => {}
            WindowEvent::LostFocus => {
                if self.visible {
                    self.hide();
                }
            }
            WindowEvent::Timer => self.tick(),
            WindowEvent::Close => {}
            WindowEvent::FilesDropped(_) => {}
        }
    }
}
