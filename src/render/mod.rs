//! 渲染层喵~
//!
//! 基于 Skia 的绘制管线,支持两种后端喵:
//! * CPU 光栅(默认,兼容性最好)喵
//! * GPU 加速(Windows 走 OpenGL/WGL,初始化失败自动回退 CPU)喵
//!
//! 其它模块:
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

use crate::app::config::RenderBackend;
use crate::platform::{GpuContext, Platform};
use skia_safe::gpu::{self, DirectContext, SurfaceOrigin};
use skia_safe::image::CachingHint;
use skia_safe::surfaces;
use skia_safe::{AlphaType, Canvas, ColorType, ImageInfo, Surface};

/// 实际生效的渲染后端喵(配置请求 GPU 但初始化失败时,回退为 CPU)喵
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// CPU 光栅喵
    Cpu,
    /// GPU 加速喵
    Gpu,
}

impl RenderMode {
    /// 给日志/界面显示的名字喵
    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Gpu => "GPU",
        }
    }
}

/// Skia 渲染器喵: 管理 Surface 生命周期与像素输出喵
///
/// GPU 模式额外持有 `DirectContext` 与平台 `GpuContext`,
/// 绘制完成后先提交命令再读回像素喵。
pub struct Renderer {
    /// 实际渲染后端喵
    mode: RenderMode,
    surface: Surface,
    /// GPU 渲染上下文喵(仅 GPU 模式持有,负责加载 GL 函数 + 维持 current)喵
    context: Option<DirectContext>,
    /// 平台 GPU 上下文喵(负责 WGL 生命周期,须先于 context 释放)喵
    gpu: Option<Box<dyn GpuContext>>,
    width: i32,
    height: i32,
}

impl Renderer {
    /// 创建指定尺寸的渲染器喵
    ///
    /// 请求 GPU 时优先尝试 GPU 后端,失败自动回退 CPU,
    /// 保证任何环境都能正常出画面喵。
    pub fn new(width: i32, height: i32, backend: RenderBackend, platform: &dyn Platform) -> Option<Self> {
        match backend {
            RenderBackend::Cpu => Self::new_cpu(width, height),
            RenderBackend::Gpu => match Self::new_gpu(width, height, platform) {
                Some(renderer) => {
                    log::info!("GPU 渲染后端已启用喵~");
                    Some(renderer)
                }
                None => {
                    log::warn!("GPU 初始化失败,回退 CPU 渲染喵~");
                    Self::new_cpu(width, height)
                }
            },
        }
    }

    /// CPU 光栅后端喵
    fn new_cpu(width: i32, height: i32) -> Option<Self> {
        let surface = surfaces::raster_n32_premul((width, height))?;
        Some(Self {
            mode: RenderMode::Cpu,
            surface,
            context: None,
            gpu: None,
            width,
            height,
        })
    }

    /// GPU 后端喵(Windows 走平台层 WGL 上下文 + OpenGL)喵
    fn new_gpu(width: i32, height: i32, platform: &dyn Platform) -> Option<Self> {
        #[cfg(feature = "gl")]
        {
            let gpu = platform.create_gpu_context()?;
            if !gpu.valid() || !gpu.make_current() {
                log::warn!("GPU: 平台上下文无效或无法 current 喵");
                return None;
            }
            // 通过平台上下文加载 GL 函数(上下文已 current,可拿到 ICD 函数)喵
            let interface = {
                let gpu = &*gpu;
                skia_safe::gpu::gl::Interface::new_load_with(|name| gpu.get_proc(name))?
            };
            if !interface.validate() {
                log::warn!("OpenGL 接口校验失败,无法启用 GPU 喵");
                return None;
            }
            let mut context = skia_safe::gpu::direct_contexts::make_gl(interface, None)?;
            let info = ImageInfo::new(
                (width, height),
                ColorType::BGRA8888,
                AlphaType::Premul,
                None,
            );
            let surface = gpu::surfaces::render_target(
                &mut context,
                gpu::Budgeted::Yes,
                &info,
                0,
                SurfaceOrigin::TopLeft,
                None,
                false,
                false,
            )?;
            Some(Self {
                mode: RenderMode::Gpu,
                surface,
                context: Some(context),
                gpu: Some(gpu),
                width,
                height,
            })
        }
        #[cfg(not(feature = "gl"))]
        {
            let _ = (width, height, platform);
            None
        }
    }

    /// 调整渲染尺寸喵(尺寸不变时跳过重建)喵
    pub fn resize(&mut self, width: i32, height: i32) {
        if width == self.width && height == self.height {
            return;
        }
        let rebuilt = if let Some(context) = &mut self.context {
            let info = ImageInfo::new(
                (width, height),
                ColorType::BGRA8888,
                AlphaType::Premul,
                None,
            );
            gpu::surfaces::render_target(
                context,
                gpu::Budgeted::Yes,
                &info,
                0,
                SurfaceOrigin::TopLeft,
                None,
                false,
                false,
            )
        } else {
            surfaces::raster_n32_premul((width, height))
        };
        if let Some(surface) = rebuilt {
            self.surface = surface;
            self.width = width;
            self.height = height;
        }
    }

    /// 当前实际生效的渲染后端喵
    pub fn mode(&self) -> RenderMode {
        self.mode
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
    /// GPU 模式下先确保上下文 current、提交渲染命令并同步 CPU,
    /// 再通过 DirectContext 显式读回(否则 GPU 图像读回会失败)喵。
    /// 返回是否读取成功;像素写入 `out` 喵。
    pub fn read_bgra(&mut self, out: &mut Vec<u8>) -> bool {
        if let Some(gpu) = &self.gpu {
            let _ = gpu.make_current();
        }
        let info = ImageInfo::new(
            (self.width, self.height),
            ColorType::BGRA8888,
            AlphaType::Premul,
            None,
        );
        let row_bytes = (self.width * 4) as usize;
        out.resize(row_bytes * self.height as usize, 0);

        if let Some(context) = &mut self.context {
            // 提交 GPU 命令并同步回 CPU,确保读回数据有效喵
            context.flush_submit_and_sync_cpu();
            let image = self.surface.image_snapshot();
            image.read_pixels_with_context(
                &mut *context,
                &info,
                out.as_mut_slice(),
                row_bytes,
                (0, 0),
                CachingHint::Allow,
            )
        } else {
            self.surface.read_pixels(&info, out, row_bytes, (0, 0))
        }
    }
}
