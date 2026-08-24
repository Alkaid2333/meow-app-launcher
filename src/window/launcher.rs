//! 启动器窗口喵~
//!
//! 单窗口架构的启动器核心: 持有应用状态、渲染器、弹簧动画,
//! 实现 `WindowHandler` 把平台事件翻译成业务动作喵。
//!
//! 状态机(显式):
//! * 呼出(show): 显示窗口 → 弹簧淡入 → 输入即搜喵
//! * 隐藏(hide): 失焦/Esc/启动后 → 清空 → 隐藏窗口喵
//! * 展开: 有结果时面板弹簧展开,无结果时折叠喵

use crate::animation::{LauncherSprings, params};
use crate::app::AppState;
use crate::apps::AppInfo;
use crate::apps::icon::IconExtractor;
use crate::platform::{Key, Platform, PlatformWindow, WindowEvent, WindowHandler};
use crate::render::{FontCache, Layout, Renderer, Theme, layout, paint_scene};
use skia_safe::Image;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// 动画帧间隔(ms)喵
const ANIM_INTERVAL_MS: u32 = 16;
/// 光标闪烁间隔(ms)喵
const CARET_INTERVAL_MS: u32 = 500;
/// 光标闪烁半周期喵
const CARET_HALF_PERIOD: Duration = Duration::from_millis(500);
/// 呼出时的屏幕垂直位置比例(距顶部)喵
const TOP_RATIO: f32 = 0.18;

/// 后台任务结果喵(异步任务完成后回传主线程)喵
enum BackgroundEvent {
    /// 应用扫描完成喵
    Scanned(Vec<AppInfo>),
    /// 图标提取完成喵
    Icon { name: String, image: Option<Image> },
}

/// 启动器窗口喵
pub struct Launcher {
    /// 应用状态喵
    state: AppState,
    /// 平台句柄喵
    platform: Arc<dyn Platform>,
    /// 窗口句柄喵
    window: PlatformWindow,
    /// Skia 渲染器喵
    renderer: Renderer,
    /// 字体缓存喵
    fonts: FontCache,
    /// 弹簧动画喵
    springs: LauncherSprings,
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
    bg_tx: mpsc::Sender<BackgroundEvent>,
    /// 后台任务结果接收端喵
    bg_rx: mpsc::Receiver<BackgroundEvent>,
    /// 图标提取器喵(可移入后台线程)喵
    icon_extractor: IconExtractor,
    /// 正在提取图标的名称集合喵(避免重复提交)喵
    pending_icons: HashSet<String>,
}

impl Launcher {
    /// 装配启动器: 初始化状态、异步扫描应用、创建窗口、注册热键喵
    pub fn new(platform: Arc<dyn Platform>, data_dir: PathBuf) -> Self {
        let state = AppState::new(data_dir);

        // 计算初始窗口尺寸与位置(屏幕顶部居中)喵
        let scale = platform.scale_factor();
        let layout = Layout::compute(&state.config, scale, 0, 0.0, 0.0);
        let (screen_w, screen_h) = platform.screen_size();
        let win_w = layout.window_width.ceil() as i32;
        let win_h = layout.window_height.ceil() as i32;
        let x = (screen_w - win_w) / 2;
        let y = (screen_h as f32 * TOP_RATIO) as i32;

        let spec = crate::platform::WindowSpec {
            x,
            y,
            width: win_w,
            height: win_h,
        };
        let window = platform.create_window(&spec).unwrap_or_else(|| {
            log::error!("窗口创建失败,即将退出喵~");
            std::process::exit(1);
        });

        // 注册全局热键喵
        let hotkey = state.config.hotkey.clone();
        if hotkey.enabled {
            let ok = platform.register_global_hotkey(&hotkey.modifiers, &hotkey.key, window);
            if !ok {
                log::warn!("全局热键注册失败喵~");
            }
        } else {
            log::info!("全局热键已禁用喵~");
        }

        let renderer = Renderer::new(win_w, win_h).unwrap_or_else(|| {
            log::error!("渲染器初始化失败,即将退出喵~");
            std::process::exit(1);
        });

        // 异步运行时 + 后台任务通道喵
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("异步运行时初始化失败喵~");
        let (bg_tx, bg_rx) = mpsc::channel();
        let icon_extractor = state.icons.extractor();

        let mut launcher = Self {
            state,
            platform,
            window,
            renderer,
            fonts: FontCache::new(),
            springs: LauncherSprings::new(),
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
        };

        // 首次启动后台异步扫描系统应用喵(不阻塞窗口创建)喵
        if launcher.state.registry.apps.is_empty() {
            log::info!("首次启动,后台异步扫描系统应用喵~");
            launcher.spawn_scan();
        }

        launcher
    }

