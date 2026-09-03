//! 弹簧引擎 —— 原项目 js/spring.js 的逐位等价实现喵
//!
//! 公式与运算顺序刻意与 JS 版保持一致喵。用 f64：精度对齐更重要喵。

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// 积分步长。调小更稳、调大更快但会飘
pub const FIXED_DT: f64 = 1.0 / 240.0;
/// 单帧最多补偿的时间，防止页面切后台回来一次性积分到天荒地老
pub const MAX_FRAME: f64 = 1.0 / 15.0;
/// bounce → 过冲量的换算：bounce=1 时过冲 55%，对 UI 是"很弹但不飞"
pub const OVERSHOOT_SCALE: f64 = 0.55;
/// ζ 的安全区间：太小会振荡到天荒地老，太大退化成临界阻尼
pub const ZETA_MIN: f64 = 0.06;
pub const ZETA_MAX: f64 = 0.995;
/// bounce 的上限
pub const BOUNCE_MAX: f64 = 0.95;

/// 与 JS 语义一致的 clamp（NaN 时原样返回，不像 f64::clamp 那样 panic）
#[inline]
pub fn clamp(v: f64, min: f64, max: f64) -> f64 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

/// 给定期望的「稳定时间」，反推需要的 ωn 系数。
///
/// 推导：欠阻尼响应包络 ≈ e^(−ζωn·t) / √(1−ζ²)，令它在 `t = duration` 时衰减到 1%
///   → ζωn·T = 4.6 − ½·ln(1−ζ²)
///
/// 数值仿真验证：ζ ∈ [0.05, 0.9] 内误差 < 8%。
pub fn omega_coef(zeta: f64) -> f64 {
    let z = clamp(zeta, ZETA_MIN, ZETA_MAX);
    (4.6 - 0.5 * (1.0 - z * z).ln()) / z
}

/// 过冲量 → 阻尼比 ζ（二阶系统经典关系 OS = e^(−πζ/√(1−ζ²)) 的反函数）
pub fn zeta_from_overshoot(os: f64) -> f64 {
    if os <= 1e-6 {
        return 1.0;
    }
    let l = -os.ln();
    clamp(l / (PI * PI + l * l).sqrt(), ZETA_MIN, 1.0)
}

/// 阻尼比 ζ → 过冲量
pub fn overshoot_from_zeta(zeta: f64) -> f64 {
    let z = clamp(zeta, ZETA_MIN, 0.999_999);
    (-PI * z / (1.0 - z * z).sqrt()).exp()
}

/// 物理三件套
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringParams {
    pub stiffness: f64,
    pub damping: f64,
    pub mass: f64,
}

/// duration + bounce 三件套喵
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DurationBounce {
    pub duration: f64,
    pub bounce: f64,
    #[serde(default = "one_mass")]
    pub mass: f64,
}

fn one_mass() -> f64 {
    1.0
}

impl SpringParams {
    /// 从「时长 + 回弹」推导物理参数。bounce 语义 = 回弹强度，0 干脆、1 约过冲 55%
    pub fn from_duration_bounce(duration: f64, bounce: f64, mass: f64) -> Self {
        let m = if mass > 0.0 { mass } else { 1.0 };
        let b = clamp(bounce, 0.0, BOUNCE_MAX);
        let zeta = zeta_from_overshoot(b * OVERSHOOT_SCALE);
        let omega = omega_coef(zeta) / duration.max(0.05);
        let stiffness = omega * omega * m;
        let damping = 2.0 * zeta * (stiffness * m).sqrt();
        SpringParams { stiffness, damping, mass: m }
    }

    /// 上面那位的逆运算，来回换算误差 < 1e-12
    pub fn to_duration_bounce(self) -> DurationBounce {
        let m = if self.mass > 0.0 { self.mass } else { 1.0 };
        let k = self.stiffness.max(1e-6);
        let zeta = clamp(self.damping / (2.0 * (k * m).sqrt()), ZETA_MIN, ZETA_MAX);
        let omega = (k / m).sqrt();
        DurationBounce {
            duration: clamp(omega_coef(zeta) / omega, 0.02, 20.0),
            bounce: clamp(overshoot_from_zeta(zeta) / OVERSHOOT_SCALE, 0.0, BOUNCE_MAX),
            mass: m,
        }
    }
}

/// 一维弹簧。保留速度，所以半路改目标不会抽搐。
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub value: f64,
    pub target: f64,
    pub velocity: f64,
    pub stiffness: f64,
    pub damping: f64,
    pub mass: f64,
    /// 静止判定：距离阈值。像素通道可以松（0.03），归一化通道要严（0.0015）
    pub rest_delta: f64,
    /// 静止判定：速度阈值
    pub rest_speed: f64,
    pub settled: bool,
    acc: f64,
}

impl Spring {
    /// 建一根弹簧。rest_delta / rest_speed 建议按通道量级给
    pub fn new(value: f64, params: SpringParams, rest_delta: f64, rest_speed: f64) -> Self {
        Spring {
            value,
            target: value,
            velocity: 0.0,
            stiffness: params.stiffness,
            damping: params.damping,
            mass: params.mass,
            rest_delta,
            rest_speed,
            settled: true,
            acc: 0.0,
        }
    }

    /// 换一套弹簧参数（改 duration/bounce 或 stiffness/damping 都行）
    pub fn configure(&mut self, params: SpringParams) {
        self.stiffness = params.stiffness;
        self.damping = params.damping;
        self.mass = params.mass;
    }

