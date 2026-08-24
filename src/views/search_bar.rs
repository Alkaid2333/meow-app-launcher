//! 灵动岛搜索框窗口喵~
//!
//! 一个长条胶囊形状的搜索框,单独呼出悬浮在主屏幕中上方喵。
//! 与选择窗口(results)分离: 无输入/无匹配时只显示搜索框,
//! 有有效备选时才在下方弹出选择窗口喵。

use crate::animation::{PanelSprings, params};
use crate::app::AppState;
use crate::ui::LauncherTheme;
use crate::views::ResultsView;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// 热键轮询间隔喵
const HOTKEY_POLL_INTERVAL: Duration = Duration::from_millis(80);
/// 动画帧间隔(~60fps)喵
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
/// 搜索框宽度(px)喵
const BAR_WIDTH: f32 = 420.0;
/// 搜索框高度(px)喵
const BAR_HEIGHT: f32 = 44.0;

/// 灵动岛搜索框视图喵
pub struct SearchBarView {
    /// 窗口句柄喵
    window_handle: AnyWindowHandle,
    /// 自己的实体句柄(供异步任务更新)喵
    self_handle: Entity<SearchBarView>,
    /// 是否可见喵
    visible: bool,
    /// 弹簧动画喵
    springs: PanelSprings,
    /// 帧循环是否在跑喵
    frame_loop_active: bool,
    /// 上一帧时间喵
    last_frame: Option<Instant>,
    /// 搜索框状态喵
    input_state: Entity<InputState>,
    /// 订阅句柄(防取消)喵
    _subscriptions: Vec<Subscription>,
    /// 热键触发标志喵
    hotkey_flag: Arc<AtomicBool>,
    // ---------- 选择窗口联动喵 ----------
    /// 选择窗口句柄喵
    results_window: AnyWindowHandle,
    /// 选择窗口视图喵(双窗口创建完成后 link 设置)喵
    results_view: Option<Entity<ResultsView>>,
}

