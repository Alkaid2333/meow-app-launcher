//! 配置窗口喵~
//!
//! 依据 window-design skill 的配置 GUI 设计,实现「侧边栏 + 分组卡片」设置界面喵。
//! 持有共享状态,修改配置项并持久化;交互通过命中测试驱动喵。
//!
//! 状态机: 常驻(隐藏) → 收到 `settings_visible` 标志 → 显示并渲染 → 红点关闭喵。

use crate::animation::{DurationBounce, IslandTransition};
use crate::app::config::{AppConfig, SearchMode, ThemeMode};
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
            )
        };
        let theme = {
            let state = self.state.borrow();
            SettingsTheme::for_mode(state.config.theme.mode)
        };

        let canvas = self.renderer.canvas();
        canvas.save();
        canvas.scale((self.scale, self.scale));
        let layout = paint_settings(canvas, &theme, &self.fonts, &pages, self.current_page, self.scroll);
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
            self.apply_hit(hit);
        }
    }

    /// 处理命中动作喵
    fn apply_hit(&mut self, hit: RowHit) {
        match hit {
            RowHit::Nav(i) => {
                self.current_page = i;
                self.scroll = 0.0;
            }
            RowHit::TrafficLight(0) => self.hide(),
            RowHit::TrafficLight(1) => self.platform.minimize_window(&self.window),
            RowHit::TrafficLight(_) => {}
            RowHit::Switch(id) => self.toggle_switch(id),
            RowHit::StepperDec(id) => self.adjust_stepper(id, -1),
            RowHit::StepperInc(id) => self.adjust_stepper(id, 1),
            RowHit::Button(id) => self.press_button(id),
            RowHit::AppPick(i) => self.pick_app(i),
            RowHit::FavStar(i) => self.toggle_fav(i),
            RowHit::Chip(i) => self.remove_chip(i),
            RowHit::Input(_) => self.tag_focus = true,
        }
        self.render();
    }

    /// 切换开关喵
    fn toggle_switch(&mut self, id: RowId) {
        let mut state = self.state.borrow_mut();
        match id {
            RowId::DarkMode => {
                state.config.theme.mode = if state.config.theme.mode == ThemeMode::Dark {
                    ThemeMode::Light
                } else {
                    ThemeMode::Dark
                };
            }
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

    /// 步进调节喵
    fn adjust_stepper(&mut self, id: RowId, delta: i32) {
        let (min, max) = self.stepper_range(id);
        let mut state = self.state.borrow_mut();
        let island = &mut state.config.island;
        let changed = match id {
            RowId::IslandW => step_f64(&mut island.width, delta, min, max),
            RowId::IslandH => step_f64(&mut island.height, delta, min, max),
            RowId::IslandX => step_f64(&mut island.x, delta, min, max),
            RowId::IslandY => step_f64(&mut island.y, delta, min, max),
            RowId::ExpandedW => {
                let ok = step_f64(&mut island.expanded_width, delta, min, max);
                state.config.window.width = island.expanded_width;
                ok
            }
            RowId::ExpandedH => {
                let ok = step_f64(&mut island.expanded_height, delta, min, max);
                state.config.window.height = island.expanded_height;
                ok
            }
            RowId::ExpandedR => step_f64(&mut island.expanded_radius, delta, min, max),
            RowId::InputRatio => {
                let mut v = (island.input_ratio * 100.0).round();
                let ok = step_f64(&mut v, delta, min, max);
                island.input_ratio = v / 100.0;
                ok
            }
            RowId::Margin => step_f64(&mut island.margin, delta, min, max),
            RowId::Squash => {
                let mut v = (island.summon_squash * 100.0).round();
                let ok = step_f64(&mut v, delta, min, max);
                island.summon_squash = v / 100.0;
                ok
            }
            RowId::IconSize => step_value(&mut state.config.window.icon_size, delta, min, max),
            RowId::SpringDuration(i) => {
                let t = spring_at(i);
                let tune = island.springs.get_mut(t);
                let mut v = (tune.duration * 100.0).round();
                let ok = step_f64(&mut v, delta, min, max);
                tune.duration = v / 100.0;
                ok
            }
            RowId::SpringBounce(i) => {
                let t = spring_at(i);
                let tune = island.springs.get_mut(t);
                let mut v = (tune.bounce * 100.0).round();
                let ok = step_f64(&mut v, delta, min, max);
                tune.bounce = (v / 100.0).clamp(0.0, 0.95);
                ok
            }
            _ => false,
        };
        if changed {
            state.persist();
        }
    }

    fn stepper_range(&self, id: RowId) -> (f32, f32) {
        let Some(page) = self.pages.get(self.current_page) else {
            return (0.0, 0.0);
        };
        for group in &page.groups {
            for row in &group.rows {
                if let SettingsRow::Stepper {
                    id: rid, min, max, ..
                } = row
                    && *rid == id
                {
                    return (*min as f32, *max as f32);
                }
            }
        }
        (0.0, 0.0)
    }

    /// 按钮动作喵
    fn press_button(&mut self, id: RowId) {
        match id {
            RowId::Backdrop => {
                let mut state = self.state.borrow_mut();
                state.config.theme.backdrop = state.config.theme.backdrop.cycle();
                state.persist();
                log::info!("浮窗材质 → {} 喵", state.config.theme.backdrop.label());
            }
            RowId::Visual => {
                let mut state = self.state.borrow_mut();
                state.config.island.visual = state.config.island.visual.cycle();
                state.persist();
                log::info!("岛视觉 → {} 喵", state.config.island.visual.label());
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
        if !self.tag_focus || ch.is_control() {
            return;
        }
        self.tag_draft.push(ch);
        self.render();
    }

    fn on_key(&mut self, key: crate::platform::Key) {
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
            WindowEvent::ImePreedit(_) => {}
            WindowEvent::FilesDropped(paths) => self.drop_files(paths),
            WindowEvent::MouseMove(_, _) | WindowEvent::MouseUp => {}
            WindowEvent::MouseWheel(delta) => {
                if self.visible {
                    // 滚轮滚动内容区,钳制到有效范围喵(每格滚动一行)喵
                    let max_scroll = self
                        .layout
                        .as_ref()
                        .map(|l| (l.content_height - SETTINGS_HEIGHT).max(0.0))
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

/// 步进值并钳制范围,返回是否变化喵
fn step_value(value: &mut f32, delta: i32, min: f32, max: f32) -> bool {
    let new_value = (*value + delta as f32).clamp(min, max);
    if (new_value - *value).abs() < f32::EPSILON {
        return false;
    }
    *value = new_value;
    true
}

fn step_f64(value: &mut f64, delta: i32, min: f32, max: f32) -> bool {
    let new_value = (*value + delta as f64).clamp(min as f64, max as f64);
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
    spring_at(i).label()
}

fn sw(id: RowId, label: &str, value: bool) -> SettingsRow {
    SettingsRow::Switch {
        id,
        label: label.into(),
        value,
    }
}

fn st(id: RowId, label: &str, value: i32, min: i32, max: i32) -> SettingsRow {
    SettingsRow::Stepper {
        id,
        label: label.into(),
        value,
        min,
        max,
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
            &format!("{} 时长 ms", spring_label(i)),
            (tune.duration * 100.0).round() as i32,
            5,
            200,
        ));
        spring_rows.push(st(
            RowId::SpringBounce(i),
            &format!("{} 弹性 %", spring_label(i)),
            (tune.bounce * 100.0).round() as i32,
            0,
            95,
        ));
    }

    vec![
        SettingsPage {
            title: "巢穴".into(),
            groups: vec![
                SettingsGroup {
                    title: "外观".into(),
                    rows: vec![
                        sw(RowId::DarkMode, "墨夜模式", config.theme.mode == ThemeMode::Dark),
                        btn(
                            RowId::Backdrop,
                            format!("浮窗材质 · {}", config.theme.backdrop.label()),
                        ),
                        btn(
                            RowId::Visual,
                            format!("岛皮肤 · {}", island.visual.label()),
                        ),
                    ],
                },
                SettingsGroup {
                    title: "热键".into(),
                    rows: vec![
                        sw(RowId::HotkeyEnabled, "启用全局热键", config.hotkey.enabled),
                        SettingsRow::Label {
                            label: "热键组合".into(),
                            value: format!("{}+{}", config.hotkey.modifiers, config.hotkey.key),
                        },
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
                    ],
                },
            ],
        },
        SettingsPage {
            title: "灵动岛".into(),
            groups: vec![
                SettingsGroup {
                    title: "几何".into(),
                    rows: vec![
                        st(RowId::IslandW, "胶囊宽", island.width as i32, 160, 640),
                        st(RowId::IslandH, "胶囊高", island.height as i32, 32, 80),
                        st(RowId::IslandX, "水平锚点 %", island.x as i32, 2, 98),
                        st(RowId::IslandY, "垂直锚点 %", island.y as i32, 2, 98),
                        st(
                            RowId::ExpandedW,
                            "展开宽",
                            island.expanded_width as i32,
                            280,
                            900,
                        ),
                        st(
                            RowId::ExpandedH,
                            "展开高",
                            island.expanded_height as i32,
                            160,
                            720,
                        ),
                        st(
                            RowId::ExpandedR,
                            "展开圆角",
                            island.expanded_radius as i32,
                            8,
                            48,
                        ),
                        st(
                            RowId::InputRatio,
                            "输入槽占比 %",
                            (island.input_ratio * 100.0) as i32,
                            20,
                            100,
                        ),
                        st(RowId::Margin, "安全边距", island.margin as i32, 0, 80),
                        st(
                            RowId::Squash,
                            "收起压扁 %",
                            (island.summon_squash * 100.0) as i32,
                            5,
                            100,
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
            groups: vec![SettingsGroup {
                title: "六段过渡 (时长×10ms / 弹性%)".into(),
                rows: spring_rows,
            }],
        },
        SettingsPage {
            title: "应用".into(),
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
            groups: vec![SettingsGroup {
                title: "关于".into(),
                rows: vec![
                    SettingsRow::Label {
                        label: "版本".into(),
                        value: env!("CARGO_PKG_VERSION").into(),
                    },
                    SettingsRow::Label {
                        label: "项目".into(),
                        value: "meow-app-launcher".into(),
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
