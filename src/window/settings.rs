//! 配置窗口喵~
//!
//! 依据 window-design skill 的配置 GUI 设计,实现「侧边栏 + 分组卡片」设置界面喵。
//! 持有共享状态,修改配置项并持久化;交互通过命中测试驱动喵。
//!
//! 可调配置项由 [`settings_data`] 的数据驱动注册表统一描述:
//! 页面构建、命中分发、脏判定、恢复默认都查同一张表,新增配置项只改一处喵。
//!
//! 状态机: 常驻(隐藏) → 收到 `settings_visible` 标志 → 显示并渲染 → 红点关闭喵。
//! 动效: 滚动平滑趋近 + 页面切换淡入,可在「常规 · 显示」里整体关掉喵。

use crate::app::config::AppConfig;
use crate::app::{Command, SharedState};use crate::platform::{PlatformWindow, Win32Platform, WindowEvent, WindowHandler};
use crate::render::edit::{caret_from_x, TextEdit};
use crate::render::font::FontCache;
use crate::render::settings::{
    paint_settings, EditFocus, RowHit, RowId, SETTINGS_HEIGHT, SETTINGS_WIDTH, SettingsGroup,
    SettingsLayout, SettingsPage, SettingsRow,
};
use crate::render::{Renderer, SettingsTheme};
use crate::window::settings_data::{self, RowKind, RowValue, PAGE_GENERAL};
use skia_safe::Rect;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

/// 心跳间隔(ms): 隐藏时低频检查显示请求喵
const HEARTBEAT_MS: u32 = 100;
/// 动效帧间隔(ms): 滚动平滑/页面淡入进行中用动画帧率喵
const ANIM_FRAME_MS: u32 = 16;
/// 滚动平滑的趋近系数喵(每帧向目标靠拢的比例)喵
const SCROLL_EASE: f32 = 0.28;
/// 页面淡入的每帧增量喵
const FADE_STEP: f32 = 0.09;

/// 正在被鼠标拖选文本的输入框喵
#[derive(Debug, Clone, Copy)]
struct TextDrag {
    /// 拖选目标的输入框矩形(逻辑坐标,随滚动绘制)喵
    rect: Rect,
}

/// 滚动条拖动快照喵
struct ScrollDrag {
    /// 拖拽起点(客户区逻辑 y)喵
    anchor_y: f32,
    /// 起点进度(0..1)喵
    anchor_prog: f32,
    /// 滑块可移动距离(逻辑 px)喵
    travel: f32,
    /// 最大滚动量(逻辑 px)喵
    max_logical: f32,
}

/// 配置窗口喵
pub struct SettingsWindow {
    /// 共享应用状态喵
    state: SharedState,
    /// 平台句柄喵
    platform: Arc<Win32Platform>,
    /// 窗口句柄喵
    window: PlatformWindow,
    /// 应用命令队列喵(重新扫描等)喵
    commands: Rc<RefCell<VecDeque<Command>>>,
    /// Skia 渲染器喵
    renderer: Renderer,
    /// 字体缓存喵
    fonts: FontCache,
    /// 当前页面索引喵
    current_page: usize,
    /// 当前页面模型喵(交互时读取控件范围)喵
    pages: Vec<SettingsPage>,
    /// 是否可见喵
    visible: bool,
    /// 内容区滚动偏移(逻辑像素)喵
    scroll: f32,
    /// 滚动目标(平滑动效趋近它)喵
    scroll_target: f32,
    /// 页面淡入透明度(1 = 不透明)喵
    page_fade: f32,
    /// 最近一次布局结果(供命中测试)喵
    layout: Option<SettingsLayout>,
    /// DPI 缩放系数喵
    scale: f32,
    /// BGRA 像素缓冲喵
    pixels: Vec<u8>,
    /// 正在手动键入的数值行(id + 文本缓冲)喵
    stepper_edit: Option<(RowId, TextEdit)>,
    /// 正在编辑的过滤关键词行(规则下标 + 文本缓冲)喵
    filter_edit: Option<(usize, TextEdit)>,
    /// 正在编辑的指令别名行(下标 + 文本缓冲)喵
    command_edit: Option<(usize, TextEdit)>,
    /// 正在编辑的自定义浏览器输入(文本缓冲)喵
    browser_edit: Option<TextEdit>,
    /// 正在拖选文本的输入框喵
    text_drag: Option<TextDrag>,
    /// 正在拖动的滚动条快照喵
    scroll_drag: Option<ScrollDrag>,
    /// 是否正在录制全局热键喵
    hotkey_capture: bool,
    /// 窗口当前逻辑尺寸(可拖拽缩放)喵
    win_w: f32,
    /// 窗口当前逻辑高度喵
    win_h: f32,
    /// 窗口屏幕位置(物理像素)喵
    win_pos: Option<(i32, i32)>,
    /// 窗口拖拽/缩放状态喵
    win_drag: Option<WinDrag>,
    /// 当前生效的定时器间隔(ms,仅变化时重设)喵
    timer_interval_ms: u32,
}

/// 窗口拖动/缩放快照喵
struct WinDrag {
    /// true = 缩放,false = 移动喵
    resize: bool,
    /// 拖拽起点(屏幕坐标)喵
    anchor_screen: (i32, i32),
    /// 起点窗口位置喵
    anchor_pos: (i32, i32),
    /// 起点窗口尺寸(物理)喵
    anchor_size: (i32, i32),
}

