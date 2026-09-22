//! 配置 GUI / 应用管理中心离屏渲染快照喵~
//!
//! 不开窗口、不碰桌面,直接用 Skia 把各页面渲染成 PNG 落到 `.tmp/snapshots/`,
//! 供人工目检排版喵(裁剪边界 / 滚动条 / 行密度全都能看)喵。
//!
//! 运行: `cargo run --release --example render_snapshot` 喵。

use meow_app_launcher::app::config::AppConfig;
use meow_app_launcher::apps::{AppInfo, AppSource};
use meow_app_launcher::render::app_manager::{paint_app_manager, ManagerEdits, MANAGER_HEIGHT, MANAGER_WIDTH};
use meow_app_launcher::render::font::FontCache;
use meow_app_launcher::render::settings::{
    paint_settings, SETTINGS_HEIGHT, SETTINGS_WIDTH,
};
use meow_app_launcher::render::theme::SettingsTheme;
use meow_app_launcher::app::config::ThemePreset;
use meow_app_launcher::window::settings::build_pages;
use skia_safe::{surfaces, EncodedImageFormat};

fn main() {
    let out_dir = std::path::Path::new(".tmp/snapshots");
    let _ = std::fs::create_dir_all(out_dir);

    // 典型配置 + 一批注册应用喵
    let mut config = AppConfig::default();
    // 塞一条自定义指令,让指令卡片的 Custom 档(带脚本输入框)进快照喵
    config.search.commands.push(meow_app_launcher::app::config::CommandEntry::custom(
        vec!["清DNS".into()],
        "ipconfig /flushdns",
    ));
    let apps: Vec<AppInfo> = ["喵喵终端", "Rust Rover", "Visual Studio Code", "Firefox", "微信", "QQ音乐"]
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let mut app = AppInfo::manual(*name, format!("C:/apps/demo-{}.lnk", i));
            app.source = AppSource::Scanned;
            app.tags = if i % 2 == 0 { vec!["开发".into()] } else { vec![] };
            app
        })
        .collect();

    let fonts = FontCache::new();
    let scale = 2.0f32; // 200% DPI 喵

    let pages = build_pages(&config, None);
    let dirty: Vec<_> = meow_app_launcher::window::settings_data::dirty_data_ids(&config);

    // 浅色(默认)与深色主题各出一组配置页快照喵
    let themes = [
        ("light", SettingsTheme::for_preset(config.theme.preset)),
        ("dark", SettingsTheme::for_preset(ThemePreset::Dark)),
    ];
    for (theme_name, theme) in themes {
    for (page_idx, scroll) in [(0usize, 0.0f32), (0, 320.0), (0, 1250.0), (1, 0.0), (3, 0.0)] {
        let (w, h) = (SETTINGS_WIDTH, SETTINGS_HEIGHT);
        let mut surface = surfaces::raster_n32_premul(((w * scale) as i32, (h * scale) as i32)).unwrap();
        let canvas = surface.canvas();
        canvas.save();
        canvas.scale((scale, scale));
        let layout = paint_settings(
            canvas,
            &theme,
            &fonts,
            &pages,
            page_idx,
            scroll,
            1.0,
            Default::default(),
            None,
            &dirty,
            w,
            h,
        );
        canvas.restore();
        let path = out_dir.join(format!("settings_{theme_name}_p{page_idx}_s{scroll}.png"));
        save_png(&mut surface, &path);
        println!("内容高度 {:.0} → {}", layout.content_height, path.display());
    }
    }

    // 应用管理中心喵(行排版 + 网格排版,浅色/深色各一组)喵
    for (theme_name, theme) in themes {
        for grid in [false, true] {
            let mut surface =
                surfaces::raster_n32_premul(((MANAGER_WIDTH * scale) as i32, (MANAGER_HEIGHT * scale) as i32))
                    .unwrap();
            let canvas = surface.canvas();
            canvas.save();
            canvas.scale((scale, scale));
            let mut icon_of = |_app: &AppInfo| -> Option<skia_safe::Image> { None };
            let layout = paint_app_manager(
                canvas,
                &theme,
                &fonts,
                &apps,
                apps.len(),
                Some("Rust Rover"),
                grid,
                0.0,
                ManagerEdits::default(),
                "",
                &["卸载".to_string(), "uninstall".to_string()],
                &mut icon_of,
                MANAGER_WIDTH,
                MANAGER_HEIGHT,
            );
            canvas.restore();
            let path = out_dir.join(format!("manager_{theme_name}_{}.png", if grid { "grid" } else { "row" }));
            save_png(&mut surface, &path);
            println!("列表内容高度 {:.0} → {}", layout.list_content_height, path.display());
        }
    }
}

/// Surface → PNG 落盘喵
fn save_png(surface: &mut skia_safe::Surface, path: &std::path::Path) {
    use skia_safe::ImageInfo;
    let (w, h) = (surface.width(), surface.height());
    let info = ImageInfo::new((w, h), skia_safe::ColorType::BGRA8888, skia_safe::AlphaType::Premul, None);
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    if !surface.read_pixels(&info, &mut pixels, (w * 4) as usize, (0, 0)) {
        panic!("读回像素失败: {}", path.display());
    }
    let image = skia_safe::images::raster_from_data(&info, skia_safe::Data::new_copy(&pixels), (w * 4) as usize)
        .expect("构造图像失败");
    // encode_to_data 的 deprecation 仅针对 GPU 图像,此处是 CPU 光栅图像(同 apps/icon.rs)喵
    #[allow(deprecated)]
    let encoded = image.encode_to_data(EncodedImageFormat::PNG).expect("PNG 编码失败");
    std::fs::write(path, encoded.as_bytes()).expect("写 PNG 失败");
}