    /// 设置目标值。已经在跑的弹簧会保留当前速度，衔接自然
    pub fn set(&mut self, target: f64) {
        self.target = target;
        self.settled = false;
        self.acc = 0.0;
    }

    /// 瞬间归位，清空速度
    pub fn jump(&mut self, value: f64) {
        self.value = value;
        self.target = value;
        self.velocity = 0.0;
        self.settled = true;
        self.acc = 0.0;
    }

    /// 推进 dt 秒，返回当前值
    pub fn step(&mut self, dt: f64) -> f64 {
        if self.settled {
            return self.value;
        }

        self.acc += dt;
        let mut guard = 0;
        while self.acc >= FIXED_DT && guard < 900 {
            self.acc -= FIXED_DT;
            guard += 1;
            let a = (-self.stiffness * (self.value - self.target)
                - self.damping * self.velocity)
                / self.mass;
            self.velocity += a * FIXED_DT;
            self.value += self.velocity * FIXED_DT;
        }
        if guard >= 900 {
            self.acc = 0.0;
        }

        if self.velocity.abs() < self.rest_speed && (self.target - self.value).abs() < self.rest_delta
        {
            self.value = self.target;
            self.velocity = 0.0;
            self.settled = true;
        }
        self.value
    }
}

// ───────────────────────── 缓动补间（与 Spring 接口一致，可互换） ─────────────────────────

fn linear(t: f64) -> f64 {
    t
}
fn ease_out_quad(t: f64) -> f64 {
    1.0 - (1.0 - t) * (1.0 - t)
}
fn ease_out_quart(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(4)
}
fn ease_out_quint(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(5)
}
fn ease_out_expo(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - (-10.0 * t).exp2()
    }
}
fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}
/// 带过冲的回弹，和 spring 的 bounce 是两种味道
fn ease_out_back(t: f64) -> f64 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// 名字 → 缓动函数（与 js 端 EASINGS 的 key 一一对应）
pub fn easing_fn(name: &str) -> fn(f64) -> f64 {
    match name {
        "linear" => linear,
        "easeOutQuad" => ease_out_quad,
        "easeOutQuart" => ease_out_quart,
        "easeOutQuint" => ease_out_quint,
        "easeOutExpo" => ease_out_expo,
        "easeInOutCubic" => ease_in_out_cubic,
        "easeOutBack" => ease_out_back,
        _ => ease_out_quint,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tween {
    pub value: f64,
    from: f64,
    pub target: f64,
    elapsed: f64,
    pub duration: f64,
    pub easing: &'static str,
    pub settled: bool,
}

impl Tween {
    pub fn new(value: f64, duration: f64, easing: &'static str) -> Self {
        Tween { value, from: value, target: value, elapsed: 0.0, duration, easing, settled: true }
    }

    pub fn set(&mut self, target: f64) {
        self.from = self.value;
        self.target = target;
        self.elapsed = 0.0;
        self.settled = false;
    }

    pub fn jump(&mut self, value: f64) {
        self.value = value;
        self.from = value;
        self.target = value;
        self.elapsed = 0.0;
        self.settled = true;
    }

    pub fn step(&mut self, dt: f64) -> f64 {
        if self.settled {
            return self.value;
        }
        self.elapsed += dt;
        let d = self.duration.max(0.0);
        let t = if d > 0.0 { self.elapsed / d } else { 1.0 };
        if t >= 1.0 {
            self.value = self.target;
            self.settled = true;
            return self.value;
        }
        let f = easing_fn(self.easing);
        self.value = self.from + (self.target - self.from) * f(t);
        self.value
    }
}

/// 弹簧 / 缓动 的统一外壳 —— DynamicIsland 只跟它打交道
#[derive(Debug, Clone, Copy)]
pub enum Animator {
    Spring(Spring),
    Tween(Tween),
}

impl Animator {
    pub fn new_spring(value: f64, params: SpringParams, rest_delta: f64, rest_speed: f64) -> Self {
        Animator::Spring(Spring::new(value, params, rest_delta, rest_speed))
    }
    pub fn new_tween(value: f64, duration: f64, easing: &'static str) -> Self {
        Animator::Tween(Tween::new(value, duration, easing))
    }

    pub fn value(&self) -> f64 {
        match self {
            Animator::Spring(s) => s.value,
            Animator::Tween(t) => t.value,
        }
    }
    pub fn settled(&self) -> bool {
        match self {
            Animator::Spring(s) => s.settled,
            Animator::Tween(t) => t.settled,
        }
    }
    pub fn is_tween(&self) -> bool {
        matches!(self, Animator::Tween(_))
    }

    pub fn set(&mut self, target: f64) {
        match self {
            Animator::Spring(s) => s.set(target),
            Animator::Tween(t) => t.set(target),
        }
    }
    pub fn jump(&mut self, value: f64) {
        match self {
            Animator::Spring(s) => s.jump(value),
            Animator::Tween(t) => t.jump(value),
        }
    }
    pub fn step(&mut self, dt: f64) -> f64 {
        match self {
            Animator::Spring(s) => s.step(dt),
            Animator::Tween(t) => t.step(dt),
        }
    }

    /// 换弹簧参数（只对 Spring 生效）
    pub fn configure_spring(&mut self, params: SpringParams) {
        if let Animator::Spring(s) = self {
            s.configure(params);
        }
    }
    /// 换补间参数（只对 Tween 生效）
    pub fn configure_tween(&mut self, duration: f64, easing: &'static str) {
        if let Animator::Tween(t) = self {
            t.duration = duration;
            t.easing = easing;
        }
    }
}
