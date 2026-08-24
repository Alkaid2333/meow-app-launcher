//! 弹簧物理内核喵~
//!
//! 纯 Rust 数学库,零 GPUI 依赖、零平台依赖,可独立单测喵!
//! 借鉴自 WinIsland 的 `utils/physics.rs`,原样移植并加了可爱注释喵。
//!
//! 心法: 所有动效的「数值层」都是纯函数——弹簧值、透明度、位移,
//! 渲染层只负责把数值翻译成样式喵。动画永远可打断,因为每帧都
//! 是根据当前状态重新计算,不存在不可逆的动画状态机喵!

/// 单个弹簧喵~
///
/// 用「半隐式欧拉积分」模拟弹簧物理:
/// 1. 先根据位移算力,更新速度
/// 2. 再用新速度更新位置
///
/// 这样能产生自然的过冲回弹,就是 iOS 灵动岛那种质感喵!
#[derive(Clone, Copy, Debug, Default)]
pub struct Spring {
    /// 当前位置喵
    pub value: f32,
    /// 当前速度喵
    pub velocity: f32,
}

impl Spring {
    /// 造一个新弹簧,停在指定位置,速度为零喵~
    pub fn new(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
        }
    }

    /// 朝目标值运动一步喵~
    ///
    /// * `target` - 目标值
    /// * `stiffness` - 刚度,越大越快到达
    /// * `damping` - 阻尼,越小越「弹」(过冲多),接近 1.0 越粘滞
    /// * `dt` - 归一化步长(60fps 基准,帧率无关)
    pub fn update_dt(&mut self, target: f32, stiffness: f32, damping: f32, dt: f32) {
        // 非法步长直接溜走,防止 NaN 传染喵
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        // 力 = 位移 × 刚度 × dt(半隐式欧拉: 先速度后位置)
        let force = (target - self.value) * stiffness * dt;
        self.velocity = (self.velocity + force) * damping.powf(dt);
        self.value += self.velocity * dt;
        // 数值兜底: 出现任何非有限值,直接吸到目标上,别飘走喵!
        if !self.value.is_finite() {
            self.value = target;
            self.velocity = 0.0;
        }
        if !self.velocity.is_finite() {
            self.velocity = 0.0;
        }
    }

    /// 打断动画: 直接取值并清零速度喵~
    ///
    /// 拖拽/点击接管时调用,这是「可打断动画」的标准动作喵!
    pub fn snap(&mut self, value: f32) {
        self.value = value;
        self.velocity = 0.0;
    }

    /// 弹簧是否已经停稳了喵?
    ///
    /// 停稳标准: 速度绝对值 ≤ 0.001
    pub fn is_still(&self) -> bool {
        self.velocity.abs() <= 0.001
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_reaches_target() {
        // 一只弹簧从 0 出发,蹦向 100 喵~
        let mut s = Spring::new(0.0);
        for _ in 0..600 {
            s.update_dt(100.0, 0.10, 0.68, 1.0);
            if s.is_still() {
                break;
            }
        }
        // 最终应该停在目标附近,误差小于 1 喵
        assert!((s.value - 100.0).abs() < 1.0, "spring settled at {}", s.value);
    }

    #[test]
    fn spring_snap_is_instant() {
        // 打断必须瞬间生效,不许磨蹭喵!
        let mut s = Spring::new(0.0);
        s.snap(42.0);
        assert_eq!(s.value, 42.0);
        assert_eq!(s.velocity, 0.0);
        assert!(s.is_still());
    }

    #[test]
    fn spring_survives_nan() {
        // 喂 NaN 毒药也不能崩溃喵!
        let mut s = Spring::new(f32::NAN);
        s.update_dt(10.0, 0.1, 0.68, 1.0);
        assert_eq!(s.value, 10.0);
        assert_eq!(s.velocity, 0.0);
    }

    #[test]
    fn spring_ignores_bad_dt() {
        // 负步长或零步长直接忽略喵
        let mut s = Spring::new(5.0);
        s.update_dt(10.0, 0.1, 0.68, -1.0);
        assert_eq!(s.value, 5.0);
        s.update_dt(10.0, 0.1, 0.68, 0.0);
        assert_eq!(s.value, 5.0);
    }
}
