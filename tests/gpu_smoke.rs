//! 渲染冒烟测试喵~ 验证 CPU/GPU 两个后端都能出正确像素喵
//!
//! 注意: GPU 后端在无 GL 驱动的环境会自动回退 CPU,因此本测试不断言具体后端,
//! 只断言两种配置下渲染结果都有有效像素(不透明)喵。

use meow_app_launcher::app::config::RenderBackend;
use meow_app_launcher::platform::platform;
use meow_app_launcher::render::Renderer;
use skia_safe::{Color, Paint};

/// 用指定后端渲染一个满屏红色矩形,读回像素校验喵
fn render_smoke(backend: RenderBackend) {
    let platform = platform();
    let mut renderer = Renderer::new(64, 48, backend, &*platform)
        .expect("渲染器初始化失败喵~");
    println!("后端 {} 实际生效: {} 喵", backend.label(), renderer.mode().label());

    let canvas = renderer.canvas();
    canvas.clear(Color::TRANSPARENT);
    let mut paint = Paint::default();
    paint.set_color(Color::RED);
    canvas.draw_rect(
        skia_safe::Rect::from_xywh(0.0, 0.0, 64.0, 48.0),
        &paint,
    );

    let mut pixels = Vec::new();
    assert!(renderer.read_bgra(&mut pixels), "像素读回失败喵~");
    assert_eq!(pixels.len(), 64 * 48 * 4, "像素缓冲尺寸不对喵");

    // 中心像素应为红色(BGRA 布局: B=0, G=0, R=255, A=255)喵
    let mid = (64 * 24 + 32) * 4;
    let (b, g, r, a) = (pixels[mid], pixels[mid + 1], pixels[mid + 2], pixels[mid + 3]);
    assert_eq!(a, 255, "中心像素应不透明,实际 alpha={a} 喵");
    assert!(r > 200 && g < 60 && b < 60, "中心像素应为红色,实际 RGBA=({r},{g},{b},{a}) 喵");
}

#[test]
fn cpu_backend_renders_correct_pixels() {
    render_smoke(RenderBackend::Cpu);
}

#[test]
fn gpu_backend_renders_correct_pixels() {
    render_smoke(RenderBackend::Gpu);
}