    /// 进入消息循环喵(阻塞直到退出)喵
    pub fn run(&mut self) {
        log::info!("启动器就绪,等待热键呼出喵~");
        // 取出句柄与平台引用,避免 self 借用冲突喵
        let window = self.window;
        let platform = self.platform.clone();
        platform.run_message_loop(&window, self);
    }

    // ---------------------------------------------------------------------
    // 显示 / 隐藏
    // ---------------------------------------------------------------------

    /// 呼出搜索框喵~
    fn show(&mut self) {
        log::info!("呼出搜索框喵~");
        self.visible = true;
        self.state.reset_search();
        self.scroll_offset = 0.0;
        self.caret_on = true;
        self.caret_acc = Duration::ZERO;
        self.platform.show_window(&self.window, true);
        self.start_animation();
        self.request_render();
    }

    /// 隐藏搜索框喵~
    fn hide(&mut self) {
        log::info!("隐藏搜索框喵~");
        self.visible = false;
        self.state.reset_search();
        self.springs.snap(false, false);
        self.platform.show_window(&self.window, false);
        self.stop_animation();
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
        // 过滤控制字符喵
        if !ch.is_control() {
            self.state.query.push(ch);
            self.refresh_results();
        }
    }

    /// 导航键按下喵
    fn on_key(&mut self, key: Key) {
        match key {
            Key::Enter => self.launch_selected(),
            Key::Escape => self.hide(),
            Key::Backspace => {
                self.state.query.pop();
                self.refresh_results();
            }
            Key::Up => self.move_selection(-1),
            Key::Down => self.move_selection(1),
            _ => {}
        }
    }

    /// 鼠标按下(点击条目启动)喵
    fn on_mouse_down(&mut self, x: f32, y: f32) {
        let scale = self.platform.scale_factor();
        let layout = Layout::compute(
            &self.state.config,
            scale,
            self.state.results.len(),
            self.springs.panel.value,
            self.scroll_offset,
        );
        for (i, rect) in layout.item_rects.iter().enumerate() {
            if rect.left <= x && x <= rect.right && rect.top <= y && y <= rect.bottom {
                self.state.selected = i;
                self.launch_selected();
                return;
            }
        }
    }

    /// 刷新搜索结果并展开/折叠面板喵
    fn refresh_results(&mut self) {
        let count = self.state.refresh_results();
        // 查询变化,滚动回到顶部喵
        self.scroll_offset = 0.0;
        log::debug!("查询更新: {:?}, 结果 {count} 条喵", self.state.query);
        // 为缺图标的条目发起异步提取喵
        self.request_missing_icons();
        self.start_animation();
        self.request_render();
    }

    /// 移动选中(环绕)喵
    fn move_selection(&mut self, delta: isize) {
        let len = self.state.results.len();
        if len == 0 {
            return;
        }
        self.state.selected =
            (self.state.selected as isize + delta).rem_euclid(len as isize) as usize;
        self.ensure_selected_visible();
        log::debug!("选中: {} 喵", self.state.selected);
        self.request_render();
    }