impl SearchBarView {
    pub fn new(
        window_handle: AnyWindowHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // 搜索框状态喵
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("搜索应用喵... (t:标签 / i:首字母)")
        });
        // 订阅输入变化喵
        let _subscriptions = vec![cx.subscribe(&input_state, |this, _, ev, cx| {
            match ev {
                InputEvent::Change => this.on_query_changed(cx),
                _ => {}
            }
        })];

        let mut view = Self {
            window_handle,
            self_handle: cx.entity(),
            visible: false,
            springs: PanelSprings::new(),
            frame_loop_active: false,
            last_frame: None,
            input_state,
            _subscriptions,
            hotkey_flag: Arc::new(AtomicBool::new(false)),
            results_window: window_handle,
            results_view: None,
        };
        view.start_hotkey_poller(cx);
        view.start_auto_test(cx);
        view
    }

    /// 链接选择窗口喵(两个窗口都创建后调用)喵
    pub fn link_results(
        &mut self,
        results_window: AnyWindowHandle,
        results_view: Entity<ResultsView>,
        _cx: &mut Context<Self>,
    ) {
        self.results_window = results_window;
        self.results_view = Some(results_view);
        log::debug!("搜索框 ↔ 选择窗口 联动建立喵");
    }

    /// 窗口句柄(供外部联动)喵
    pub fn window_handle(&self) -> AnyWindowHandle {
        self.window_handle
    }

    /// 绑定全局热键 + 主线程轮询喵
    fn start_hotkey_poller(&mut self, cx: &mut Context<Self>) {
        let app_state = cx.global::<AppState>();
        let cfg = app_state.config.clone();
        let hotkey = cfg.hotkey;
        let platform = app_state.platform.clone();
        let flag = self.hotkey_flag.clone();
        let this = self.self_handle.clone();
        let window_handle = self.window_handle;

        if hotkey.enabled {
            let ok = platform.register_global_hotkey(
                &hotkey.modifiers,
                &hotkey.key,
                Box::new({
                    let flag = flag.clone();
                    move || flag.store(true, Ordering::SeqCst)
                }),
            );
            if !ok {
                log::warn!("全局热键注册失败喵~");
            }
        } else {
            log::info!("全局热键已禁用喵~");
        }

        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor().timer(HOTKEY_POLL_INTERVAL).await;
                if flag.swap(false, Ordering::SeqCst) {
                    log::debug!("收到热键触发喵!");
                    let _ = window_handle.update(cx, |_, window, cx| {
                        let _ = this.update(cx, |view, cx| view.toggle(window, cx));
                    });
                }
            }
        })
        .detach();
    }

    /// 自动冒烟测试喵: `MEOWAL_TEST_TOGGLE=1` 时自动呼出 → 模拟搜索 → 隐藏喵
    ///
    /// 流程: 3 秒后呼出 → 1 秒后输入 "chrome"(选择窗口应显示) →
    /// 2 秒后清空查询(选择窗口应隐藏)→ 1 秒后整体隐藏喵。
    fn start_auto_test(&self, cx: &mut Context<Self>) {
        if std::env::var("MEOWAL_TEST_TOGGLE").map(|v| v == "1").unwrap_or(false) {
            let this = self.self_handle.clone();
            let window_handle = self.window_handle;
            cx.spawn(async move |_, cx| {
                // 呼出喵
                cx.background_executor()
                    .timer(Duration::from_secs(3))
                    .await;
                let _ = window_handle.update(cx, |_, window, cx| {
                    let _ = this.update(cx, |view, cx| view.toggle(window, cx));
                });
                // 模拟输入 "chrome" 喵
                cx.background_executor()
                    .timer(Duration::from_secs(1))
                    .await;
                let _ = window_handle.update(cx, |_, _window, cx| {
                    let _ = this.update(cx, |view, cx| view.query_set("chrome".into(), cx));
                });
                // 等待选择窗口显示喵
                cx.background_executor()
                    .timer(Duration::from_secs(2))
                    .await;
                // 清空查询喵
                let _ = window_handle.update(cx, |_, _window, cx| {
                    let _ = this.update(cx, |view, cx| view.query_set(String::new(), cx));
                });
                // 整体隐藏喵
                cx.background_executor()
                    .timer(Duration::from_secs(1))
                    .await;
                let _ = window_handle.update(cx, |_, window, cx| {
                    let _ = this.update(cx, |view, cx| view.toggle(window, cx));
                });
            })
            .detach();
        }
    }

    // ---------------------------------------------------------------------
    // 显示 / 隐藏
    // ---------------------------------------------------------------------

    /// 切换可见性喵
    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.visible {
            self.hide(window, cx);
        } else {
            self.show(window, cx);
        }
    }

    /// 呼出搜索框喵~
    ///
    /// 窗口显示(置顶+激活)延迟到 spawn 任务里执行: 事件处理期间 App 已被借用,
    /// 直接 ShowWindow(SW_SHOW) 会同步触发窗口激活回调,回调里再 update 窗口
    /// 就会 "RefCell already borrowed"。延迟到 App 空闲时执行就安全喵。
    fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        log::info!("呼出搜索框喵~");
        self.visible = true;

        // 清空输入与搜索状态喵
        self.input_state.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        cx.global_mut::<AppState>().reset_search();

        // 延迟显示 + 激活 + 聚焦搜索框喵
        // 关键: 全局热键线程触发时前台是别的应用,仅 ShowWindow(SW_SHOW) 拿不到
        // 键盘焦点。必须用 window.activate_window()(内部 SetForegroundWindow + 模拟
        // Alt 突破 Windows 前台限制)把窗口真正带到前台,键盘输入才能进 Input 喵。
        let platform = cx.global::<AppState>().platform.clone();
        let wh = self.window_handle;
        let input = self.input_state.clone();
        cx.spawn(async move |_, cx| {
            // 1) 聚焦输入框(只改 GPUI 焦点图,不触发平台事件)喵
            let hwnd = wh
                .update(cx, |_, window, cx| {
                    let _ = input.update(cx, |input, cx| input.focus(window, cx));
                    platform.window_hwnd(window)
                })
                .ok()
                .flatten();
            // 2) 显示并激活窗口喵
            if let Some(h) = hwnd {
                platform.set_visible_hwnd(h, true, true);
            }
            // 3) 把窗口带到前台(激活,确保键盘焦点)喵
            let _ = wh.update(cx, |_, window, _| window.activate_window());
        })
        .detach();

        // 隐藏选择窗口(呼出时不该残留)喵
        self.sync_results_window(cx);

        self.springs.snap(true);
        self.ensure_frame_loop(cx);
        cx.notify();
    }

    /// 隐藏搜索框喵~(窗口隐藏同样延迟,避免 borrow 竞争)喵
    fn hide(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        log::info!("隐藏搜索框喵~");
        self.visible = false;

        // 清空搜索状态,避免选择窗被 sync 重新拉出来喵
        cx.global_mut::<AppState>().reset_search();

        let platform = cx.global::<AppState>().platform.clone();
        let wh = self.window_handle;
        cx.spawn(async move |_, cx| {
            let hwnd = wh
                .update(cx, |_, window, _| platform.window_hwnd(window))
                .ok()
                .flatten();
            if let Some(h) = hwnd {
                platform.set_visible_hwnd(h, false, false);
            }
        })
        .detach();

        // 连带隐藏选择窗口喵
        self.sync_results_window(cx);
        self.ensure_frame_loop(cx);
        cx.notify();
    }

    /// 仅更新自身可见状态喵(由结果窗启动应用后调用;结果窗已由调用方隐藏,
    /// 这里不再 sync 结果窗,避免窗口事件抖动引发 RefCell 竞争)喵
    pub fn mark_hidden(&mut self, cx: &mut Context<Self>) {
        self.visible = false;
        self.springs.snap(false);
        self.ensure_frame_loop(cx);
        cx.notify();
    }

    /// 重新聚焦输入框喵(选择窗被点击后,把键盘焦点还给搜索框,回到可键入状态)喵
    pub fn refocus(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let wh = self.window_handle;
        let input = self.input_state.clone();
        cx.spawn(async move |_, cx| {
            let _ = wh.update(cx, |_, window, cx| {
                window.activate_window();
                let _ = input.update(cx, |input, cx| input.focus(window, cx));
            });
        })
        .detach();
    }

    /// 同步选择窗口的可见性喵(根据 AppState.results_visible)喵
    ///
    /// 用 cx.spawn 延迟执行: 事件回调期间 App 已被借用,直接 update_window
    /// 会触发 "RefCell already borrowed",延迟到下一轮主线程执行就安全喵。
    fn sync_results_window(&self, cx: &mut Context<Self>) {
        let Some(rv) = &self.results_view else {
            return;
        };
        let rw = self.results_window;
        let rv = rv.clone();
        cx.spawn(async move |_, cx| {
            let _ = rw.update(cx, |_, window, cx| {
                let _ = rv.update(cx, |view, cx| view.sync_visibility(window, cx));
            });
        })
        .detach();
    }

    /// 通知选择窗口重绘喵(选中高亮变化)喵
    fn refresh_results_window(&self, cx: &mut Context<Self>) {
        let Some(rv) = &self.results_view else {
            return;
        };
        let rv = rv.clone();
        cx.spawn(async move |_, cx| {
            let _ = rv.update(cx, |view, cx| view.refresh(cx));
        })
        .detach();
    }

    // ---------------------------------------------------------------------
    // 搜索与导航
    // ---------------------------------------------------------------------

    /// 输入变化: 更新查询 + 计算结果 + 同步选择窗口喵
    fn on_query_changed(&mut self, cx: &mut Context<Self>) {
        self.query_set(self.input_state.read(cx).value().to_string(), cx);
    }

    /// 设置查询并更新搜索结果喵
    fn query_set(&mut self, query: String, cx: &mut Context<Self>) {
        let result_count = {
            let app_state = cx.global_mut::<AppState>();
            app_state.query = query;
            app_state.refresh_results();
            app_state.results.len()
        };
        log::debug!("查询更新: {:?},结果 {result_count} 条喵", cx.global::<AppState>().query);
        self.sync_results_window(cx);
        cx.notify();
    }

    /// 移动选中喵(环绕)喵
    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let name = {
            let app_state = cx.global_mut::<AppState>();
            let len = app_state.results.len();
            if len == 0 {
                return;
            }
            app_state.selected =
                (app_state.selected as isize + delta).rem_euclid(len as isize) as usize;
            app_state
                .results
                .get(app_state.selected)
                .map(|r| r.app.name.clone())
                .unwrap_or_default()
        };
        log::debug!("选中: {name} 喵");
        self.refresh_results_window(cx);
        cx.notify();
    }

    /// 启动选中的应用并隐藏两个窗口喵
    fn launch_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let launched_name = cx.global::<AppState>().launch_selected();
        if let Some(name) = launched_name {
            cx.global_mut::<AppState>().record_launch(&name);
        } else {
            log::debug!("没有可启动的应用喵");
        }
        self.hide(window, cx);
    }
}

