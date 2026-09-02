//! 渲染层喵~
//!
//! 基于 Skia 光栅渲染的绘制管线喵:
//! * `Renderer` 管理 Skia Surface 生命周期与像素输出喵
//! * `Layout` 计算物理像素布局(纯函数,可单测)喵
//! * `Theme` 语义色板喵
//! * `shape` 连续曲率圆角几何喵
//! * `paint` 场景绘制喵

pub mod font;
pub mod layout;
pub mod paint;
pub mod settings;
pub mod shape;
pub mod svg;
pub mod text;
pub mod theme;
pub mod edit;

pub use font::FontCache;
pub use layout::Layout;
pub use paint::paint_scene;
pub use theme::{SettingsTheme, Theme};

use skia_safe::{AlphaType, Canvas, ColorType, ImageInfo, Surface, image::CachingHint, surfaces};

/// Skia 光栅渲染器喵
pub struct Renderer {
    surface: Surface,
    width: i32,
    height: i32,
}

impl Renderer {
    /// 创建指定尺寸的渲染器喵
    pub fn new(width: i32, height: i32) -> Option<Self> {
        let surface = surfaces::raster_n32_premul((width, height))?;
        Some(Self {
            surface,
            width,
            height,
        })
    }

    /// 调整渲染尺寸喵(尺寸不变时跳过重建)喵
    pub fn resize(&mut self, width: i32, height: i32) {
        if width == self.width && height == self.height {
            return;
        }
        if let Some(surface) = surfaces::raster_n32_premul((width, height)) {
            self.surface = surface;
            self.width = width;
            self.height = height;
        }
    }

    /// 获取绘制画布喵(Skia Canvas 方法均为 &self,内部可变)喵
    pub fn canvas(&mut self) -> &Canvas {
        self.surface.canvas()
    }

    /// 当前宽度(px)喵
    pub fn width(&self) -> i32 {
        self.width
    }

    /// 当前高度(px)喵
    pub fn height(&self) -> i32 {
        self.height
    }

    /// 读取 BGRA 像素(供平台层 present)喵
    ///
    /// 返回是否读取成功;像素写入 `out` 喵。
    pub fn read_bgra(&mut self, out: &mut Vec<u8>) -> bool {
        let image = self.surface.image_snapshot();
        let info = ImageInfo::new(
            (self.width, self.height),
            ColorType::BGRA8888,
            AlphaType::Premul,
            None,
        );
        let row_bytes = (self.width * 4) as usize;
        out.resize(row_bytes * self.height as usize, 0);
        image.read_pixels(&info, out, row_bytes, (0, 0), CachingHint::Allow)
    }
}
