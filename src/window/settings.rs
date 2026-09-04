//! 配置窗口喵~
//!
//! 依据 window-design skill 的配置 GUI 设计,实现「侧边栏 + 分组卡片」设置界面喵。
//! 持有共享状态,修改配置项并持久化;交互通过命中测试驱动喵。
//!
//! 状态机: 常驻(隐藏) → 收到 `settings_visible` 标志 → 显示并渲染 → 红点关闭喵。

use crate::animation::{DurationBounce, IslandTransition};
use crate::app::config::{AppConfig, AppLayout, SearchMode};
use crate::app::{Command, SharedState};
use crate::platform::{Platform, PlatformWindow, WindowEvent, WindowHandler};
use crate::render::edit::{caret_from_x, TextEdit};
use crate::render::font::FontCache;
use crate::render::settings::{
    paint_settings, EditFocus, RowHit, RowId, SETTINGS_HEIGHT, SETTINGS_WIDTH, SettingsGroup,
    SettingsLayout, SettingsPage, SettingsRow,
};
use crate::render::{Renderer, SettingsTheme};
use skia_safe::Rect;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

/// 心跳间隔(ms): 隐藏时低频检查显示请求喵
const HEARTBEAT_MS: u32 = 100;

/// 正在被鼠标拖选文本的输入框喵
#[derive(Debug, Clone, Copy)]
struct TextDrag {
    /// 拖选目标的输入框矩形(逻辑坐标,随滚动绘制)喵
    rect: Rect,
}