impl Render for SearchBarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = cx.global::<AppState>();
        let theme = LauncherTheme::for_mode(app_state.config.theme.mode);
        let alpha = self.springs.alpha.value;
        let size_scale = self.springs.scale.value;
        let offset = self.springs.offset.value;

        // 胶囊搜索框喵: 圆角 = 高度一半喵
        div()
            .id("searchbar-root")
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id("searchbar-capsule")
                    .w(px(BAR_WIDTH * size_scale))
                    .h(px(BAR_HEIGHT * size_scale))
                    .rounded(px(BAR_HEIGHT / 2.0))
                    .bg(theme.bg)
                    .border_1()
                    .border_color(theme.border)
                    .shadow_lg()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .px(px(16.0))
                    .opacity(alpha)
                    .mt(px(offset))
                    // 键盘导航喵(Input 单行模式会把方向键/回车冒泡上来喵)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        match event.keystroke.key.as_str() {
                            "down" => this.move_selection(1, cx),
                            "up" => this.move_selection(-1, cx),
                            "enter" => this.launch_selected(window, cx),
                            "escape" => this.hide(window, cx),
                            _ => {}
                        }
                    }))
                    // 搜索图标喵
                    .child(search_icon(&theme))
                    // 搜索输入框喵
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.input_state).appearance(false).h_full()),
                    ),
            )
    }
}