impl SettingsWindow {
    /// 装配配置窗口喵: 创建窗口(初始隐藏)、渲染器、启动心跳喵
    ///
    /// 返回配置窗口句柄,供启动器打开设置时使用喵。
    pub fn spawn(
        platform: Arc<Win32Platform>,
        state: SharedState,
        commands: Rc<RefCell<VecDeque<Command>>>,
    ) -> PlatformWindow {
        let scale = platform.scale_factor();
        let win_w = (SETTINGS_WIDTH * scale).ceil() as i32;
        let win_h = (SETTINGS_HEIGHT * scale).ceil() as i32;
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
            log::error!("配置窗口创建失败,即将退出喵~");
            std::process::exit(1);
        });

        let backend = state.borrow().config.render_backend;
        let renderer = Renderer::new(win_w, win_h, backend, &platform).unwrap_or_else(|| {
            log::error!("配置窗口渲染器初始化失败,即将退出喵~");
            std::process::exit(1);
        });

        let settings = SettingsWindow {
            state,
            platform: platform.clone(),
            window,
            commands,
            renderer,
            fonts: FontCache::new(),
            current_page: 0,
            pages: Vec::new(),
            visible: false,
            scroll: 0.0,
            scroll_target: 0.0,
            page_fade: 1.0,
            layout: None,
            scale,
            pixels: Vec::new(),
            stepper_edit: None,
            filter_edit: None,
            command_edit: None,
            browser_edit: None,
            text_drag: None,
            scroll_drag: None,
            hotkey_capture: false,
            win_w: SETTINGS_WIDTH,
            win_h: SETTINGS_HEIGHT,
            win_pos: Some((x, y)),
            win_drag: None,
            timer_interval_ms: HEARTBEAT_MS,
        };

        platform.set_window_handler(&window, Box::new(settings));
        platform.enable_file_drop(&window);
        platform.set_timer(&window, HEARTBEAT_MS);

        window
    }

    // ---------------------------------------------------------------------
    // 显示 / 隐藏
    // ---------------------------------------------------------------------

    /// 显示配置窗口喵
    fn show(&mut self) {
        log::info!("显示配置窗口喵~");
        self.visible = true;
        self.scroll = 0.0;
        self.scroll_target = 0.0;
        self.page_fade = if self.anim_enabled() { 0.0 } else { 1.0 };
        self.platform.show_window(&self.window, true);
        self.render();
    }

    /// 隐藏配置窗口喵(先提交挂起的编辑,避免丢改动)喵
    fn hide(&mut self) {
        log::info!("隐藏配置窗口喵~");
        self.commit_active_edit();
        self.visible = false;
        self.state.borrow_mut().settings_visible = false;
        self.platform.show_window(&self.window, false);
    }

    // ---------------------------------------------------------------------
    // 动效
    // ---------------------------------------------------------------------

    /// 面板动效是否开启喵
    fn anim_enabled(&self) -> bool {
        self.state.borrow().config.settings_anim
    }

    /// 推进面板动效(滚动平滑 + 页面淡入),返回是否仍在动喵
    fn step_anim(&mut self) -> bool {
        if !self.anim_enabled() {
            self.page_fade = 1.0;
            self.scroll = self.scroll_target;
            return false;
        }
        let mut animating = false;
        // 滚动平滑: 每帧向目标靠拢一截,越近越慢喵
        let delta = self.scroll_target - self.scroll;
        if delta.abs() > 0.5 {
            self.scroll += delta * SCROLL_EASE;
            animating = true;
        } else {
            self.scroll = self.scroll_target;
        }
        // 页面淡入喵
        if self.page_fade < 1.0 {
            self.page_fade = (self.page_fade + FADE_STEP).min(1.0);
            animating = true;
        }
        animating
    }

    /// 设置定时器间隔,仅在实际变化时才重设喵(避免每帧重置导致节奏抖动)喵
    fn set_timer_interval(&mut self, ms: u32) {
        if self.timer_interval_ms != ms {
            self.timer_interval_ms = ms;
            self.platform.set_timer(&self.window, ms);
        }
    }

    /// 当前内容的最大滚动量喵
    fn max_scroll(&self) -> f32 {
        self.layout
            .as_ref()
            .map(|l| (l.content_height - self.win_h).max(0.0))
            .unwrap_or(0.0)
    }

    // ---------------------------------------------------------------------
    // 渲染
    // ---------------------------------------------------------------------

    /// 重新生成页面并渲染喵
    fn render(&mut self) {
        // 从当前配置重建页面喵
        let pages = {
            let state = self.state.borrow();
            build_pages(&state.config, self.hotkey_capture)
        };
        let theme = {
            let state = self.state.borrow();
            SettingsTheme::for_preset(state.config.theme.preset)
        };

        let canvas = self.renderer.canvas();
        canvas.save();
        canvas.scale((self.scale, self.scale));
        // 偏离默认值的行: 行首显示「恢复默认」图标喵
        let dirty = {
            let state = self.state.borrow();
            settings_data::dirty_data_ids(&state.config)
        };
        // 汇总当前编辑焦点(同一时刻只有一个输入框在编辑)喵
        let edit = EditFocus {
            stepper: self.stepper_edit.as_ref().map(|(id, te)| (*id, te)),
            filter: self.filter_edit.as_ref().map(|(i, te)| (*i, te)),
            command: self.command_edit.as_ref().map(|(i, te)| (*i, te)),
            browser: self.browser_edit.as_ref(),
        };
        let layout = paint_settings(
            canvas,
            &theme,
            &self.fonts,
            &pages,
            self.current_page,
            self.scroll,
            self.page_fade,
            edit,
            self.hotkey_capture,
            &dirty,
            self.win_w,
            self.win_h,
        );
        canvas.restore();
        self.layout = Some(layout);
        self.pages = pages;

        self.renderer.read_bgra(&mut self.pixels);
        self.platform.present(
            &self.window,
            self.renderer.width(),
            self.renderer.height(),
            &self.pixels,
        );
    }

    /// 按配置重建渲染器喵(切换 CPU/GPU 时调用,失败保留原渲染器)喵
    fn recreate_renderer(&mut self) {
        let backend = self.state.borrow().config.render_backend;
        let (w, h) = (self.renderer.width(), self.renderer.height());
        if let Some(renderer) = Renderer::new(w, h, backend, &self.platform) {
            log::info!("配置窗口渲染器已重建 → {} 喵", renderer.mode().label());
            self.renderer = renderer;
        }
    }

    // ---------------------------------------------------------------------
    // 交互
    // ---------------------------------------------------------------------

    /// 命中测试并处理点击喵
    fn handle_click(&mut self, x: f32, y: f32) {
        // 物理坐标 → 逻辑坐标喵
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

        // 与管理中心统一: 点在文本框外,挂起的草稿立即提交,焦点随点击离开喵
        let on_input = matches!(
            &hit,
            Some((_, RowHit::StepperEdit(_)))
                | Some((_, RowHit::FilterInput(_)))
                | Some((_, RowHit::CommandAlias(_)))
                | Some((_, RowHit::Input(_)))
        );
        if !on_input {
            self.commit_active_edit();
        }

        if let Some((rect, hit)) = hit {
            match hit {
                RowHit::TitleBar => {
                    self.begin_window_drag(x, y, false);
                    self.render();
                }
                RowHit::ResizeGrip => {
                    self.begin_window_drag(x, y, true);
                    self.render();
                }
                // 文本框: 需要矩形做光标命中与拖选,提前拦截喵
                RowHit::StepperEdit(id) => self.click_stepper_box(rect, id, lx),
                RowHit::FilterInput(i) => self.click_filter_box(rect, i, lx),
                RowHit::CommandAlias(i) => self.click_command_box(rect, i, lx),
                RowHit::Input(RowId::WebBrowser) => self.click_browser_box(rect, lx),
                RowHit::ScrollThumb => self.begin_scroll_drag(ly),
                _ => self.apply_hit(hit),
            }
        }
    }

    /// 输入框通用取色画笔喵
    fn box_paint(&self) -> skia_safe::Paint {
        let mut p = skia_safe::Paint::default();
        p.set_color(skia_safe::Color::BLACK);
        p
    }

    /// 点击数值框: 未聚焦 → 全选进入编辑;已聚焦 → 光标定位并开始拖选喵
    fn click_stepper_box(&mut self, box_rect: Rect, id: RowId, lx: f32) {
        let paint = self.box_paint();
        let cur = self
            .state
            .borrow()
            .config
            .clone();
        let cur = match settings_data::data_value(&cur, id) {
            Some(RowValue::Num(n)) => n,
            _ => 0,
        };
        let font = self.fonts.font(12.0);
        // 字段借用与自方法调用分离,避免同一作用域双借用喵
        let hit_editing_same = matches!(&self.stepper_edit, Some((eid, _)) if *eid == id);
        if hit_editing_same {
            if let Some((_, te)) = self.stepper_edit.as_mut() {
                let caret = caret_from_x(&font, &paint, &te.text, box_rect.left + 8.0, lx);
                te.begin_select(caret);
            }
        } else {
            let mut te = TextEdit::new(cur.to_string());
            te.select_all();
            self.stepper_edit = Some((id, te));
        }
        self.filter_edit = None;
        self.command_edit = None;
        self.browser_edit = None;
        self.text_drag = Some(TextDrag { rect: box_rect });
        log::debug!("数值框聚焦: {id:?} 喵");
        self.render();
    }

    /// 点击过滤关键词输入框喵
    fn click_filter_box(&mut self, box_rect: Rect, index: usize, lx: f32) {
        let paint = self.box_paint();
        let font = self.fonts.font(12.0);
        let keyword = self
            .state
            .borrow()
            .config
            .search
            .filters
            .get(index)
            .map(|f| f.keyword.clone())
            .unwrap_or_default();
        if matches!(&self.filter_edit, Some((i, _)) if *i == index) {
            if let Some((_, te)) = self.filter_edit.as_mut() {
                let caret = caret_from_x(&font, &paint, &te.text, box_rect.left + 8.0, lx);
                te.begin_select(caret);
            }
        } else {
            let mut te = TextEdit::new(keyword);
            te.select_all();
            self.filter_edit = Some((index, te));
        }
        self.stepper_edit = None;
        self.command_edit = None;
        self.browser_edit = None;
        self.text_drag = Some(TextDrag { rect: box_rect });
        log::debug!("过滤关键词框聚焦: 规则 {index} 喵");
        self.render();
    }

    /// 点击指令别名输入框喵
    fn click_command_box(&mut self, box_rect: Rect, index: usize, lx: f32) {
        let paint = self.box_paint();
        let font = self.fonts.font(12.0);
        let aliases = self
            .state
            .borrow()
            .config
            .search
            .commands
            .get(index)
            .map(|c| c.aliases.join(", "))
            .unwrap_or_default();
        if matches!(&self.command_edit, Some((i, _)) if *i == index) {
            if let Some((_, te)) = self.command_edit.as_mut() {
                let caret = caret_from_x(&font, &paint, &te.text, box_rect.left + 8.0, lx);
                te.begin_select(caret);
            }
        } else {
            let mut te = TextEdit::new(aliases);
            te.select_all();
            self.command_edit = Some((index, te));
        }
        self.stepper_edit = None;
        self.filter_edit = None;
        self.browser_edit = None;
        self.text_drag = Some(TextDrag { rect: box_rect });
        log::debug!("指令别名框聚焦: 指令 {index} 喵");
        self.render();
    }

    /// 点击自定义浏览器输入框喵
    fn click_browser_box(&mut self, box_rect: Rect, lx: f32) {
        let paint = self.box_paint();
        let font = self.fonts.font(12.0);
        let current = self.state.borrow().config.search.web_browser.clone();
        if self.browser_edit.is_some() {
            if let Some(te) = self.browser_edit.as_mut() {
                let caret = caret_from_x(&font, &paint, &te.text, box_rect.left + 8.0, lx);
                te.begin_select(caret);
            }
        } else {
            let mut te = TextEdit::new(current);
            te.select_all();
            self.browser_edit = Some(te);
        }
        self.stepper_edit = None;
        self.filter_edit = None;
        self.command_edit = None;
        self.text_drag = Some(TextDrag { rect: box_rect });
        log::debug!("自定义浏览器框聚焦喵");
        self.render();
    }

    /// 开始拖动滚动条喵: 锚定起点进度与滑块行程喵
    fn begin_scroll_drag(&mut self, y: f32) {
        let max = self.max_scroll();
        if max <= 0.0 {
            return;
        }
        // 布局里的轨道与滑块(窗口逻辑坐标)喵
        let Some(layout) = self.layout.as_ref() else {
            return;
        };
        let Some((track, thumb)) = layout.scrollbar else {
            return;
        };
        let travel = (track.height() - thumb.height()).max(1.0);
        let prog = (self.scroll / max).clamp(0.0, 1.0);
        self.scroll_drag = Some(ScrollDrag {
            anchor_y: y,
            anchor_prog: prog,
            travel,
            max_logical: max,
        });
        // 拖动期间直接跟手,不走平滑动效喵
        self.scroll_target = self.scroll;
        log::debug!("开始拖动滚动条喵");
    }

    /// 开始窗口拖拽/缩放喵: 锚定屏幕坐标与窗口位置/尺寸喵
    fn begin_window_drag(&mut self, x: f32, y: f32, resize: bool) {
        let (wx, wy) = self.win_pos.unwrap_or((0, 0));
        let (w, h) = (self.renderer.width(), self.renderer.height());
        self.win_drag = Some(WinDrag {
            resize,
            anchor_screen: (wx + x as i32, wy + y as i32),
            anchor_pos: (wx, wy),
            anchor_size: (w, h),
        });
        log::debug!("窗口{}开始喵", if resize { "缩放" } else { "拖动" });
    }

    /// 拖动窗口 / 调整窗口尺寸 / 拖滚动条 / 扩展拖选喵
    fn on_mouse_move(&mut self, x: f32, y: f32) {
        let ly = y / self.scale;
        // 文本拖选: 跟随指针扩展选中区(反向也可选)喵
        if let Some(drag) = self.text_drag {
            let lx = x / self.scale;
            let paint = self.box_paint();
            if let Some((_, te)) = &mut self.stepper_edit {
                let caret = caret_from_x(
                    &self.fonts.font(12.0),
                    &paint,
                    &te.text,
                    drag.rect.left + 8.0,
                    lx,
                );
                te.extend_select(caret);
                self.render();
            } else if let Some((_, te)) = &mut self.filter_edit {
                let caret = caret_from_x(
                    &self.fonts.font(12.0),
                    &paint,
                    &te.text,
                    drag.rect.left + 8.0,
                    lx,
                );
                te.extend_select(caret);
                self.render();
            } else if let Some(te) = &mut self.browser_edit {
                let caret = caret_from_x(
                    &self.fonts.font(12.0),
                    &paint,
                    &te.text,
                    drag.rect.left + 8.0,
                    lx,
                );
                te.extend_select(caret);
                self.render();
            }
            return;
        }

        // 拖动滚动条: 滑块行程线性映射内容滚动量喵
        if let Some(d) = &self.scroll_drag {
            let dy = ly - d.anchor_y;
            let prog = (d.anchor_prog + dy / d.travel).clamp(0.0, 1.0);
            self.scroll = prog * d.max_logical;
            self.scroll_target = self.scroll;
            self.render();
            return;
        }

        let Some(ref d) = self.win_drag else {
            return;
        };
        let (wx, wy) = self.win_pos.unwrap_or((0, 0));
        let (dx, dy) = (wx + x as i32 - d.anchor_screen.0, wy + y as i32 - d.anchor_screen.1);
        if d.resize {
            // 缩放: 钳制最小尺寸,不超出屏幕喵
            let min_w = (560.0 * self.scale) as i32;
            let min_h = (500.0 * self.scale) as i32;
            let (sw, sh) = self.platform.screen_size();
            let nw = (d.anchor_size.0 + dx).clamp(min_w, sw.max(min_w));
            let nh = (d.anchor_size.1 + dy).clamp(min_h, sh.max(min_h));
            self.renderer.resize(nw, nh);
            self.platform.resize_window(&self.window, nw, nh);
            self.win_w = nw as f32 / self.scale;
            self.win_h = nh as f32 / self.scale;
            self.render();
        } else {
            let nx = d.anchor_pos.0 + dx;
            let ny = d.anchor_pos.1 + dy;
            self.platform.move_window(&self.window, nx, ny);
            self.win_pos = Some((nx, ny));
        }
    }

    /// 处理命中动作喵(文本框/滚动条类命中已在 handle_click 拦截)喵
    fn apply_hit(&mut self, hit: RowHit) {
        match hit {
            RowHit::Nav(i) => {
                self.current_page = i;
                self.scroll = 0.0;
                self.scroll_target = 0.0;
                self.page_fade = if self.anim_enabled() { 0.0 } else { 1.0 };
                self.clear_edits();
            }
            RowHit::Close => self.hide(),
            RowHit::Switch(id) => self.toggle_data_switch(id),
            RowHit::StepperDec(id) => self.adjust_data_stepper(id, -1),
            RowHit::StepperInc(id) => self.adjust_data_stepper(id, 1),
            RowHit::Choice(id) => self.cycle_data_choice(id),
            RowHit::Button(id) => self.press_button(id),
            RowHit::Restore(id) => self.restore_row_defaults(id),
            RowHit::FilterCase(i) => self.toggle_filter_case(i),
            RowHit::FilterDelete(i) => self.delete_filter(i),
            RowHit::CommandKind(i) => self.cycle_command_kind(i),
            RowHit::CommandDelete(i) => self.delete_command(i),
            // 窗口栏/手柄与文本框在 handle_click 里先行拦截,这里仅收尾喵
            RowHit::TitleBar
            | RowHit::ResizeGrip
            | RowHit::StepperEdit(_)
            | RowHit::FilterInput(_)
            | RowHit::CommandAlias(_)
            | RowHit::Input(_)
            | RowHit::ScrollThumb => {}
        }
        self.render();
    }

    /// 清空全部编辑态喵
    fn clear_edits(&mut self) {
        self.stepper_edit = None;
        self.filter_edit = None;
        self.command_edit = None;
        self.browser_edit = None;
        self.text_drag = None;
        self.hotkey_capture = false;
    }

    // ---------------------------------------------------------------------
    // 数据驱动行的统一交互喵(开关/步进/循环选项)
    // ---------------------------------------------------------------------

    /// 应用一个配置值喵(带持久化与跨组件副作用)喵
    fn apply_data(&mut self, id: RowId, value: RowValue) {
        let mut state = self.state.borrow_mut();
        if !settings_data::apply_data_value(&mut state.config, id, value) {
            return;
        }
        state.persist();
        log::debug!("配置 {id:?} 已更新喵");
        drop(state);
        // 需要同步到系统/其它窗口的项单独处理喵
        match id {
            RowId::AutoStart => {
                let on = self.state.borrow().config.auto_start;
                if !self.platform.set_auto_start(on) {
                    // 写注册表失败即回滚,保证配置与系统状态一致喵
                    self.state.borrow_mut().config.auto_start = !on;
                    self.state.borrow_mut().persist();
                    log::warn!("开机自启设置失败,已回滚喵~");
                } else {
                    log::info!("开机自启 → {} 喵", on);
                }
            }
            RowId::RenderBackend => {
                self.recreate_renderer();
                self.commands.borrow_mut().push_back(Command::RecreateRenderer);
            }
            _ => {}
        }
    }

    /// 切换开关行喵
    fn toggle_data_switch(&mut self, id: RowId) {
        let cur = settings_data::data_value(&self.state.borrow().config, id);
        if let Some(RowValue::Bool(b)) = cur {
            self.apply_data(id, RowValue::Bool(!b));
        }
    }

    /// 步进调节喵: 按行配置的步幅增减喵
    fn adjust_data_stepper(&mut self, id: RowId, dir: i32) {
        // 若正在手动键入同一行,先退出键入态,避免草稿与最新值错位喵
        if self.stepper_edit.as_ref().is_some_and(|(eid, _)| *eid == id) {
            self.stepper_edit = None;
        }
        let Some(meta) = settings_data::find_row(id).and_then(|d| d.stepper_meta()) else {
            return;
        };
        let (_, _, step, _) = meta;
        let cur = match settings_data::data_value(&self.state.borrow().config, id) {
            Some(RowValue::Num(n)) => n,
            _ => return,
        };
        self.apply_data(id, RowValue::Num(cur + dir * step));
    }

    /// 循环选项喵: 点击切到下一档喵
    fn cycle_data_choice(&mut self, id: RowId) {
        let desc = match settings_data::find_row(id) {
            Some(d) => d,
            None => return,
        };
        let (options, cur) = match (desc.kind, settings_data::data_value(&self.state.borrow().config, id)) {
            (RowKind::Choice { options }, Some(RowValue::Index(i))) => (options, i),
            _ => return,
        };
        let next = (cur + 1) % options.len().max(1);
        self.apply_data(id, RowValue::Index(next));
        log::info!("{} → {} 喵", desc.label, options.get(next).unwrap_or(&""));
    }

    /// 恢复单个配置项到默认值喵
    fn restore_row_defaults(&mut self, id: RowId) {
        // 正在键入同一行时先退出编辑态,避免草稿过期喵
        if self.stepper_edit.as_ref().is_some_and(|(eid, _)| *eid == id) {
            self.stepper_edit = None;
        }
        let mut state = self.state.borrow_mut();
        if settings_data::restore_data_row(&mut state.config, id) {
            state.persist();
            log::info!("已恢复 {id:?} 到默认值喵~");
            drop(state);
            // 副作用同步喵
            match id {
                RowId::HotkeyEnabled => self.commands.borrow_mut().push_back(Command::ReapplyHotkey),
                RowId::AutoStart => {
                    let on = self.state.borrow().config.auto_start;
                    self.platform.set_auto_start(on);
                }
                RowId::RenderBackend => {
                    self.recreate_renderer();
                    self.commands.borrow_mut().push_back(Command::RecreateRenderer);
                }
                _ => {}
            }
        }
    }

    // ---------------------------------------------------------------------
    // 过滤 / 指令 / 标签 / 浏览器编辑喵
    // ---------------------------------------------------------------------

    /// 提交过滤关键词缓冲喵(空关键词保留但不起过滤作用)喵
    fn commit_filter_edit(&mut self) {
        let Some((index, te)) = self.filter_edit.take() else {
            return;
        };
        let mut state = self.state.borrow_mut();
        let changed = if let Some(rule) = state.config.search.filters.get_mut(index) {
            let kw = te.text.trim().to_string();
            if rule.keyword != kw {
                rule.keyword = kw;
                true
            } else {
                false
            }
        } else {
            false
        };
        if changed {
            state.persist();
            log::info!("过滤关键词已更新: 规则 {index} 喵");
        }
    }

    /// 切换过滤规则的大小写判定喵
    fn toggle_filter_case(&mut self, index: usize) {
        let mut state = self.state.borrow_mut();
        let now = if let Some(rule) = state.config.search.filters.get_mut(index) {
            rule.case_sensitive = !rule.case_sensitive;
            Some(rule.case_sensitive)
        } else {
            None
        };
        if let Some(v) = now {
            state.persist();
            log::debug!("过滤规则 {index} 大小写判定 → {v} 喵");
        }
    }

    /// 删除过滤规则喵
    fn delete_filter(&mut self, index: usize) {
        let mut state = self.state.borrow_mut();
        if index < state.config.search.filters.len() {
            state.config.search.filters.remove(index);
            state.persist();
            log::info!("过滤规则 {index} 已删除,剩余 {} 条喵", state.config.search.filters.len());
        }
        self.filter_edit = None;
    }

    /// 提交指令别名缓冲喵(逗号/空格分隔,保序去重)喵
    fn commit_command_edit(&mut self) {
        let Some((index, te)) = self.command_edit.take() else {
            return;
        };
        let mut state = self.state.borrow_mut();
        // 逗号(中英文)/空格均可作分隔符喵
        let mut aliases: Vec<String> = Vec::new();
        for part in te.text.split(|c: char| c == ',' || c == '\u{ff0c}' || c.is_whitespace()) {
            let part = part.trim();
            if !part.is_empty() && !aliases.iter().any(|a: &String| a.eq_ignore_ascii_case(part)) {
                aliases.push(part.to_string());
            }
        }
        let mut changed = false;
        if let Some(entry) = state.config.search.commands.get_mut(index)
            && entry.aliases != aliases
        {
            entry.aliases = aliases;
            changed = true;
        }
        if changed {
            state.persist();
            log::info!("指令 {index} 别名已更新喵~");
        }
    }

    /// 切换指令的系统命令类型喵
    fn cycle_command_kind(&mut self, index: usize) {
        use crate::platform::SystemCommandKind as K;
        let mut state = self.state.borrow_mut();
        // 借用收在块内,取好展示名再持久化喵
        let title = {
            let Some(entry) = state.config.search.commands.get_mut(index) else {
                return;
            };
            entry.kind = match entry.kind {
                K::Lock => K::Sleep,
                K::Sleep => K::Shutdown,
                K::Shutdown => K::Restart,
                K::Restart => K::Lock,
            };
            entry.kind.title()
        };
        state.persist();
        log::info!("指令 {index} 类型 → {title} 喵");
        self.command_edit = None;
    }

    /// 删除指令喵
    fn delete_command(&mut self, index: usize) {
        let mut state = self.state.borrow_mut();
        if index < state.config.search.commands.len() {
            state.config.search.commands.remove(index);
            state.persist();
            log::info!("指令 {index} 已删除,剩余 {} 条喵", state.config.search.commands.len());
        }
        self.command_edit = None;
    }

    /// 提交自定义浏览器缓冲喵(空串 = 回到系统默认)喵
    fn commit_browser_edit(&mut self) {
        let Some(te) = self.browser_edit.take() else {
            return;
        };
        let mut state = self.state.borrow_mut();
        let value = te.text.trim().to_string();
        if state.config.search.web_browser != value {
            state.config.search.web_browser = value;
            state.persist();
            log::info!("自定义浏览器已更新喵~");
        }
    }

    /// 手动键入提交: 解析缓冲 → 写配置(表内自带钳制)喵
    fn commit_stepper_edit(&mut self) {
        let Some((id, te)) = self.stepper_edit.take() else {
            return;
        };
        match te.text.trim().parse::<i32>() {
            Ok(v) => {
                self.apply_data(id, RowValue::Num(v));
                log::debug!("数值键入提交: {id:?} = {v} 喵");
            }
            Err(_) => log::warn!("数值键入非法,已放弃: {id:?} = {:?} 喵", te.text),
        }
    }

    /// 按钮动作喵(数据行以外的动作型按钮)喵
    fn press_button(&mut self, id: RowId) {
        match id {
            RowId::HotkeyRecord => {
                // 进入录制模式: 等待用户按下新的组合键(Esc 取消)喵
                self.hotkey_capture = true;
                log::info!("热键录制中,请按下新组合键喵~");
            }
            RowId::Reset => {
                {
                    let mut state = self.state.borrow_mut();
                    state.config = AppConfig::default();
                    state.persist();
                    log::info!("已恢复默认设置喵~");
                }
                // 重置后重新注册热键 + 同步开机自启 + 重建渲染器 + 清空编辑态喵
                self.commands.borrow_mut().push_back(Command::ReapplyHotkey);
                self.commands.borrow_mut().push_back(Command::RecreateRenderer);
                self.platform.set_auto_start(false);
                self.recreate_renderer();
                self.clear_edits();
                self.scroll = 0.0;
                self.scroll_target = 0.0;
            }
            RowId::OpenAppManager => {
                // 应用管理中心窗口自己检测标志并显示喵
                self.state.borrow_mut().manager_visible = true;
                log::info!("已请求打开应用管理中心喵~");
            }
            RowId::FilterAdd => {
                let mut state = self.state.borrow_mut();
                state
                    .config
                    .search
                    .filters
                    .push(crate::app::config::FilterRule::default());
                let count = state.config.search.filters.len();
                state.persist();
                log::info!("添加过滤关键词行,共 {count} 条喵");
                // 自动进入新一行的编辑喵
                self.stepper_edit = None;
                self.filter_edit = Some((count - 1, TextEdit::default()));
            }
            RowId::CommandAdd => {
                let mut state = self.state.borrow_mut();
                state
                    .config
                    .search
                    .commands
                    .push(crate::app::config::CommandEntry {
                        aliases: Vec::new(),
                        kind: crate::platform::SystemCommandKind::Lock,
                    });
                let count = state.config.search.commands.len();
                state.persist();
                log::info!("添加指令行,共 {count} 条喵");
                // 自动进入新一行的编辑喵
                self.stepper_edit = None;
                self.filter_edit = None;
                self.browser_edit = None;
                self.command_edit = Some((count - 1, TextEdit::default()));
            }
            _ => {}
        }
    }

    /// 提交当前活跃的编辑槽喵(点击离开输入框时调用,焦点随之释放)喵
    fn commit_active_edit(&mut self) {
        if self.filter_edit.is_some() {
            self.commit_filter_edit();
        } else if self.command_edit.is_some() {
            self.commit_command_edit();
        } else if self.browser_edit.is_some() {
            self.commit_browser_edit();
        } else if self.stepper_edit.is_some() {
            self.commit_stepper_edit();
        }
    }

    /// 文本编辑按键喵: 按下即提交(Enter)/取消(Esc),其余字符走 on_char 喵
    fn edit_key(&mut self, key: crate::platform::Key, kind: &mut dyn FnMut(&mut SettingsWindow)) -> bool {
        match key {
            crate::platform::Key::Backspace => {
                if let Some((_, te)) = &mut self.stepper_edit {
                    te.backspace();
                } else if let Some((_, te)) = &mut self.filter_edit {
                    te.backspace();
                } else if let Some((_, te)) = &mut self.command_edit {
                    te.backspace();
                } else if let Some(te) = &mut self.browser_edit {
                    te.backspace();
                }
                self.render();
                true
            }
            crate::platform::Key::Delete => {
                if let Some((_, te)) = &mut self.stepper_edit {
                    te.delete();
                } else if let Some((_, te)) = &mut self.filter_edit {
                    te.delete();
                } else if let Some((_, te)) = &mut self.command_edit {
                    te.delete();
                } else if let Some(te) = &mut self.browser_edit {
                    te.delete();
                }
                self.render();
                true
            }
            crate::platform::Key::Enter => {
                kind(self);
                self.render();
                true
            }
            crate::platform::Key::Escape => {
                self.stepper_edit = None;
                self.filter_edit = None;
                self.command_edit = None;
                self.browser_edit = None;
                self.render();
                true
            }
            _ => false,
        }
    }

    fn on_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        // 过滤关键词编辑喵
        if let Some((_, te)) = &mut self.filter_edit {
            te.insert_char(ch);
            self.render();
            return;
        }
        // 指令别名编辑喵
        if let Some((_, te)) = &mut self.command_edit {
            te.insert_char(ch);
            self.render();
            return;
        }
        // 自定义浏览器编辑喵
        if let Some(te) = &mut self.browser_edit {
            te.insert_char(ch);
            self.render();
            return;
        }
        // 数值编辑: 只收数字;负号仅当整段选中或文本为空时允许喵
        if let Some((_, te)) = &mut self.stepper_edit {
            let ok = ch.is_ascii_digit()
                || (ch == '-' && (te.text.is_empty() || te.selection().is_some()));
            if ok {
                te.insert_char(ch);
                self.render();
            }
        }
    }

    fn on_key(&mut self, key: crate::platform::Key) {
        // 三个文本输入框共用编辑按键逻辑,按优先级处理提交喵
        if self.filter_edit.is_some() {
            self.edit_key(key, &mut |w| w.commit_filter_edit());
            return;
        }
        if self.command_edit.is_some() {
            self.edit_key(key, &mut |w| w.commit_command_edit());
            return;
        }
        if self.browser_edit.is_some() {
            self.edit_key(key, &mut |w| w.commit_browser_edit());
            return;
        }
        if self.stepper_edit.is_some() {
            self.edit_key(key, &mut |w| w.commit_stepper_edit());
        }
    }

    /// 热键录制: 收到按键组合时保存并通知重新注册喵
    fn on_hotkey_chord(&mut self, modifiers: String, key: String) {
        if !self.hotkey_capture {
            return;
        }
        // Esc 取消录制喵
        if key == "esc" {
            self.hotkey_capture = false;
            log::debug!("热键录制已取消喵");
            self.render();
            return;
        }
        // 只接受带修饰键的组合,防止误设单键热键抢占输入喵
        if modifiers.is_empty() {
            log::debug!("热键需包含修饰键,忽略 {key} 喵");
            return;
        }
        {
            let mut state = self.state.borrow_mut();
            state.config.hotkey.enabled = true;
            state.config.hotkey.modifiers = modifiers.clone();
            state.config.hotkey.key = key.clone();
            state.persist();
        }
        self.hotkey_capture = false;
        self.commands.borrow_mut().push_back(Command::ReapplyHotkey);
        log::info!("全局热键已更新: {} + {} 喵", modifiers, key);
        self.render();
    }

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
            // 打开应用管理中心给用户可见反馈喵(应用陈列已整体迁过去)喵
            self.state.borrow_mut().manager_visible = true;
            self.render();
            log::info!("拖入注册完成,新增 {added} 个应用喵");
        }
    }

    /// 心跳喵: 检查显示请求、推进动效并渲染喵
    fn tick(&mut self) {
        let should_show = self.state.borrow().settings_visible;
        if should_show && !self.visible {
            self.show();
        }
        if self.visible {
            let animating = self.step_anim();
            self.render();
            // 动效未收敛时保持动画帧率,静止后回低频心跳喵
            let next = if animating { ANIM_FRAME_MS } else { HEARTBEAT_MS };
            self.set_timer_interval(next);
        }
    }
}

