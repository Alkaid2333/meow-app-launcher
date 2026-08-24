//! 选择窗口喵~
//!
//! 与搜索框分离的结果列表窗口喵: 圆角矩形,与搜索框 x 轴中心对齐,
//! y 轴上与搜索框保持一点距离喵。仅当有有效匹配时才出现喵。

use crate::animation::{PanelSprings, params};
use crate::app::AppState;
use crate::ui::{LauncherTheme, app_item};
use crate::views::SearchBarView;
use gpui::*;
use gpui_component::scroll::ScrollableElement;
use std::time::{Duration, Instant};

/// 动画帧间隔(~60fps)喵
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
/// 选择窗口与搜索框的垂直间距(px)喵
const RESULTS_GAP: f32 = 10.0;

/// 选择窗口视图喵
pub struct ResultsView {
    /// 窗口句柄喵
    window_handle: AnyWindowHandle,
    /// 自己的实体句柄(供异步任务更新)喵
    self_handle: Entity<ResultsView>,
    /// 是否可见喵
    visible: bool,
    /// 弹簧动画喵
    springs: PanelSprings,
    /// 帧循环是否在跑喵
    frame_loop_active: bool,
    /// 上一帧时间喵
    last_frame: Option<Instant>,
    // ---------- 搜索框联动喵 ----------
    /// 搜索框窗口句柄(点击启动后连带隐藏)喵
    searchbar_window: AnyWindowHandle,
    /// 搜索框视图喵
    searchbar_view: Option<Entity<SearchBarView>>,
}

impl ResultsView {
    pub fn new(
        window_handle: AnyWindowHandle,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            window_handle,
            self_handle: cx.entity(),
            visible: false,
            springs: PanelSprings::new(),
            frame_loop_active: false,
            last_frame: None,
            searchbar_window: window_handle,
            searchbar_view: None,
        }
    }

    /// 链接搜索框窗口喵(两个窗口都创建后调用)喵
    pub fn link_searchbar(
        &mut self,
        searchbar_window: AnyWindowHandle,
        searchbar_view: Entity<SearchBarView>,
        _cx: &mut Context<Self>,
    ) {
        self.searchbar_window = searchbar_window;
        self.searchbar_view = Some(searchbar_view);
        log::debug!("选择窗口 ↔ 搜索框 联动建立喵");
    }

    /// 窗口句柄(供外部联动)喵
    pub fn window_handle(&self) -> AnyWindowHandle {
        self.window_handle
    }

    /// 同步可见性(由搜索框输入变化时调用)喵
    ///
    /// 定位与显示都延迟到 spawn 任务里执行,避免窗口事件回调
    /// 与我们的处理竞争 App 借用喵。
    pub fn sync_visibility(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let should_show = cx.global::<AppState>().results_visible;

        if should_show != self.visible {
            self.visible = should_show;

            // 延迟: 提取 hwnd 后(App 借用释放)再定位+显示/隐藏,避免窗口事件
            // 回调在 update 期间嵌套借用喵。
            let platform = cx.global::<AppState>().platform.clone();
            let wh = self.window_handle;
            let bounds = cx.global::<AppState>().searchbar_bounds;
            let rw = cx.global::<AppState>().config.window.width;
            cx.spawn(async move |_, cx| {
                let hwnd = wh
                    .update(cx, |_, window, _| platform.window_hwnd(window))
                    .ok()
                    .flatten();
                if let Some(h) = hwnd {
                    if should_show {
                        if let Some((sx, sy, sw, sh)) = bounds {
                            let x = sx + (sw - rw) / 2.0;
                            let y = sy + sh + RESULTS_GAP;
                            platform.move_window_hwnd(h, x, y);
                        }
                    }
                    // 选择窗显示不激活(activate=false),避免抢走搜索框的键盘焦点喵
                    platform.set_visible_hwnd(h, should_show, false);
                }
            })
            .detach();

            if should_show {
                log::debug!("选择窗口显示喵");
                self.springs.snap(true);
            } else {
                log::debug!("选择窗口隐藏喵");
                self.springs.snap(false);
            }
            self.ensure_frame_loop(cx);
            cx.notify();
        }
    }

    /// 重绘(选中高亮变化)喵
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// 把键盘焦点还给搜索框喵(点击选择窗后,回到可键入状态)喵
    fn refocus_searchbar(&self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(sv) = &self.searchbar_view else {
            return;
        };
        let sw = self.searchbar_window;
        let sv = sv.clone();
        cx.spawn(async move |_, cx| {
            let _ = sw.update(cx, |_, window, cx| {
                let _ = sv.update(cx, |view, cx| view.refocus(window, cx));
            });
        })
        .detach();
    }

    /// 点击选择窗口中的某项: 选中并启动应用,然后隐藏两个窗口喵
    fn click_launch(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        let launched_name = {
            let app_state = cx.global_mut::<AppState>();
            app_state.selected = idx;
            app_state
                .results
                .get(idx)
                .and_then(|r| {
                    let ok = app_state.platform.launch(&r.app.path);
                    if ok {
                        log::info!("启动应用: {} ({}) 喵", r.app.name, r.app.path);
                        Some(r.app.name.clone())
                    } else {
                        None
                    }
                })
        };
        if let Some(name) = launched_name {
            cx.global_mut::<AppState>().record_launch(&name);
        }
        self.hide_all(window, cx);
    }

    /// 隐藏选择窗口 + 连带隐藏搜索框喵
    ///
    /// 关键: 先清空搜索状态(否则 hide 链上的 sync_visibility 读到 results_visible=true,
    /// 会把选择窗重新拉出来,造成窗口"隐藏→显示"抖动,进而引发窗口事件交错 → RefCell 刷屏)喵。
    /// 两个窗口的隐藏合并到单个 spawn,减少并发窗口操作喵。
    fn hide_all(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 清空搜索状态喵
        cx.global_mut::<AppState>().reset_search();

        self.visible = false;
        self.springs.snap(false);
        self.ensure_frame_loop(cx);
        cx.notify();

        // 延迟隐藏结果窗 + 搜索框(合并到一个 spawn)喵
        let platform = cx.global::<AppState>().platform.clone();
        let rwh = self.window_handle;
        let swh = self.searchbar_window;
        let sv = self.searchbar_view.clone();
        cx.spawn(async move |_, cx| {
            // 隐藏结果窗喵
            if let Some(h) = rwh
                .update(cx, |_, window, _| platform.window_hwnd(window))
                .ok()
                .flatten()
            {
                platform.set_visible_hwnd(h, false, false);
            }
            // 隐藏搜索框窗口喵
            if let Some(h) = swh
                .update(cx, |_, window, _| platform.window_hwnd(window))
                .ok()
                .flatten()
            {
                platform.set_visible_hwnd(h, false, false);
            }
            // 更新搜索框视图状态(仅状态,不再 sync 结果窗)喵
            if let Some(sv) = sv {
                let _ = swh.update(cx, |_, _window, cx| {
                    let _ = sv.update(cx, |view, cx| view.mark_hidden(cx));
                });
            }
        })
        .detach();
    }
}

