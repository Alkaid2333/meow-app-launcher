//! 动画内核喵~
//!
//! 纯 Rust 数学库,不依赖 Skia、不依赖平台,可单测喵!
//! 渲染层只消费这里的数值结果(弹簧值、透明度、面板进度),不自己算动画喵。
//!
//! 设计心法(与 WinIsland 动画 skill 一致):
//! * 弹簧即几何——所有动效参数都是弹簧值,绘制参数 = f(弹簧当前值)喵
//! * 动画永远可打断——每帧根据当前状态重算,不存在不可逆的状态机喵
//! * 停稳即退出帧循环——零功耗喵

pub mod springs;

pub use springs::Spring;

/// 启动器窗口的弹簧集喵~
///
/// 单窗口架构下,灵动岛由三根弹簧驱动:
/// * `alpha`  整体透明度(呼出淡入 / 隐藏淡出)喵
/// * `panel`  结果面板展开进度 0..1(驱动面板高度与内容透明度)喵
/// * `offset` 呼出时的垂直位移(营造「从屏幕上方滑入」感)喵
#[derive(Debug)]
pub struct LauncherSprings {
    /// 整体透明度喵
    pub alpha: Spring,
    /// 结果面板展开进度喵
    pub panel: Spring,
    /// 呼出位移(px)喵
    pub offset: Spring,
}

impl LauncherSprings {
    /// 造一套初始隐藏态的弹簧喵~
    pub fn new() -> Self {
        Self {
            alpha: Spring::new(0.0),
            panel: Spring::new(0.0),
            offset: Spring::new(-16.0),
        }
    }

    /// 推进一帧喵,返回是否还在动画中喵~
    ///
    /// * `dt` - 归一化步长(60fps 基准)喵
    /// * `visible` - 窗口是否呼出喵
    /// * `has_results` - 是否有结果需要展开面板喵
    pub fn tick(&mut self, dt: f32, visible: bool, has_results: bool) -> bool {
        let alpha_t = if visible { 1.0 } else { 0.0 };
        let offset_t = if visible { 0.0 } else { -16.0 };
        let panel_t = if visible && has_results { 1.0 } else { 0.0 };

        let (k, d) = params::GEOMETRY;
        self.alpha.update_dt(alpha_t, k, d, dt);
        self.offset.update_dt(offset_t, k, d, dt);
        // 面板用「跟手」参数,展开/折叠更利落喵
        self.panel.update_dt(panel_t, params::PAGE.0, params::PAGE.1, dt);
        self.animating()
    }

    /// 是否还有动画在跑喵~
    pub fn animating(&self) -> bool {
        !self.alpha.is_still() || !self.panel.is_still() || !self.offset.is_still()
    }

    /// 直接跳到目标态喵~(呼出/隐藏瞬间,跳过过渡)喵
    pub fn snap(&mut self, visible: bool, has_results: bool) {
        self.alpha.snap(if visible { 1.0 } else { 0.0 });
        self.offset.snap(if visible { 0.0 } else { -16.0 });
        self.panel.snap(if visible && has_results { 1.0 } else { 0.0 });
    }
}

impl Default for LauncherSprings {
    fn default() -> Self {
        Self::new()
    }
}

/// 弹簧参数基准喵~(WinIsland 实测调校值,直接复用)
///
/// dt 归一化规则: `dt = (距上帧秒数 × 60.0).clamp(0.1, 6.0)`,以 60fps 为基准,
/// 跨刷新率观感一致喵!
pub mod params {
    /// 几何形变(尺寸/透明度/位移): 轻微过冲,灵动岛质感来源喵
    pub const GEOMETRY: (f32, f32) = (0.10, 0.68);
    /// 页面/内容平移(面板展开): 稍快、更「跟手」喵
    pub const PAGE: (f32, f32) = (0.12, 0.68);
    /// 隐藏滑出: 干脆利落喵
    #[allow(dead_code)] // 待隐藏/托盘动效接入喵
    pub const HIDE_OUT: (f32, f32) = (0.12, 0.70);
    /// 隐藏弹回: 更绵软,回弹感强喵
    #[allow(dead_code)]
    pub const HIDE_IN: (f32, f32) = (0.08, 0.78);
    /// 强弹跳(通知/横幅): 1-2 个周期收敛喵
    #[allow(dead_code)]
    pub const BOUNCE: (f32, f32) = (0.18, 0.60);

    /// 把距上帧的秒数归一化到 60fps 步长喵~
    pub fn normalize_dt(elapsed_secs: f32) -> f32 {
        (elapsed_secs * 60.0).clamp(0.1, 6.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 呼出后弹簧应收敛到可见态喵
    #[test]
    fn show_settles_to_visible() {
        let mut s = LauncherSprings::new();
        for _ in 0..600 {
            s.tick(1.0, true, true);
            if !s.animating() {
                break;
            }
        }
        assert!((s.alpha.value - 1.0).abs() < 0.01, "alpha={}", s.alpha.value);
        assert!((s.panel.value - 1.0).abs() < 0.01, "panel={}", s.panel.value);
        assert!((s.offset.value - 0.0).abs() < 0.01, "offset={}", s.offset.value);
    }

    /// 隐藏后应收敛到不可见态喵
    #[test]
    fn hide_settles_to_hidden() {
        let mut s = LauncherSprings::new();
        s.snap(true, true);
        for _ in 0..600 {
            s.tick(1.0, false, false);
            if !s.animating() {
                break;
            }
        }
        assert!((s.alpha.value - 0.0).abs() < 0.01, "alpha={}", s.alpha.value);
    }

    /// snap 必须瞬间到位喵
    #[test]
    fn snap_is_instant() {
        let mut s = LauncherSprings::new();
        s.snap(true, false);
        assert!((s.alpha.value - 1.0).abs() < f32::EPSILON);
        assert!((s.panel.value - 0.0).abs() < f32::EPSILON);
        assert!(!s.animating());
    }
}