impl WindowHandler for SettingsWindow {
    fn on_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::MouseDown(x, y) => self.handle_click(x, y),
            WindowEvent::Char(ch) => self.on_char(ch),
            WindowEvent::KeyDown(key) => self.on_key(key),
            WindowEvent::HotkeyChord { modifiers, key } => self.on_hotkey_chord(modifiers, key),
            WindowEvent::ImePreedit(_) => {}
            WindowEvent::FilesDropped(paths) => self.drop_files(paths),
            WindowEvent::MouseMove(x, y) => self.on_mouse_move(x, y),
            WindowEvent::MouseUp => {
                self.win_drag = None;
                self.text_drag = None;
                self.scroll_drag = None;
                // 结束拖选模态(保留选中区,清锚点)喵
                if let Some((_, te)) = &mut self.stepper_edit {
                    te.end_select();
                } else if let Some((_, te)) = &mut self.filter_edit {
                    te.end_select();
                } else if let Some((_, te)) = &mut self.command_edit {
                    te.end_select();
                } else if let Some(te) = &mut self.browser_edit {
                    te.end_select();
                }
            }
            WindowEvent::MouseWheel(delta) => {
                if self.visible {
                    // 滚轮更新滚动目标(每格滚动一行);动效开着就平滑趋近喵
                    let step = delta / 120.0 * 48.0;
                    let max = self.max_scroll();
                    if self.anim_enabled() {
                        self.scroll_target = (self.scroll_target - step).clamp(0.0, max);
                    } else {
                        self.scroll = (self.scroll - step).clamp(0.0, max);
                        self.scroll_target = self.scroll;
                    }
                    self.render();
                }
            }
            WindowEvent::LostFocus => {
                // 失焦不关闭(配置窗口常驻)喵
            }
            WindowEvent::Timer => self.tick(),
            WindowEvent::Close => self.hide(),
            WindowEvent::Hotkey | WindowEvent::ContextMenu(..) | WindowEvent::IpcCommand(_) => {}
        }
    }
}

