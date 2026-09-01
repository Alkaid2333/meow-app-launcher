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
use crate::render::font::FontCache;
use crate::render::settings::{
    paint_settings, RowHit, RowId, SETTINGS_HEIGHT, SETTINGS_WIDTH, SettingsGroup, SettingsLayout,
    SettingsPage, SettingsRow,
};
use crate::render::{Renderer, SettingsTheme};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

/// 心跳间隔(ms): 隐藏时低频检查显示请求喵
const HEARTBEAT_MS: u32 = 100;

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
    /// 标签输入草稿喵
    tag_draft: String,
    /// 是否正在编辑标签喵
    tag_focus: bool,
    /// 正在手动键入的数值行(id + 草稿 + 是否未编辑)喵
    stepper_edit: Option<(RowId, String, bool)>,
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

        let renderer = Renderer::new(win_w, win_h).unwrap_or_else(|| {
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
            tag_draft: String::new(),
            tag_focus: false,
            stepper_edit: None,
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
                &self.tag_draft,
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
        let layout = paint_settings(
            canvas,
            &theme,
            &self.fonts,
            &pages,
            self.current_page,
            self.scroll,
            self.stepper_edit.as_ref(),
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
            .map(|(_, h)| *h);

        if let Some(hit) = hit {
            match hit {
                RowHit::TitleBar => {
                    self.begin_window_drag(x, y, false);
                    self.render();
                }
                RowHit::ResizeGrip => {
                    self.begin_window_drag(x, y, true);
                    self.render();
                }
                _ => self.apply_hit(hit),
            }
        }
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

    /// 拖动窗口 / 调整窗口尺寸喵
    fn on_mouse_move(&mut self, x: f32, y: f32) {
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

    /// 处理命中动作喵
    fn apply_hit(&mut self, hit: RowHit) {
        match hit {
            RowHit::Nav(i) => {
                self.current_page = i;
                self.scroll = 0.0;
                self.stepper_edit = None;
                self.hotkey_capture = false;
            }
            RowHit::Close => self.hide(),
            RowHit::Switch(id) => self.toggle_switch(id),
            RowHit::StepperDec(id) => self.adjust_stepper(id, -1),
            RowHit::StepperInc(id) => self.adjust_stepper(id, 1),
            RowHit::StepperEdit(id) => self.handle_stepper_edit(id),
            RowHit::Button(id) => self.press_button(id),
            RowHit::Restore(id) => self.restore_row_defaults(id),
            RowHit::AppPick(i) => self.pick_app(i),
            RowHit::FavStar(i) => self.toggle_fav(i),
            RowHit::Chip(i) => self.remove_chip(i),
            RowHit::Input(_) => self.tag_focus = true,
            // 窗口栏/手柄在 handle_click 里先行拦截,这里仅收尾喵
            RowHit::TitleBar | RowHit::ResizeGrip => {}
        }
        self.render();
    }

    /// 进入数值手动键入喵: 以当前值作为草稿起点(键入即覆盖,免去先删旧值)喵
    fn begin_stepper_edit(&mut self, id: RowId) {
        let cur = self.stepper_value(id);
        self.stepper_edit = Some((id, cur.to_string(), true));
        log::debug!("数值键入开始: {id:?} 起点={cur} 喵");
    }

    /// 点击数值框: 首次点击进入键入(全选态),再次点击/拖动重新全选喵
    fn handle_stepper_edit(&mut self, id: RowId) {
        if self.stepper_edit.as_ref().is_some_and(|(eid, _, _)| *eid == id) {
            if let Some((_, _, pristine)) = self.stepper_edit.as_mut() {
                *pristine = true; // 重新全选,再键入整体替换喵
            }
        } else {
            self.begin_stepper_edit(id);
        }
    }

    /// 恢复单个配置项到默认值喵
    fn restore_row_defaults(&mut self, id: RowId) {
        // 正在键入同一行时先退出编辑态,避免草稿过期喵
        if self.stepper_edit.as_ref().is_some_and(|(eid, _, _)| *eid == id) {
            self.stepper_edit = None;
        }
        let mut state = self.state.borrow_mut();
        if restore_default(id, &mut state.config) {
            state.persist();
            log::info!("已恢复 {id:?} 到默认值喵~");
            if id == RowId::HotkeyEnabled {
                self.commands.borrow_mut().push_back(Command::ReapplyHotkey);
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
            _ => {}
        }
        state.persist();
        log::debug!("开关 {id:?} 已热写配置喵");
    }

    /// 步进调节喵: 按行配置的步幅增减喵
    fn adjust_stepper(&mut self, id: RowId, dir: i32) {
        // 若正在手动键入同一行,先退出键入态,避免草稿与最新值错位喵
        if self.stepper_edit.as_ref().is_some_and(|(eid, _, _)| *eid == id) {
            self.stepper_edit = None;
        }
        let Some((_, _, step)) = self.stepper_meta(id) else {
            return;
        };
        let cur = self.stepper_value(id) + dir * step;
        self.apply_stepper(id, cur);
    }

    /// 手动键入提交: 解析草稿 → 钳制 → 写配置喵
    fn commit_stepper_edit(&mut self) {
        let Some((id, draft, _)) = self.stepper_edit.take() else {
            return;
        };
        match draft.trim().parse::<i32>() {
            Ok(v) => {
                self.apply_stepper(id, v);
                log::debug!("数值键入提交: {id:?} = {v} 喵");
            }
            Err(_) => log::warn!("数值键入非法,已放弃: {id:?} = {draft:?} 喵"),
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
                // 重置后重新注册热键 + 清空各编辑态喵
                self.commands.borrow_mut().push_back(Command::ReapplyHotkey);
                self.stepper_edit = None;
                self.tag_focus = false;
                self.tag_draft.clear();
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
            _ => {}
        }
    }

    fn pick_app(&mut self, index: usize) {
        self.selected_app = self.app_name_at(index);
        self.tag_draft.clear();
        self.tag_focus = false;
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

    fn on_char(&mut self, ch: char) {
        // 数值键入中: 只收数字与负号;首次输入直接覆盖旧值(选中即替换)喵
        if self.stepper_edit.is_some() {
            if ch.is_ascii_digit() || ch == '-' {
                if let Some((_, draft, pristine)) = self.stepper_edit.as_mut() {
                    if *pristine {
                        draft.clear();
                        *pristine = false;
                    }
                    // 只允许开头的负号喵
                    if ch == '-' && !draft.is_empty() {
                        return;
                    }
                    if draft.len() < 8 {
                        draft.push(ch);
                    }
                    self.render();
                }
            }
            return;
        }
        if !self.tag_focus || ch.is_control() {
            return;
        }
        self.tag_draft.push(ch);
        self.render();
    }

    fn on_key(&mut self, key: crate::platform::Key) {
        // 数值键入中的按键喵
        if self.stepper_edit.is_some() {
            match key {
                crate::platform::Key::Backspace => {
                    if let Some((_, draft, pristine)) = self.stepper_edit.as_mut() {
                        draft.pop();
                        *pristine = false;
                    }
                    self.render();
                }
                crate::platform::Key::Enter => self.commit_stepper_edit(),
                crate::platform::Key::Escape => {
                    self.stepper_edit = None;
                    self.render();
                }
                _ => {}
            }
            return;
        }
        if !self.tag_focus {
            return;
        }
        match key {
            crate::platform::Key::Backspace => {
                self.tag_draft.pop();
                self.render();
            }
            crate::platform::Key::Enter => {
                if let Some(name) = self.selected_app.clone() {
                    self.state.borrow_mut().add_tag(&name, &self.tag_draft);
                    self.tag_draft.clear();
                    self.render();
                }
            }
            crate::platform::Key::Escape => {
                self.tag_focus = false;
                self.render();
            }
            _ => {}
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
            self.state.borrow_mut().register_app(&name, &path, None);
        }
        self.render();
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
            WindowEvent::MouseUp => self.win_drag = None,
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

/// 从配置构建页面模型喵
fn build_pages(
    config: &AppConfig,
    apps: &[crate::apps::AppInfo],
    selected: Option<&str>,
    tag_draft: &str,
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
        value: tag_draft.into(),
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
            subtitle: "注册管理 · 标签整理喵".into(),
            groups: vec![
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