    /// 保证选中项在结果面板可视区内,必要时滚动喵
    fn ensure_selected_visible(&mut self) {
        let len = self.state.results.len();
        if len == 0 {
            self.scroll_offset = 0.0;
            return;
        }
        let selected = self.state.selected as f32;

        // 面板满展开逻辑高度(与 Layout 计算保持一致)喵
        let max_h = self.state.config.window.height.max(80.0);
        let content_h = len as f32 * layout::ITEM_HEIGHT + layout::PANEL_PADDING * 2.0;
        let panel_h = content_h.min(max_h);

        // 选中项的内容坐标范围喵
        let item_top = layout::PANEL_PADDING + selected * layout::ITEM_HEIGHT;
        let item_bottom = item_top + layout::ITEM_HEIGHT;
        // 可视内容区下缘(内容坐标)喵
        let view_bottom = self.scroll_offset + (panel_h - layout::PANEL_PADDING);

        if item_top < self.scroll_offset + layout::PANEL_PADDING {
            // 向上滚: 条目顶部对齐可视区上缘喵
            self.scroll_offset = item_top - layout::PANEL_PADDING;
        } else if item_bottom > view_bottom {
            // 向下滚: 条目底部对齐可视区下缘喵
            self.scroll_offset = item_bottom - (panel_h - layout::PANEL_PADDING);
        }

        // 钳制到合法范围喵
        let max_scroll = (content_h - panel_h).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);
    }

    /// 启动选中的应用并隐藏喵
    fn launch_selected(&mut self) {
        let launched = self.state.launch_selected();
        if launched.is_none() {
            log::debug!("没有可启动的应用喵");
        }
        self.hide();
    }

    // ---------------------------------------------------------------------
    // 后台异步任务
    // ---------------------------------------------------------------------

    /// 后台扫描系统应用喵(不阻塞消息循环)喵
    fn spawn_scan(&mut self) {
        let tx = self.bg_tx.clone();
        self.runtime.spawn_blocking(move || {
            let apps = crate::apps::scanner::scan_installed_apps();
            let _ = tx.send(BackgroundEvent::Scanned(apps));
        });
    }

    /// 为缺图标的条目发起异步图标提取喵
    fn request_missing_icons(&mut self) {
        let extractor = self.icon_extractor.clone();
        // 克隆结果列表,避免迭代期间借用 self.state 喵
        for app in self.state.results.clone() {
            // 已提交提取或已有缓存则跳过喵
            if self.pending_icons.contains(&app.name) {
                continue;
            }
            if self.state.icons.cached_image(&app).is_some() {
                continue;
            }
            self.pending_icons.insert(app.name.clone());
            let tx = self.bg_tx.clone();
            let extractor = extractor.clone();
            self.runtime.spawn_blocking(move || {
                let (name, image) = extractor.extract(app);
                let _ = tx.send(BackgroundEvent::Icon { name, image });
            });
        }
    }

    /// 处理后台任务结果喵(主线程轮询)喵
    fn drain_bg_events(&mut self) {
        while let Ok(event) = self.bg_rx.try_recv() {
            match event {
                BackgroundEvent::Scanned(apps) => {
                    self.state.merge_scanned(apps);
                }
                BackgroundEvent::Icon { name, image } => {
                    self.pending_icons.remove(&name);
                    self.state.icons.cache_image(name, image);
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

    /// 停止定时器(零功耗)喵
    fn stop_animation(&mut self) {
        self.platform.kill_timer(&self.window);
    }

    /// 动画帧推进喵
    fn tick(&mut self) {
        let now = Instant::now();
        let elapsed = self.last_tick.map(|t| now - t).unwrap_or(Duration::ZERO);
        self.last_tick = Some(now);

        // 光标闪烁(半周期)喵
        self.caret_acc += elapsed;
        if self.caret_acc >= CARET_HALF_PERIOD {
            self.caret_acc = Duration::ZERO;
            self.caret_on = !self.caret_on;
        }

        // 推进弹簧喵
        let animating = self.springs.tick(
            params::normalize_dt(elapsed.as_secs_f32()),
            self.visible,
            self.state.results_visible,
        );

        // 处理后台任务结果喵(扫描合并 / 图标回填)喵
        self.drain_bg_events();

        self.request_render();

        // 定时器策略: 动画中 16ms;静止但可见时 500ms(光标闪烁);隐藏则停喵
        if self.visible {
            let interval = if animating {
                ANIM_INTERVAL_MS
            } else {
                CARET_INTERVAL_MS
            };
            self.platform.set_timer(&self.window, interval);
        } else {
            self.stop_animation();
        }
    }

    /// 渲染一帧并呈现喵
    fn request_render(&mut self) {
        let scale = self.platform.scale_factor();
        let layout = Layout::compute(
            &self.state.config,
            scale,
            self.state.results.len(),
            self.springs.panel.value,
            self.scroll_offset,
        );

        // 窗口尺寸随内容(面板展开)变化喵
        let w = layout.window_width.ceil() as i32;
        let h = layout.window_height.ceil() as i32;
        if w != self.renderer.width() || h != self.renderer.height() {
            self.renderer.resize(w, h);
            self.platform.resize_window(&self.window, w, h);
        }

        // 绘制场景喵
        let theme = Theme::for_mode(self.state.config.theme.mode);
        let alpha = self.springs.alpha.value;
        let caret_on = self.caret_on && self.visible && !self.state.query.is_empty();
        {
            let canvas = self.renderer.canvas();
            paint_scene(
                canvas,
                &theme,
                &layout,
                &mut self.state,
                &self.fonts,
                alpha,
                caret_on,
            );
        }

        // 呈现喵
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
            WindowEvent::MouseDown(x, y) => self.on_mouse_down(x, y),
            WindowEvent::LostFocus => {
                if self.visible {
                    self.hide();
                }
            }
            WindowEvent::Timer => self.tick(),
            WindowEvent::Close => {
                log::info!("收到退出请求,关闭喵~");
                self.platform.destroy_window(&self.window);
            }
        }
    }
}