/// 放大镜搜索图标喵(文本字符,避开 transform 限制)喵
fn search_icon(theme: &LauncherTheme) -> impl IntoElement {
    div()
        .id("search-icon")
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(16.0))
        .text_color(theme.text_dim)
        .child("🔍")
}

// 帧循环公共实现,抽到 trait 里避免重复喵
// (SearchBar 与 Results 共用同一套帧循环逻辑喵)
impl SearchBarView {
    /// 启动弹簧帧循环(动画活跃时才跑,停稳即退出喵)
    fn ensure_frame_loop(&mut self, cx: &mut Context<Self>) {
        if self.frame_loop_active {
            return;
        }
        self.frame_loop_active = true;
        self.last_frame = Some(Instant::now());
        let this = self.self_handle.clone();
        cx.spawn(async move |_, cx| {
            loop {
                let still = this
                    .update(cx, |view, cx| {
                        let now = Instant::now();
                        let elapsed = view
                            .last_frame
                            .map(|t| now.duration_since(t).as_secs_f32())
                            .unwrap_or(1.0 / 60.0);
                        view.last_frame = Some(now);
                        let dt = params::normalize_dt(elapsed);
                        let animating = view.springs.tick(dt, view.visible);
                        cx.notify();
                        !animating
                    })
                    .unwrap_or(true);
                if still {
                    break;
                }
                cx.background_executor().timer(FRAME_INTERVAL).await;
            }
            this.update(cx, |view, _| {
                view.frame_loop_active = false;
            })
            .ok();
        })
        .detach();
    }
}
