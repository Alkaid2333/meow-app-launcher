//! 喵喵应用启动器喵~ (meowal)
//!
//! 类 macOS 聚焦搜索的跨平台高性能应用启动器喵!
//! 双窗口架构: 灵动岛胶囊搜索框 + 分离的选择窗口喵。

mod app;
mod animation;
mod apps;
mod platform;
mod search;
mod ui;
mod utils;
mod views;

use app::AppState;
use gpui::*;
use gpui_component::Root;
use std::path::PathBuf;
use views::{ResultsView, SearchBarView};

/// 搜索框尺寸(px)喵
const BAR_WIDTH: f32 = 420.0;
const BAR_HEIGHT: f32 = 44.0;
/// 搜索框在屏幕中的垂直位置比例(距顶部)喵
const BAR_TOP_RATIO: f32 = 0.18;

fn main() {
    // 数据目录: ./.datas 喵
    let data_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(app::config::DATA_DIR_NAME);

    // 日志喵(MEOWAL_VERBOSE=1 开启 debug 日志)喵
    utils::logger::init(&data_dir, std::env::var("MEOWAL_VERBOSE").is_ok());
    log::info!("喵喵启动器启动喵! 版本 {}", env!("CARGO_PKG_VERSION"));

    Application::new().run(|cx| {
        // gpui-component 初始化喵
        gpui_component::init(cx);

        // 自绘窗口关键修复喵~
        // gpui-component 的 `Root` 会把整窗刷一层不透明主题背景 + 描边,
        // 于是呼出时看到的是一块「大边框带背景的窗口」,搜索框/选择窗被嵌在里面。
        // 这里把主题背景与窗口描边都置为透明,`Root` 就退化成纯透明画布,
        // 视觉上只剩我们自己绘制的胶囊搜索框与结果面板喵。
        {
            let theme = gpui_component::Theme::global_mut(cx);
            theme.colors.background = gpui::transparent_black();
            theme.colors.window_border = gpui::transparent_black();
        }

        // 全局应用状态喵
        let mut state = AppState::new(data_dir);
        if state.registry.apps.is_empty() {
            log::info!("首次启动,自动扫描系统应用喵~");
            state.rescan_apps();
        }
        cx.set_global(state);

        // 主屏尺寸,用于计算窗口位置喵
        let display = cx.primary_display();
        let (screen_x, screen_y, screen_w, screen_h) = display
            .map(|d| {
                let b = d.bounds();
                (
                    f32::from(b.origin.x),
                    f32::from(b.origin.y),
                    f32::from(b.size.width),
                    f32::from(b.size.height),
                )
            })
            .unwrap_or((0.0, 0.0, 1920.0, 1080.0));

        let cfg = cx.global::<AppState>().config.clone();

        // ---------- 搜索框窗口(灵动岛胶囊,屏中上方)喵 ----------
        let sb_bounds = Bounds::new(
            Point {
                x: px(screen_x + (screen_w - BAR_WIDTH) / 2.0),
                y: px(screen_y + screen_h * BAR_TOP_RATIO),
            },
            Size {
                width: px(BAR_WIDTH),
                height: px(BAR_HEIGHT),
            },
        );
        let mut searchbar_entity: Option<gpui::Entity<SearchBarView>> = None;
        let searchbar_win = cx.open_window(
            window_options(sb_bounds),
            |window, cx| {
                let wh = window.window_handle();
                let view = cx.new(|cx| SearchBarView::new(wh, window, cx));
                searchbar_entity = Some(view.clone());
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        // 搜索框位置是确定的(创建时计算),直接存入全局供选择窗口对齐喵
        cx.global_mut::<AppState>().searchbar_bounds = Some((
            f32::from(sb_bounds.origin.x),
            f32::from(sb_bounds.origin.y),
            f32::from(sb_bounds.size.width),
            f32::from(sb_bounds.size.height),
        ));

        // ---------- 选择窗口(初始隐藏,显示时定位)喵 ----------
        let rs_bounds = Bounds::new(
            Point {
                x: px(screen_x + (screen_w - cfg.window.width) / 2.0),
                y: px(screen_y + screen_h * 0.3),
            },
            Size {
                width: px(cfg.window.width),
                height: px(cfg.window.height),
            },
        );
        let mut results_entity: Option<gpui::Entity<ResultsView>> = None;
        let results_win = cx.open_window(
            window_options(rs_bounds),
            |window, cx| {
                let wh = window.window_handle();
                let view = cx.new(|cx| ResultsView::new(wh, window, cx));
                results_entity = Some(view.clone());
                cx.new(|cx| Root::new(view, window, cx))
            },
        );

        // ---------- 建立双窗口联动喵 ----------
        if let (Some(sv), Some(rv), Ok(sw), Ok(rw)) = (
            &searchbar_entity,
            &results_entity,
            &searchbar_win,
            &results_win,
        ) {
            let _ = sw.read(cx);
            let _ = rw.read(cx);
            let rwh = rv.read(cx).window_handle();
            let swh = sv.read(cx).window_handle();
            let _ = sv.update(cx, |view, cx| view.link_results(rwh, rv.clone(), cx));
            let _ = rv.update(cx, |view, cx| view.link_searchbar(swh, sv.clone(), cx));
        }
    });
}

/// 构建浮窗窗口选项喵: 透明、无边框、置顶弹窗、初始隐藏喵
fn window_options(bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        // 初始隐藏,热键呼出喵
        show: false,
        focus: false,
        // 置顶弹窗类型喵
        kind: WindowKind::PopUp,
        is_movable: true,
        is_resizable: false,
        is_minimizable: false,
        // 隐藏系统标题栏(自绘喵)
        titlebar: Some(TitlebarOptions {
            appears_transparent: true,
            ..TitlebarOptions::default()
        }),
        // 透明背景 + 无边框(自绘喵)
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
        app_id: Some("meowal".into()),
        ..WindowOptions::default()
    }
}
