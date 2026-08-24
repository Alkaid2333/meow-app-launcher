//! 动画内核喵~
//!
//! 纯 Rust 数学库,不依赖 GPUI、不依赖平台,可单测喵!
//! 渲染层只消费这里的数值结果(弹簧值、alpha),不自己算动画喵。

pub mod springs;

pub use springs::Spring;

/// 通用面板动画弹簧集喵~
///
/// gpui 0.2.2 没有 transform/scale 样式,所以:
/// * `alpha` 驱动透明度喵
/// * `scale` 驱动面板尺寸(营造缩放感)喵
/// * `offset` 驱动垂直位移(用 margin-top 模拟)喵
#[derive(Debug)]
pub struct PanelSprings {
    /// 整体透明度喵
    pub alpha: Spring,
    /// 尺寸缩放(映射到宽高)喵
    pub scale: Spring,
    /// 垂直位移(映射到 margin-top)喵
    pub offset: Spring,
}

impl PanelSprings {
    pub fn new() -> Self {
        Self {
            alpha: Spring::new(0.0),
            scale: Spring::new(0.96),
            offset: Spring::new(-12.0),
        }
    }

    /// 推进一帧喵,返回是否还在动画中喵
    ///
    /// * `dt` - 归一化步长(60fps 基准)喵
    /// * `visible` - 目标是否可见喵
    pub fn tick(&mut self, dt: f32, visible: bool) -> bool {
        let (alpha_t, scale_t, offset_t) = if visible {
            (1.0, 1.0, 0.0)
        } else {
            (0.0, 0.96, -12.0)
        };
        let (k, d) = params::GEOMETRY;
        self.alpha.update_dt(alpha_t, k, d, dt);
        self.scale.update_dt(scale_t, k, d, dt);
        self.offset.update_dt(offset_t, params::PAGE.0, params::PAGE.1, dt);
        self.animating()
    }

    /// 是否有动画在跑喵
    pub fn animating(&self) -> bool {
        !self.alpha.is_still() || !self.scale.is_still() || !self.offset.is_still()
    }

    /// 直接跳到目标态(呼出/隐藏瞬间)喵
    pub fn snap(&mut self, visible: bool) {
        self.alpha.snap(if visible { 1.0 } else { 0.0 });
        self.scale.snap(if visible { 1.0 } else { 0.96 });
        self.offset.snap(if visible { 0.0 } else { -12.0 });
    }
}

impl Default for PanelSprings {
    fn default() -> Self {
        Self::new()
    }
}

/// 弹簧参数基准喵~(WinIsland 实测调校值,直接复用)
///
/// dt 归一化规则: `dt = (距上帧秒数 × 60.0).clamp(0.1, 6.0)`,以 60fps 为基准,
/// 跨刷新率观感一致喵!
#[allow(dead_code)] // 部分配方参数待第二阶段动效使用喵
pub mod params {
    /// 几何形变(尺寸/圆角): 轻微过冲,灵动岛质感来源喵
    pub const GEOMETRY: (f32, f32) = (0.10, 0.68);
    /// 页面/内容平移: 稍快、更「跟手」喵
    pub const PAGE: (f32, f32) = (0.12, 0.68);
    /// 隐藏滑出: 干脆利落喵
    pub const HIDE_OUT: (f32, f32) = (0.12, 0.70);
    /// 隐藏弹回: 更绵软,回弹感强喵
    pub const HIDE_IN: (f32, f32) = (0.08, 0.78);
    /// 强弹跳(通知/横幅): 1-2 个周期收敛喵
    pub const BOUNCE: (f32, f32) = (0.18, 0.60);

    /// 把距上帧的秒数归一化到 60fps 步长喵~
    pub fn normalize_dt(elapsed_secs: f32) -> f32 {
        (elapsed_secs * 60.0).clamp(0.1, 6.0)
    }
}