impl Render for ResultsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_state = cx.global::<AppState>();
        let config = &app_state.config;
        let theme = LauncherTheme::for_mode(config.theme.mode);
        let panel_w = config.window.width;
        let panel_h = config.window.height;
        let icon_size = config.window.icon_size;
        let results = &app_state.results;
        let selected = app_state.selected;
        let alpha = self.springs.alpha.value;
        let size_scale = self.springs.scale.value;
        let offset = self.springs.offset.value;

        // 结果列表喵(每项可点击)喵
        let items: Vec<AnyElement> = results
            .iter()
            .enumerate()
            .map(|(idx, result)| {
                let item = app_item::app_item(
                    &result.app,
                    result.icon.clone(),
                    idx == selected,
                    &theme,
                    icon_size,
                );
                div()
                    .id(SharedString::from(format!("result-{idx}")))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.click_launch(idx, window, cx);
                    }))
                    .child(item)
                    .into_any_element()
            })
            .collect();

        div()
            .id("results-root")
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            // 点击选择窗任意位置(含空白)都把键盘焦点还给搜索框,回到可键入状态喵
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _: &MouseDownEvent, window, cx| {
                this.refocus_searchbar(window, cx);
            }))
            .child(
                div()
                    .id("results-panel")
                    .w(px(panel_w * size_scale))
                    .h(px(panel_h * size_scale))
                    .rounded(px(12.0))
                    .bg(theme.bg)
                    .border_1()
                    .border_color(theme.border)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .py(px(8.0))
                    .px(px(8.0))
                    .gap(px(2.0))
                    .opacity(alpha)
                    .mt(px(offset))
                    .child(
                        div()
                            .flex_1()
                            .overflow_y_scrollbar()
                            .children(items),
                    ),
            )
    }
}

impl ResultsView {
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