// ---------------------------------------------------------------------------
// 页面模型构建喵
// ---------------------------------------------------------------------------

fn btn(id: RowId, label: String) -> SettingsRow {
    SettingsRow::Button { id, label }
}

/// 把数据行描述转换成渲染行喵(弹簧行标签拼上过渡名)喵
fn desc_to_row(desc: &settings_data::RowDesc, cfg: &AppConfig) -> SettingsRow {
    let label = match desc.id {
        RowId::SpringDuration(i) | RowId::SpringBounce(i) => {
            format!("{} {}", settings_data::spring_label(i as usize), desc.label)
        }
        _ => desc.label.to_string(),
    };
    match desc.kind {
        RowKind::Switch => SettingsRow::Switch {
            id: desc.id,
            label,
            value: desc.value(cfg) == RowValue::Bool(true),
        },
        RowKind::Stepper { min, max, step, unit } => SettingsRow::Stepper {
            id: desc.id,
            label,
            value: match desc.value(cfg) {
                RowValue::Num(n) => n,
                _ => 0,
            },
            min,
            max,
            step,
            unit: unit.into(),
        },
        RowKind::Choice { .. } => SettingsRow::Choice {
            id: desc.id,
            label,
            value: desc.choice_text(cfg),
        },
    }
}

/// 把数据驱动行按 (页, 分组) 聚合喵(注册顺序即显示顺序)喵
fn collect_data_groups(cfg: &AppConfig) -> [Vec<(String, Vec<SettingsRow>)>; 3] {
    let mut pages: [Vec<(String, Vec<SettingsRow>)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for desc in settings_data::data_rows() {
        let page = pages
            .get_mut(desc.page)
            .unwrap_or_else(|| panic!("数据行归属页越界: {:?}", desc.id));
        let row = desc_to_row(&desc, cfg);
        match page.iter_mut().find(|(g, _)| g == desc.group) {
            Some((_, rows)) => rows.push(row),
            None => page.push((desc.group.to_string(), vec![row])),
        }
    }
    pages
}

/// 从当前配置构建页面模型喵(过滤草稿由绘制层的编辑焦点接管)喵
///
/// 纯函数无副作用,公开供快照渲染等离屏验证使用喵。
/// 应用陈列 / 打标签已整体迁入应用管理中心,这里只留入口与过滤喵。
pub fn build_pages(config: &AppConfig, hotkey_capture: bool) -> Vec<SettingsPage> {
    let mut data_pages = collect_data_groups(config);

    // ---- 常规页: 数据分组 + 特殊分组喵 ----
    let mut general: Vec<SettingsGroup> = data_pages[PAGE_GENERAL]
        .drain(..)
        .map(|(title, rows)| SettingsGroup { title, rows })
        .collect();
    // 热键组: 数据行(开关)后追加录制按钮喵
    if let Some(group) = general.iter_mut().find(|g| g.title == "热键") {
        group.rows.push(btn(
            RowId::HotkeyRecord,
            if hotkey_capture {
                "按下新组合键··· (Esc 取消)".into()
            } else if config.hotkey.modifiers.is_empty() {
                format!("重新录制热键 · {}", config.hotkey.key)
            } else {
                format!(
                    "重新录制热键 · {}+{}",
                    config.hotkey.modifiers, config.hotkey.key
                )
            },
        ));
    }
    // Web 搜索组: 浏览器输入行排在引擎行前喵
    if let Some(group) = general.iter_mut().find(|g| g.title == "Web 搜索") {
        group.rows.insert(
            0,
            SettingsRow::Input {
                id: RowId::WebBrowser,
                label: "自定义浏览器".into(),
                value: config.search.web_browser.clone(),
                placeholder: "留空用系统默认;可含 %1 占位喵".into(),
            },
        );
    }
    let mut command_rows: Vec<SettingsRow> = config
        .search
        .commands
        .iter()
        .enumerate()
        .map(|(i, c)| SettingsRow::Command {
            index: i,
            kind: c.kind,
            aliases: c.aliases.join(", "),
        })
        .collect();
    command_rows.push(btn(RowId::CommandAdd, "添加指令".into()));
    general.push(SettingsGroup {
        title: "指令模块".into(),
        rows: command_rows,
    });
    general.push(SettingsGroup {
        title: "维护".into(),
        rows: vec![btn(RowId::Reset, "恢复默认设置".into())],
    });

    // ---- 应用页: 管理中心入口 + 关键词过滤喵(应用陈列/打标签在管理中心)喵 ----
    let mut filter_rows: Vec<SettingsRow> = Vec::new();
    for (i, rule) in config.search.filters.iter().enumerate() {
        filter_rows.push(SettingsRow::Filter {
            index: i,
            keyword: rule.keyword.clone(),
            case_sensitive: rule.case_sensitive,
        });
    }
    filter_rows.push(btn(RowId::FilterAdd, "添加过滤关键词".into()));

    vec![
        SettingsPage {
            title: "常规".into(),
            subtitle: "主题 · 热键 · 列表显示喵".into(),
            groups: general,
        },
        SettingsPage {
            title: "灵动岛".into(),
            subtitle: "胶囊几何 · 锚点 · 动效策略喵".into(),
            groups: data_pages[1]
                .drain(..)
                .map(|(title, rows)| SettingsGroup { title, rows })
                .collect(),
        },
        SettingsPage {
            title: "弹簧".into(),
            subtitle: "六段过渡的时长与弹性手感喵".into(),
            groups: data_pages[2]
                .drain(..)
                .map(|(title, rows)| SettingsGroup { title, rows })
                .collect(),
        },
        SettingsPage {
            title: "应用".into(),
            subtitle: "管理中心 · 关键词过滤喵".into(),
            groups: vec![
                SettingsGroup {
                    title: "管理".into(),
                    rows: vec![btn(RowId::OpenAppManager, "打开应用管理中心".into())],
                },
                SettingsGroup {
                    title: "关键词过滤".into(),
                    rows: filter_rows,
                },
            ],
        },
        SettingsPage {
            title: "关于".into(),
            subtitle: "版本与内核信息喵".into(),
            groups: vec![SettingsGroup {
                title: "关于".into(),
                rows: vec![
                    SettingsRow::Label {
                        label: "版本".into(),
                        value: env!("CARGO_PKG_VERSION").into(),
                    },
                    SettingsRow::Label {
                        label: "全称".into(),
                        value: "meow app launcher".into(),
                    },
                    SettingsRow::Label {
                        label: "指令".into(),
                        value: "meowal".into(),
                    },
                    SettingsRow::Label {
                        label: "岛内核".into(),
                        value: "spring + FSM + 边界回弹".into(),
                    },
                ],
            }],
        },
    ]
}