/// 配置窗口喵
pub struct SettingsWindow {
    /// 共享应用状态喵
    state: SharedState,
    /// 平台句柄喵
    platform: Arc<dyn Platform>,
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
    /// 最近一次布局结果(供命中测试)喵
    layout: Option<SettingsLayout>,
    /// DPI 缩放系数喵
    scale: f32,
    /// BGRA 像素缓冲喵
    pixels: Vec<u8>,
    /// 当前选中的应用名喵
    selected_app: Option<String>,
    /// 正在手动键入的数值行(id + 文本缓冲)喵
    stepper_edit: Option<(RowId, TextEdit)>,
    /// 正在编辑的过滤关键词行(规则下标 + 文本缓冲)喵
    filter_edit: Option<(usize, TextEdit)>,
    /// 正在编辑的标签行(文本缓冲)喵
    tag_edit: Option<TextEdit>,
    /// 正在拖选文本的输入框喵
    text_drag: Option<TextDrag>,
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
        platform: Arc<dyn Platform>,
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
        let renderer = Renderer::new(win_w, win_h, backend, &*platform).unwrap_or_else(|| {
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
            layout: None,
            scale,
            pixels: Vec::new(),
            selected_app: None,
            stepper_edit: None,
            filter_edit: None,
            tag_edit: None,
            text_drag: None,
            hotkey_capture: false,
            win_w: SETTINGS_WIDTH,
            win_h: SETTINGS_HEIGHT,
            win_pos: Some((x, y)),
            win_drag: None,
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
        self.platform.show_window(&self.window, true);
        self.render();
    }

    /// 隐藏配置窗口喵
    fn hide(&mut self) {
        log::info!("隐藏配置窗口喵~");
        self.visible = false;
        self.state.borrow_mut().settings_visible = false;
        self.platform.show_window(&self.window, false);
    }

    // ---------------------------------------------------------------------
    // 渲染
    // ---------------------------------------------------------------------

    /// 重新生成页面并渲染喵
    fn render(&mut self) {
        // 从当前配置重建页面喵
        let pages = {
            let state = self.state.borrow();
            build_pages(
                &state.config,
                &state.registry.apps,
                self.selected_app.as_deref(),
                self.hotkey_capture,
            )
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
            dirty_rows(&state.config)
        };
        // 汇总当前编辑焦点(同一时刻只有一个输入框在编辑)喵
        let edit = EditFocus {
            stepper: self.stepper_edit.as_ref().map(|(id, te)| (*id, te)),
            filter: self.filter_edit.as_ref().map(|(i, te)| (*i, te)),
            tag: self.tag_edit.as_ref(),
        };
        let layout = paint_settings(
            canvas,
            &theme,
            &self.fonts,
            &pages,
            self.current_page,
            self.scroll,
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
        if let Some(renderer) = Renderer::new(w, h, backend, &*self.platform) {
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
                RowHit::Input(_) => self.click_tag_box(rect, lx),
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
        let cur = self.stepper_value(id);
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
        self.tag_edit = None;
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
        self.tag_edit = None;
        self.text_drag = Some(TextDrag { rect: box_rect });
        log::debug!("过滤关键词框聚焦: 规则 {index} 喵");
        self.render();
    }

    /// 点击标签输入框喵
    fn click_tag_box(&mut self, box_rect: Rect, lx: f32) {
        let paint = self.box_paint();
        let font = self.fonts.font(12.0);
        if let Some(te) = self.tag_edit.as_mut() {
            let caret = caret_from_x(&font, &paint, &te.text, box_rect.left + 8.0, lx);
            te.begin_select(caret);
        } else {
            self.tag_edit = Some(TextEdit::default());
        }
        self.stepper_edit = None;
        self.filter_edit = None;
        self.text_drag = Some(TextDrag { rect: box_rect });
        self.render();
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

    /// 拖动窗口 / 调整窗口尺寸喵;先处理文本拖选喵
    fn on_mouse_move(&mut self, x: f32, y: f32) {
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
            } else if let Some(te) = &mut self.tag_edit {
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

    /// 处理命中动作喵(文本框类命中已在 handle_click 拦截)喵
    fn apply_hit(&mut self, hit: RowHit) {
        match hit {
            RowHit::Nav(i) => {
                self.current_page = i;
                self.scroll = 0.0;
                self.stepper_edit = None;
                self.filter_edit = None;
                self.tag_edit = None;
                self.text_drag = None;
                self.hotkey_capture = false;
            }
            RowHit::Close => self.hide(),
            RowHit::Switch(id) => self.toggle_switch(id),
            RowHit::StepperDec(id) => self.adjust_stepper(id, -1),
            RowHit::StepperInc(id) => self.adjust_stepper(id, 1),
            RowHit::Button(id) => self.press_button(id),
            RowHit::Restore(id) => self.restore_row_defaults(id),
            RowHit::AppPick(i) => self.pick_app(i),
            RowHit::FavStar(i) => self.toggle_fav(i),
            RowHit::Chip(i) => self.remove_chip(i),
            RowHit::FilterCase(i) => self.toggle_filter_case(i),
            RowHit::FilterDelete(i) => self.delete_filter(i),
            // 窗口栏/手柄与文本框在 handle_click 里先行拦截,这里仅收尾喵
            RowHit::TitleBar
            | RowHit::ResizeGrip
            | RowHit::StepperEdit(_)
            | RowHit::FilterInput(_)
            | RowHit::Input(_) => {}
        }
        self.render();
    }

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

    /// 恢复单个配置项到默认值喵
    fn restore_row_defaults(&mut self, id: RowId) {
        // 正在键入同一行时先退出编辑态,避免草稿过期喵
        if self.stepper_edit.as_ref().is_some_and(|(eid, _)| *eid == id) {
            self.stepper_edit = None;
        }
        let mut state = self.state.borrow_mut();
        if restore_default(id, &mut state.config) {
            state.persist();
            log::info!("已恢复 {id:?} 到默认值喵~");
            if id == RowId::HotkeyEnabled {
                self.commands.borrow_mut().push_back(Command::ReapplyHotkey);
            } else if id == RowId::AutoStart {
                // 恢复默认即关闭自启,同步注册表喵
                self.platform.set_auto_start(state.config.auto_start);
            }
        }
    }

    /// 读取某数值行的当前显示值喵
    fn stepper_value(&self, id: RowId) -> i32 {
        let Some(page) = self.pages.get(self.current_page) else {
            return 0;
        };
        for group in &page.groups {
            for row in &group.rows {
                if let SettingsRow::Stepper {
                    id: rid, value, ..
                } = row
                    && *rid == id
                {
                    return *value;
                }
            }
        }
        0
    }

    /// 切换开关喵
    fn toggle_switch(&mut self, id: RowId) {
        let mut state = self.state.borrow_mut();
        match id {
            RowId::AlwaysOnTop => state.config.window.always_on_top = !state.config.window.always_on_top,
            RowId::HotkeyEnabled => state.config.hotkey.enabled = !state.config.hotkey.enabled,
            RowId::ShowRecent => state.config.window.show_recent = !state.config.window.show_recent,
            RowId::ShowFavorites => {
                state.config.window.show_favorites = !state.config.window.show_favorites
            }
            RowId::ShowFrequent => state.config.window.show_frequent = !state.config.window.show_frequent,
            RowId::ShowAll => state.config.window.show_all = !state.config.window.show_all,
            RowId::AutoMorph => state.config.island.auto_morph = !state.config.island.auto_morph,
            RowId::Draggable => state.config.island.draggable = !state.config.island.draggable,
            RowId::ReduceMotion => {
                state.config.island.reduce_motion = !state.config.island.reduce_motion
            }
            RowId::AutoStart => {
                state.config.auto_start = !state.config.auto_start;
                // 立即写注册表;失败则回滚,保证配置与系统状态一致喵
                let ok = self.platform.set_auto_start(state.config.auto_start);
                if ok {
                    log::info!("开机自启 → {} 喵", state.config.auto_start);
                } else {
                    state.config.auto_start = !state.config.auto_start;
                    log::warn!("开机自启设置失败,已回滚喵~");
                }
            }
            _ => {}
        }
        state.persist();
        log::debug!("开关 {id:?} 已热写配置喵");
    }

    /// 步进调节喵: 按行配置的步幅增减喵
    fn adjust_stepper(&mut self, id: RowId, dir: i32) {
        // 若正在手动键入同一行,先退出键入态,避免草稿与最新值错位喵
        if self.stepper_edit.as_ref().is_some_and(|(eid, _)| *eid == id) {
            self.stepper_edit = None;
        }
        let Some((_, _, step)) = self.stepper_meta(id) else {
            return;
        };
        let cur = self.stepper_value(id) + dir * step;
        self.apply_stepper(id, cur);
    }

    /// 手动键入提交: 解析缓冲 → 钳制 → 写配置喵
    fn commit_stepper_edit(&mut self) {
        let Some((id, te)) = self.stepper_edit.take() else {
            return;
        };
        match te.text.trim().parse::<i32>() {
            Ok(v) => {
                self.apply_stepper(id, v);
                log::debug!("数值键入提交: {id:?} = {v} 喵");
            }
            Err(_) => log::warn!("数值键入非法,已放弃: {id:?} = {:?} 喵", te.text),
        }
    }

    /// 写配置(带范围钳制),有变化时持久化喵
    fn apply_stepper(&mut self, id: RowId, value: i32) {
        let (min, max, _) = self.stepper_meta(id).unwrap_or((i32::MIN, i32::MAX, 1));
        let v = value.clamp(min, max);
        let mut state = self.state.borrow_mut();
        let island = &mut state.config.island;
        let changed = match id {
            RowId::IslandW => set_f64(&mut island.width, v),
            RowId::IslandH => set_f64(&mut island.height, v),
            RowId::IslandX => set_f64(&mut island.x, v),
            RowId::IslandY => set_f64(&mut island.y, v),
            RowId::ExpandedW => {
                let ok = set_f64(&mut island.expanded_width, v);
                state.config.window.width = island.expanded_width;
                ok
            }
            RowId::ExpandedH => {
                let ok = set_f64(&mut island.expanded_height, v);
                state.config.window.height = island.expanded_height;
                ok
            }
            RowId::ExpandedR => set_f64(&mut island.expanded_radius, v),
            RowId::InputRatio => set_ratio(&mut island.input_ratio, v),
            RowId::Margin => set_f64(&mut island.margin, v),
            RowId::Squash => set_ratio(&mut island.summon_squash, v),
            RowId::IconSize => set_f32(&mut state.config.window.icon_size, v),
            RowId::SpringDuration(i) => {
                let tune = island.springs.get_mut(spring_at(i));
                let last = tune.duration;
                tune.duration = v as f64 / 100.0;
                (tune.duration - last).abs() > f64::EPSILON
            }
            RowId::SpringBounce(i) => {
                let tune = island.springs.get_mut(spring_at(i));
                let last = tune.bounce;
                tune.bounce = (v as f64 / 100.0).clamp(0.0, 0.95);
                (tune.bounce - last).abs() > f64::EPSILON
            }
            _ => false,
        };
        if changed {
            state.persist();
        }
    }

    /// 读取数值行的 (min, max, step) 喵
    fn stepper_meta(&self, id: RowId) -> Option<(i32, i32, i32)> {
        let page = self.pages.get(self.current_page)?;
        for group in &page.groups {
            for row in &group.rows {
                if let SettingsRow::Stepper {
                    id: rid,
                    min,
                    max,
                    step,
                    ..
                } = row
                    && *rid == id
                {
                    return Some((*min, *max, *step));
                }
            }
        }
        None
    }

    /// 按钮动作喵
    fn press_button(&mut self, id: RowId) {
        match id {
            RowId::HotkeyRecord => {
                // 进入录制模式: 等待用户按下新的组合键(Esc 取消)喵
                self.hotkey_capture = true;
                log::info!("热键录制中,请按下新组合键喵~");
            }
            RowId::Theme => {
                let mut state = self.state.borrow_mut();
                state.config.theme.preset = state.config.theme.preset.cycle();
                state.persist();
                log::info!("主题 → {} 喵", state.config.theme.preset.label());
            }
            RowId::AnimFps => {
                let mut state = self.state.borrow_mut();
                state.config.island.anim_fps = next_fps(state.config.island.anim_fps);
                state.persist();
                log::info!(
                    "动画帧率 → {} 喵",
                    fps_label(state.config.island.anim_fps)
                );
            }
            RowId::AppLayout => {
                let mut state = self.state.borrow_mut();
                state.config.window.layout = match state.config.window.layout {
                    AppLayout::Row => AppLayout::Grid,
                    AppLayout::Grid => AppLayout::Row,
                };
                state.persist();
                log::info!(
                    "结果排版 → {} 喵",
                    app_layout_label(state.config.window.layout)
                );
            }
            RowId::Reset => {
                let mut state = self.state.borrow_mut();
                state.config = AppConfig::default();
                state.persist();
                log::info!("已恢复默认设置喵~");
                // 重置后重新注册热键 + 同步开机自启 + 清空各编辑态喵
                drop(state);
                self.commands.borrow_mut().push_back(Command::ReapplyHotkey);
                self.commands.borrow_mut().push_back(Command::RecreateRenderer);
                self.platform.set_auto_start(false);
                self.recreate_renderer();
                self.stepper_edit = None;
                self.filter_edit = None;
                self.tag_edit = None;
                self.text_drag = None;
                self.hotkey_capture = false;
                self.scroll = 0.0;
            }
            RowId::MotionMode => {
                let mut state = self.state.borrow_mut();
                state.config.island.motion_mode = state.config.island.motion_mode.cycle();
                state.persist();
                log::info!("运动模式 → {} 喵", state.config.island.motion_mode.label());
            }
            RowId::Easing => {
                let mut state = self.state.borrow_mut();
                state.config.island.easing = state.config.island.easing.cycle();
                state.persist();
                log::info!("缓动 → {} 喵", state.config.island.easing.label());
            }
            RowId::SearchMode => {
                let mut state = self.state.borrow_mut();
                state.config.search.default_mode = match state.config.search.default_mode {
                    SearchMode::Name => SearchMode::Tag,
                    SearchMode::Tag => SearchMode::Initial,
                    SearchMode::Initial => SearchMode::Name,
                };
                state.persist();
            }
            RowId::RenderBackend => {
                let mut state = self.state.borrow_mut();
                state.config.render_backend = state.config.render_backend.cycle();
                state.persist();
                log::info!("渲染后端 → {} 喵", state.config.render_backend.label());
                // 切换后重建本窗口渲染器,并通知启动器同步重建喵
                drop(state);
                self.recreate_renderer();
                self.commands.borrow_mut().push_back(Command::RecreateRenderer);
            }
            RowId::Rescan => {
                self.commands.borrow_mut().push_back(Command::Rescan);
                log::info!("已请求重新扫描应用喵~");
            }
            RowId::RemoveApp => {
                if let Some(name) = self.selected_app.clone() {
                    self.state.borrow_mut().remove_app(&name);
                    self.selected_app = None;
                    log::info!("已移除应用 {name} 喵");
                }
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
                self.tag_edit = None;
                self.filter_edit = Some((count - 1, TextEdit::default()));
            }
            _ => {}
        }
    }

    fn pick_app(&mut self, index: usize) {
        self.selected_app = self.app_name_at(index);
        self.tag_edit = None;
        self.text_drag = None;
    }

    fn toggle_fav(&mut self, index: usize) {
        if let Some(name) = self.app_name_at(index) {
            self.state.borrow_mut().toggle_favorite(&name);
        }
    }

    fn remove_chip(&mut self, index: usize) {
        let Some(app_name) = self.selected_app.clone() else {
            return;
        };
        let tag = self.chip_label_at(index);
        if let Some(tag) = tag {
            self.state.borrow_mut().remove_tag(&app_name, &tag);
        }
    }

    fn app_name_at(&self, index: usize) -> Option<String> {
        let page = self.pages.get(self.current_page)?;
        let mut idx = 0;
        for group in &page.groups {
            for row in &group.rows {
                if idx == index
                    && let SettingsRow::AppPick { name, .. } = row
                {
                    return Some(name.clone());
                }
                idx += 1;
            }
        }
        None
    }

    fn chip_label_at(&self, index: usize) -> Option<String> {
        let page = self.pages.get(self.current_page)?;
        let mut idx = 0;
        for group in &page.groups {
            for row in &group.rows {
                if idx == index
                    && let SettingsRow::Chip { label } = row
                {
                    return Some(label.clone());
                }
                idx += 1;
            }
        }
        None
    }

    /// 文本编辑按键喵: 按下即提交(Enter)/取消(Esc),其余字符走 on_char 喵
    fn edit_key(&mut self, key: crate::platform::Key, kind: &mut dyn FnMut(&mut SettingsWindow)) -> bool {
        match key {
            crate::platform::Key::Backspace => {
                if let Some((_, te)) = &mut self.stepper_edit {
                    te.backspace();
                } else if let Some((_, te)) = &mut self.filter_edit {
                    te.backspace();
                } else if let Some(te) = &mut self.tag_edit {
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
                } else if let Some(te) = &mut self.tag_edit {
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
                self.tag_edit = None;
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
        // 数值编辑: 只收数字;负号仅当整段选中或文本为空时允许喵
        if let Some((_, te)) = &mut self.stepper_edit {
            let ok = ch.is_ascii_digit()
                || (ch == '-' && (te.text.is_empty() || te.selection().is_some()));
            if ok {
                te.insert_char(ch);
                self.render();
            }
            return;
        }
        // 标签编辑喵
        if let Some(te) = &mut self.tag_edit {
            te.insert_char(ch);
            self.render();
        }
    }

    fn on_key(&mut self, key: crate::platform::Key) {
        // 三个文本输入框共用编辑按键逻辑,按优先级处理提交喵
        if self.filter_edit.is_some() {
            self.edit_key(key, &mut |w| w.commit_filter_edit());
            return;
        }
        if self.stepper_edit.is_some() {
            self.edit_key(key, &mut |w| w.commit_stepper_edit());
            return;
        }
        if self.tag_edit.is_some() {
            self.edit_key(key, &mut |w| w.commit_tag_edit());
        }
    }

    /// 提交标签缓冲喵(空文本忽略)喵
    fn commit_tag_edit(&mut self) {
        let Some(te) = self.tag_edit.take() else {
            return;
        };
        let draft = te.text.trim().to_string();
        if draft.is_empty() {
            return;
        }
        if let Some(name) = self.selected_app.clone() {
            self.state.borrow_mut().add_tag(&name, &draft);
            log::info!("已为 {name} 添加标签: {draft} 喵");
        }
        self.render();
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
            // 跳到「应用」页(索引 3)给用户可见反馈喵
            self.current_page = 3;
            self.scroll = 0.0;
            self.render();
            log::info!("拖入注册完成,新增 {added} 个应用喵");
        }
    }

    /// 心跳喵: 检查显示请求并渲染喵
    fn tick(&mut self) {
        let should_show = self.state.borrow().settings_visible;
        if should_show && !self.visible {
            self.show();
        }
        if self.visible {
            self.render();
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
                // 结束拖选模态(保留选中区,清锚点)喵
                if let Some((_, te)) = &mut self.stepper_edit {
                    te.end_select();
                } else if let Some((_, te)) = &mut self.filter_edit {
                    te.end_select();
                } else if let Some(te) = &mut self.tag_edit {
                    te.end_select();
                }
            }
            WindowEvent::MouseWheel(delta) => {
                if self.visible {
                    // 滚轮滚动内容区,钳制到有效范围喵(每格滚动一行)喵
                    let max_scroll = self
                        .layout
                        .as_ref()
                        .map(|l| (l.content_height - self.win_h).max(0.0))
                        .unwrap_or(0.0);
                    self.scroll = (self.scroll - delta / 120.0 * 48.0).clamp(0.0, max_scroll);
                    self.render();
                }
            }
            WindowEvent::LostFocus => {
                // 失焦不关闭(配置窗口常驻)喵
            }
            WindowEvent::Timer => self.tick(),
            WindowEvent::Close => self.hide(),
            WindowEvent::Hotkey => {}
        }
    }
}

/// 写入整数值到 f64 字段,返回是否变化喵
fn set_f64(value: &mut f64, v: i32) -> bool {
    let new_value = v as f64;
    if (new_value - *value).abs() < f64::EPSILON {
        return false;
    }
    *value = new_value;
    true
}

/// 写入整数值到 f32 字段,返回是否变化喵
fn set_f32(value: &mut f32, v: i32) -> bool {
    let new_value = v as f32;
    if (new_value - *value).abs() < f32::EPSILON {
        return false;
    }
    *value = new_value;
    true
}

/// 写入整数值(百分数形式)到比率字段,返回是否变化喵
fn set_ratio(value: &mut f64, v: i32) -> bool {
    let new_value = v as f64 / 100.0;
    if (new_value - *value).abs() < f64::EPSILON {
        return false;
    }
    *value = new_value;
    true
}

fn spring_at(i: u8) -> IslandTransition {
    match i {
        0 => IslandTransition::Summon,
        1 => IslandTransition::Dismiss,
        2 => IslandTransition::Expand,
        3 => IslandTransition::Collapse,
        4 => IslandTransition::SummonExpanded,
        _ => IslandTransition::DismissExpanded,
    }
}

fn spring_label(i: u8) -> &'static str {
    match spring_at(i) {
        IslandTransition::Summon => "唤出",
        IslandTransition::Dismiss => "收回",
        IslandTransition::Expand => "展开",
        IslandTransition::Collapse => "收起",
        IslandTransition::SummonExpanded => "直达展开",
        IslandTransition::DismissExpanded => "直达收回",
    }
}

/// 走查所有可调配置项,返回偏离默认值的行喵
fn dirty_rows(cfg: &AppConfig) -> Vec<RowId> {
    let d = AppConfig::default();
    let mut rows = vec![
        RowId::Theme,
        RowId::AnimFps,
        RowId::MotionMode,
        RowId::Easing,
        RowId::AutoMorph,
        RowId::Draggable,
        RowId::ReduceMotion,
        RowId::IslandW,
        RowId::IslandH,
        RowId::IslandX,
        RowId::IslandY,
        RowId::ExpandedW,
        RowId::ExpandedH,
        RowId::ExpandedR,
        RowId::InputRatio,
        RowId::Margin,
        RowId::Squash,
        RowId::IconSize,
        RowId::AlwaysOnTop,
        RowId::HotkeyEnabled,
        RowId::AutoStart,
        RowId::RenderBackend,
        RowId::ShowRecent,
        RowId::ShowFavorites,
        RowId::ShowFrequent,
        RowId::ShowAll,
        RowId::SearchMode,
        RowId::AppLayout,
    ];
    rows.extend((0..6u8).map(RowId::SpringDuration));
    rows.extend((0..6u8).map(RowId::SpringBounce));
    rows.retain(|id| row_is_dirty(*id, cfg, &d));
    rows
}

/// 判断某个配置项是否偏离默认值喵
fn row_is_dirty(id: RowId, cfg: &AppConfig, d: &AppConfig) -> bool {
    match id {
        RowId::Theme => cfg.theme.preset != d.theme.preset,
        RowId::AnimFps => cfg.island.anim_fps != d.island.anim_fps,
        RowId::MotionMode => cfg.island.motion_mode != d.island.motion_mode,
        RowId::Easing => cfg.island.easing != d.island.easing,
        RowId::AutoMorph => cfg.island.auto_morph != d.island.auto_morph,
        RowId::Draggable => cfg.island.draggable != d.island.draggable,
        RowId::ReduceMotion => cfg.island.reduce_motion != d.island.reduce_motion,
        RowId::IslandW => neq(cfg.island.width, d.island.width),
        RowId::IslandH => neq(cfg.island.height, d.island.height),
        RowId::IslandX => neq(cfg.island.x, d.island.x),
        RowId::IslandY => neq(cfg.island.y, d.island.y),
        RowId::ExpandedW => neq(cfg.island.expanded_width, d.island.expanded_width),
        RowId::ExpandedH => neq(cfg.island.expanded_height, d.island.expanded_height),
        RowId::ExpandedR => neq(cfg.island.expanded_radius, d.island.expanded_radius),
        RowId::InputRatio => neq(cfg.island.input_ratio, d.island.input_ratio),
        RowId::Margin => neq(cfg.island.margin, d.island.margin),
        RowId::Squash => neq(cfg.island.summon_squash, d.island.summon_squash),
        RowId::IconSize => (cfg.window.icon_size - d.window.icon_size).abs() > f32::EPSILON,
        RowId::AlwaysOnTop => cfg.window.always_on_top != d.window.always_on_top,
        RowId::HotkeyEnabled => cfg.hotkey.enabled != d.hotkey.enabled,
        RowId::AutoStart => cfg.auto_start != d.auto_start,
        RowId::RenderBackend => cfg.render_backend != d.render_backend,
        RowId::ShowRecent => cfg.window.show_recent != d.window.show_recent,
        RowId::ShowFavorites => cfg.window.show_favorites != d.window.show_favorites,
        RowId::ShowFrequent => cfg.window.show_frequent != d.window.show_frequent,
        RowId::ShowAll => cfg.window.show_all != d.window.show_all,
        RowId::SearchMode => cfg.search.default_mode != d.search.default_mode,
        RowId::AppLayout => cfg.window.layout != d.window.layout,
        RowId::SpringDuration(i) => {
            cfg.island.springs.get(spring_at(i)) != d.island.springs.get(spring_at(i))
        }
        RowId::SpringBounce(i) => {
            cfg.island.springs.get(spring_at(i)) != d.island.springs.get(spring_at(i))
        }
        _ => false,
    }
}

/// 把单个配置项恢复为默认值,返回是否有对应项喵
fn restore_default(id: RowId, cfg: &mut AppConfig) -> bool {
    let d = AppConfig::default();
    match id {
        RowId::Theme => cfg.theme = d.theme,
        RowId::AnimFps => cfg.island.anim_fps = d.island.anim_fps,
        RowId::MotionMode => cfg.island.motion_mode = d.island.motion_mode,
        RowId::Easing => cfg.island.easing = d.island.easing,
        RowId::AutoMorph => cfg.island.auto_morph = d.island.auto_morph,
        RowId::Draggable => cfg.island.draggable = d.island.draggable,
        RowId::ReduceMotion => cfg.island.reduce_motion = d.island.reduce_motion,
        RowId::IslandW => cfg.island.width = d.island.width,
        RowId::IslandH => cfg.island.height = d.island.height,
        RowId::IslandX => cfg.island.x = d.island.x,
        RowId::IslandY => cfg.island.y = d.island.y,
        RowId::ExpandedW => {
            cfg.island.expanded_width = d.island.expanded_width;
            cfg.window.width = d.window.width;
        }
        RowId::ExpandedH => {
            cfg.island.expanded_height = d.island.expanded_height;
            cfg.window.height = d.window.height;
        }
        RowId::ExpandedR => cfg.island.expanded_radius = d.island.expanded_radius,
        RowId::InputRatio => cfg.island.input_ratio = d.island.input_ratio,
        RowId::Margin => cfg.island.margin = d.island.margin,
        RowId::Squash => cfg.island.summon_squash = d.island.summon_squash,
        RowId::IconSize => cfg.window.icon_size = d.window.icon_size,
        RowId::AlwaysOnTop => cfg.window.always_on_top = d.window.always_on_top,
        RowId::HotkeyEnabled => cfg.hotkey.enabled = d.hotkey.enabled,
        RowId::AutoStart => cfg.auto_start = d.auto_start,
        RowId::RenderBackend => cfg.render_backend = d.render_backend,
        RowId::ShowRecent => cfg.window.show_recent = d.window.show_recent,
        RowId::ShowFavorites => cfg.window.show_favorites = d.window.show_favorites,
        RowId::ShowFrequent => cfg.window.show_frequent = d.window.show_frequent,
        RowId::ShowAll => cfg.window.show_all = d.window.show_all,
        RowId::SearchMode => cfg.search.default_mode = d.search.default_mode,
        RowId::AppLayout => cfg.window.layout = d.window.layout,
        RowId::SpringDuration(i) => {
            let t = spring_at(i);
            *cfg.island.springs.get_mut(t) = d.island.springs.get(t);
        }
        RowId::SpringBounce(i) => {
            let t = spring_at(i);
            *cfg.island.springs.get_mut(t) = d.island.springs.get(t);
        }
        _ => return false,
    }
    true
}

/// f64 比较(容差)喵
fn neq(a: f64, b: f64) -> bool {
    (a - b).abs() > 1e-9
}

/// 动画帧率档位循环: 0(跟随屏刷) → 30 → 60 → 90 → 120 → 144 喵
fn next_fps(cur: u32) -> u32 {
    match cur {
        0 => 30,
        30 => 60,
        60 => 90,
        90 => 120,
        120 => 144,
        _ => 0,
    }
}

/// 帧率档位显示名喵
fn fps_label(v: u32) -> String {
    if v == 0 {
        "自动(屏刷)".into()
    } else {
        format!("{v} Hz")
    }
}

/// 排版模式显示名喵
fn app_layout_label(l: AppLayout) -> &'static str {
    match l {
        AppLayout::Grid => "网格",
        AppLayout::Row => "列表",
    }
}

fn sw(id: RowId, label: &str, value: bool) -> SettingsRow {
    SettingsRow::Switch {
        id,
        label: label.into(),
        value,
    }
}

/// 数值步进行: step = 每次点击调整幅度,unit = 单位喵
fn st(id: RowId, label: &str, value: i32, min: i32, max: i32, step: i32, unit: &str) -> SettingsRow {
    SettingsRow::Stepper {
        id,
        label: label.into(),
        value,
        min,
        max,
        step,
        unit: unit.into(),
    }
}

fn btn(id: RowId, label: String) -> SettingsRow {
    SettingsRow::Button { id, label }
}

/// 从配置构建页面模型喵(标签草稿 / 过滤草稿由绘制层的编辑焦点接管)喵
fn build_pages(
    config: &AppConfig,
    apps: &[crate::apps::AppInfo],
    selected: Option<&str>,
    hotkey_capture: bool,
) -> Vec<SettingsPage> {
    let mut app_rows: Vec<SettingsRow> = vec![
        btn(RowId::Rescan, "重新扫描应用".into()),
        btn(RowId::RemoveApp, "删除选中应用".into()),
        SettingsRow::Label {
            label: "已注册".into(),
            value: format!("{} 个 · 拖入 .lnk/.exe 即可注册喵", apps.len()),
        },
    ];
    for app in apps.iter().take(40) {
        app_rows.push(SettingsRow::AppPick {
            name: app.name.clone(),
            favorite: app.favorite,
            tags: if app.tags.is_empty() {
                app.path.clone()
            } else {
                app.tags.join(" / ")
            },
            selected: selected == Some(app.name.as_str()),
        });
    }

    let mut tag_rows = vec![SettingsRow::Input {
        id: RowId::TagInput,
        label: "给选中应用加 Tag".into(),
        value: String::new(),
    }];
    if let Some(name) = selected
        && let Some(app) = apps.iter().find(|a| a.name == name)
    {
        for tag in &app.tags {
            tag_rows.push(SettingsRow::Chip { label: tag.clone() });
        }
    }

    let island = &config.island;
    let mut spring_rows = Vec::new();
    for i in 0..6u8 {
        let t = spring_at(i);
        let tune: DurationBounce = island.springs.get(t);
        spring_rows.push(st(
            RowId::SpringDuration(i),
            &format!("{} 时长", spring_label(i)),
            (tune.duration * 100.0).round() as i32,
            5,
            200,
            10,
            "ms",
        ));
        spring_rows.push(st(
            RowId::SpringBounce(i),
            &format!("{} 弹性", spring_label(i)),
            (tune.bounce * 100.0).round() as i32,
            0,
            95,
            5,
            "%",
        ));
    }

    // 过滤关键词行喵(绘制层的编辑焦点负责草稿显示)喵
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
            groups: vec![
                SettingsGroup {
                    title: "外观".into(),
                    rows: vec![btn(
                        RowId::Theme,
                        format!("主题 · {}", config.theme.preset.label()),
                    )],
                },
                SettingsGroup {
                    title: "热键".into(),
                    rows: vec![
                        sw(RowId::HotkeyEnabled, "启用全局热键", config.hotkey.enabled),
                        btn(
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
                        ),
                    ],
                },
                SettingsGroup {
                    title: "显示".into(),
                    rows: vec![
                        sw(RowId::ShowRecent, "显示最近打开", config.window.show_recent),
                        sw(RowId::ShowFavorites, "显示收藏", config.window.show_favorites),
                        sw(RowId::ShowFrequent, "显示最常用", config.window.show_frequent),
                        sw(RowId::ShowAll, "始终显示全部", config.window.show_all),
                        sw(RowId::AlwaysOnTop, "始终置顶", config.window.always_on_top),
                        st(
                            RowId::IconSize,
                            "图标大小",
                            config.window.icon_size as i32,
                            24,
                            64,
                            2,
                            "px",
                        ),
                        btn(
                            RowId::SearchMode,
                            format!(
                                "默认模式 · {}",
                                match config.search.default_mode {
                                    SearchMode::Name => "名称",
                                    SearchMode::Tag => "标签 t:",
                                    SearchMode::Initial => "首字母 i:",
                                }
                            ),
                        ),
                        btn(
                            RowId::AppLayout,
                            format!("结果排版 · {}", app_layout_label(config.window.layout)),
                        ),
                    ],
                },
                SettingsGroup {
                    title: "维护".into(),
                    rows: vec![btn(RowId::Reset, "恢复默认设置".into())],
                },
                SettingsGroup {
                    title: "系统".into(),
                    rows: vec![
                        sw(RowId::AutoStart, "开机自启", config.auto_start),
                        btn(
                            RowId::RenderBackend,
                            format!("渲染后端 · {}", config.render_backend.label()),
                        ),
                    ],
                },
            ],
        },
        SettingsPage {
            title: "灵动岛".into(),
            subtitle: "胶囊几何 · 锚点 · 动效策略喵".into(),
            groups: vec![
                SettingsGroup {
                    title: "几何".into(),
                    rows: vec![
                        st(RowId::IslandW, "胶囊宽", island.width as i32, 160, 640, 4, "px"),
                        st(RowId::IslandH, "胶囊高", island.height as i32, 32, 80, 2, "px"),
                        st(RowId::IslandX, "水平锚点", island.x as i32, 2, 98, 1, "%"),
                        st(RowId::IslandY, "垂直锚点", island.y as i32, 2, 98, 1, "%"),
                        st(
                            RowId::ExpandedW,
                            "展开宽",
                            island.expanded_width as i32,
                            280,
                            900,
                            10,
                            "px",
                        ),
                        st(
                            RowId::ExpandedH,
                            "展开高",
                            island.expanded_height as i32,
                            160,
                            720,
                            8,
                            "px",
                        ),
                        st(
                            RowId::ExpandedR,
                            "展开圆角",
                            island.expanded_radius as i32,
                            8,
                            48,
                            2,
                            "px",
                        ),
                        st(
                            RowId::InputRatio,
                            "输入槽占比",
                            (island.input_ratio * 100.0).round() as i32,
                            20,
                            100,
                            1,
                            "%",
                        ),
                        st(RowId::Margin, "安全边距", island.margin as i32, 0, 80, 2, "px"),
                        st(
                            RowId::Squash,
                            "收起压扁",
                            (island.summon_squash * 100.0).round() as i32,
                            5,
                            100,
                            1,
                            "%",
                        ),
                    ],
                },
                SettingsGroup {
                    title: "行为".into(),
                    rows: vec![
                        sw(RowId::AutoMorph, "输入自动展开", island.auto_morph),
                        sw(RowId::Draggable, "允许拖拽定位", island.draggable),
                        sw(RowId::ReduceMotion, "减少动效", island.reduce_motion),
                        btn(
                            RowId::AnimFps,
                            format!("动画帧率 · {}", fps_label(island.anim_fps)),
                        ),
                        btn(
                            RowId::MotionMode,
                            format!("运动引擎 · {}", island.motion_mode.label()),
                        ),
                        btn(
                            RowId::Easing,
                            format!("缓动曲线 · {}", island.easing.label()),
                        ),
                    ],
                },
            ],
        },
        SettingsPage {
            title: "弹簧".into(),
            subtitle: "六段过渡的时长与弹性手感喵".into(),
            groups: vec![
                SettingsGroup {
                    title: "过渡参数".into(),
                    rows: spring_rows,
                },
            ],
        },
        SettingsPage {
            title: "应用".into(),
            subtitle: "注册管理 · 标签整理 · 关键词过滤喵".into(),
            groups: vec![
                SettingsGroup {
                    title: "关键词过滤".into(),
                    rows: filter_rows,
                },
                SettingsGroup {
                    title: "注册".into(),
                    rows: app_rows,
                },
                SettingsGroup {
                    title: "标签".into(),
                    rows: tag_rows,
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
