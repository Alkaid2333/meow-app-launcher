//! 灵动岛状态机与几何求解 —— js/island.js 的 Rust 等价实现
//!
//! 只算几何，不碰渲染。每帧 [`DynamicIsland::step`] 返回一个 [`IslandFrame`]，
//! 里面是画一个岛所需的全部数字，拿去喂 skia 就行。
//!
//! 几何锚定规则（与 JS 端一致，改之前先想清楚）：
//! - 水平以「中心 X」为锚，向两侧对称呼吸
//! - 垂直以「胶囊顶边」为锚，**默认向 +Y 生长**；
//!   一旦顶到底部安全线，就把整块往上推，继续向 −Y 补足高度
//! - 岛和扩展态永远被夹在 margin 围出的安全区里
//! - 键入区宽度 = 岛宽 × input_ratio，严格居中

use super::springs::{clamp, Animator, DurationBounce, SpringParams, MAX_FRAME};
use serde::{Deserialize, Serialize};

#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[inline]
fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

// ───────────────────────── 配置 ─────────────────────────

/// 六条转换各自的动画配置喵
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpringSet {
    pub summon: DurationBounce,
    pub dismiss: DurationBounce,
    pub expand: DurationBounce,
    pub collapse: DurationBounce,
    #[serde(rename = "summonExpanded")]
    pub summon_expanded: DurationBounce,
    #[serde(rename = "dismissExpanded")]
    pub dismiss_expanded: DurationBounce,
}

impl SpringSet {
    pub fn get(self, t: IslandTransition) -> DurationBounce {
        match t {
            IslandTransition::Summon => self.summon,
            IslandTransition::Dismiss => self.dismiss,
            IslandTransition::Expand => self.expand,
            IslandTransition::Collapse => self.collapse,
            IslandTransition::SummonExpanded => self.summon_expanded,
            IslandTransition::DismissExpanded => self.dismiss_expanded,
        }
    }

    pub fn get_mut(&mut self, t: IslandTransition) -> &mut DurationBounce {
        match t {
            IslandTransition::Summon => &mut self.summon,
            IslandTransition::Dismiss => &mut self.dismiss,
            IslandTransition::Expand => &mut self.expand,
            IslandTransition::Collapse => &mut self.collapse,
            IslandTransition::SummonExpanded => &mut self.summon_expanded,
            IslandTransition::DismissExpanded => &mut self.dismiss_expanded,
        }
    }
}

impl Default for SpringSet {
    fn default() -> Self {
        SpringSet {
            summon: DurationBounce { duration: 0.55, bounce: 0.28, mass: 1.0 },
            dismiss: DurationBounce { duration: 0.40, bounce: 0.00, mass: 1.0 },
            expand: DurationBounce { duration: 0.62, bounce: 0.14, mass: 1.0 },
            collapse: DurationBounce { duration: 0.48, bounce: 0.10, mass: 1.0 },
            summon_expanded: DurationBounce { duration: 0.74, bounce: 0.22, mass: 1.0 },
            dismiss_expanded: DurationBounce { duration: 0.52, bounce: 0.04, mass: 1.0 },
        }
    }
}

/// 过渡引擎喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionMode {
    #[default]
    Spring,
    Ease,
    Linear,
    Instant,
}

impl MotionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Spring => "弹簧",
            Self::Ease => "缓动",
            Self::Linear => "线性",
            Self::Instant => "瞬切",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Spring => Self::Ease,
            Self::Ease => Self::Linear,
            Self::Linear => Self::Instant,
            Self::Instant => Self::Spring,
        }
    }
}

/// 缓动曲线名喵
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EasingName {
    Linear,
    EaseOutQuad,
    EaseOutQuart,
    #[default]
    EaseOutQuint,
    EaseOutExpo,
    EaseInOutCubic,
    EaseOutBack,
}

impl EasingName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::EaseOutQuad => "easeOutQuad",
            Self::EaseOutQuart => "easeOutQuart",
            Self::EaseOutQuint => "easeOutQuint",
            Self::EaseOutExpo => "easeOutExpo",
            Self::EaseInOutCubic => "easeInOutCubic",
            Self::EaseOutBack => "easeOutBack",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Linear => "线性",
            Self::EaseOutQuad => "二次缓出",
            Self::EaseOutQuart => "四次缓出",
            Self::EaseOutQuint => "五次缓出",
            Self::EaseOutExpo => "指数缓出",
            Self::EaseInOutCubic => "三次缓入缓出",
            Self::EaseOutBack => "回弹缓出",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Linear => Self::EaseOutQuad,
            Self::EaseOutQuad => Self::EaseOutQuart,
            Self::EaseOutQuart => Self::EaseOutQuint,
            Self::EaseOutQuint => Self::EaseOutExpo,
            Self::EaseOutExpo => Self::EaseInOutCubic,
            Self::EaseInOutCubic => Self::EaseOutBack,
            Self::EaseOutBack => Self::Linear,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IslandConfig {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    #[serde(rename = "inputRatio", default = "def_input_ratio")]
    pub input_ratio: f64,
    #[serde(rename = "expandedWidth")]
    pub expanded_width: f64,
    #[serde(rename = "expandedHeight")]
    pub expanded_height: f64,
    #[serde(rename = "expandedRadius", default = "def_expanded_radius")]
    pub expanded_radius: f64,
    #[serde(rename = "padX", default = "def_pad_x")]
    pub pad_x: f64,
    #[serde(rename = "padY", default = "def_pad_y")]
    pub pad_y: f64,
    #[serde(rename = "slotHeight", default = "def_slot_h")]
    pub slot_height: f64,
    #[serde(rename = "summonSquash", default = "def_squash")]
    pub summon_squash: f64,
    #[serde(default = "def_margin")]
    pub margin: f64,
    /// 动画帧率(Hz),0 = 跟随显示器刷新率喵
    #[serde(rename = "animFps", default)]
    pub anim_fps: u32,
    #[serde(default)]
    pub springs: SpringSet,
    #[serde(rename = "motionMode", default)]
    pub motion_mode: MotionMode,
    #[serde(default)]
    pub easing: EasingName,
    #[serde(rename = "reduceMotion", default)]
    pub reduce_motion: bool,
    #[serde(rename = "autoMorph", default = "def_true")]
    pub auto_morph: bool,
    #[serde(default = "def_true")]
    pub draggable: bool,
}

fn def_input_ratio() -> f64 {
    0.72
}
fn def_expanded_radius() -> f64 {
    34.0
}
fn def_pad_x() -> f64 {
    18.0
}
fn def_pad_y() -> f64 {
    15.0
}
fn def_slot_h() -> f64 {
    34.0
}
fn def_squash() -> f64 {
    0.34
}
fn def_margin() -> f64 {
    16.0
}
fn def_true() -> bool {
    true
}

impl Default for IslandConfig {
    fn default() -> Self {
        IslandConfig {
            width: 300.0,
            height: 46.0,
            x: 50.0,
            y: 14.0,
            input_ratio: 0.72,
            expanded_width: 560.0,
            expanded_height: 268.0,
            expanded_radius: 34.0,
            pad_x: 18.0,
            pad_y: 15.0,
            slot_height: 34.0,
            summon_squash: 0.34,
            margin: 16.0,
            anim_fps: 0,
            springs: SpringSet::default(),
            motion_mode: MotionMode::Spring,
            easing: EasingName::EaseOutQuint,
            reduce_motion: false,
            auto_morph: true,
            draggable: true,
        }
    }
}

// ───────────────────────── 状态机 ─────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IslandState {
    Hidden,
    Island,
    Expanded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IslandTransition {
    /// 唤出岛：从中间向两侧弹开
    Summon,
    /// 关闭岛：收回中间后消失
    Dismiss,
    /// 展开：宽向两侧、高向 +Y，触底反向
    Expand,
    /// 收起：缩回胶囊
    Collapse,
    /// 直接展开：跳过胶囊态
    SummonExpanded,
    /// 直接关闭：一步收干
    DismissExpanded,
}

impl IslandTransition {
    pub fn from_state(self) -> IslandState {
        match self {
            IslandTransition::Summon | IslandTransition::SummonExpanded => IslandState::Hidden,
            IslandTransition::Expand | IslandTransition::Dismiss => IslandState::Island,
            IslandTransition::Collapse | IslandTransition::DismissExpanded => IslandState::Expanded,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Summon => "summon",
            Self::Dismiss => "dismiss",
            Self::Expand => "expand",
            Self::Collapse => "collapse",
            Self::SummonExpanded => "summonExpanded",
            Self::DismissExpanded => "dismissExpanded",
        }
    }

    pub fn to_state(self) -> IslandState {
        match self {
            IslandTransition::Summon | IslandTransition::Collapse => IslandState::Island,
            IslandTransition::Expand | IslandTransition::SummonExpanded => IslandState::Expanded,
            IslandTransition::Dismiss | IslandTransition::DismissExpanded => IslandState::Hidden,
        }
    }
    /// 六条转换的固定顺序，方便遍历
    pub const ALL: [IslandTransition; 6] = [
        IslandTransition::Summon,
        IslandTransition::Dismiss,
        IslandTransition::Expand,
        IslandTransition::Collapse,
        IslandTransition::SummonExpanded,
        IslandTransition::DismissExpanded,
    ];
}

// ───────────────────────── 输出 ─────────────────────────

/// 一帧的几何快照。渲染层只需要这些数字。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IslandFrame {
    pub width: f64,
    pub height: f64,
    pub left: f64,
    pub top: f64,
    pub opacity: f64,
    pub radius: f64,
    /// 收起时可以将整块跳过不画
    pub visible: bool,
    /// 当前形态进度：0 = 胶囊，1 = 扩展（渲染内容样式时常用）
    pub morph: f64,
    /// 键入区（搜索框），相对岛本体左上角
    pub slot: Rect,
    /// 扩展面板（结果列表），相对岛本体左上角
    pub panel: Rect,
    pub panel_opacity: f64,
    /// 面板的入场位移（px，向下为正）
    pub panel_shift: f64,
    /// 垂直方向被安全线挡住（触底反弹中）
    pub hit_bottom: bool,
    /// 水平方向被安全线挡住
    pub hit_x: bool,
}

// ───────────────────────── 主体 ─────────────────────────

struct IslandSprings {
    w: Animator,
    h: Animator,
    cx: Animator,
    ty: Animator,
    opacity: Animator,
    morph: Animator,
}

impl IslandSprings {
    fn new(v: [f64; 6], tween: bool) -> Self {
        let tune = SpringParams::from_duration_bounce(0.28, 0.0, 1.0);
        if tween {
            IslandSprings {
                w: Animator::new_tween(v[0], 0.28, "easeOutQuint"),
                h: Animator::new_tween(v[1], 0.28, "easeOutQuint"),
                cx: Animator::new_tween(v[2], 0.28, "easeOutQuint"),
                ty: Animator::new_tween(v[3], 0.28, "easeOutQuint"),
                opacity: Animator::new_tween(v[4], 0.28, "easeOutQuint"),
                morph: Animator::new_tween(v[5], 0.28, "easeOutQuint"),
            }
        } else {
            IslandSprings {
                w: Animator::new_spring(v[0], tune, 0.03, 1.2),
                h: Animator::new_spring(v[1], tune, 0.03, 1.2),
                cx: Animator::new_spring(v[2], tune, 0.03, 1.2),
                ty: Animator::new_spring(v[3], tune, 0.03, 1.2),
                opacity: Animator::new_spring(v[4], tune, 0.0015, 0.004),
                morph: Animator::new_spring(v[5], tune, 0.0012, 0.004),
            }
        }
    }
}

pub struct DynamicIsland {
    pub config: IslandConfig,
    pub state: IslandState,
    springs: IslandSprings,
    stage_w: f64,
    stage_h: f64,
}

/// 一条转换的动画参数（决定用哪套引擎）
enum AnimatorSpec {
    Spring(DurationBounce),
    Tween { duration: f64, easing: &'static str },
}

impl DynamicIsland {
    pub fn new(config: IslandConfig) -> Self {
        let mut island = DynamicIsland {
            config,
            state: IslandState::Hidden,
            springs: IslandSprings::new([0.0; 6], false),
            stage_w: 0.0,
            stage_h: 0.0,
        };
        island.sync(false);
        island
    }

    /// 舞台尺寸变了要调这个
    pub fn set_stage(&mut self, w: f64, h: f64) {
        self.stage_w = w;
        self.stage_h = h;
        if self.settled() {
            self.sync(false);
        } else {
            self.sync(true);
        }
    }

    /// 触发一次转换。非法转换（起点对不上）返回 false。
    pub fn transition(&mut self, t: IslandTransition) -> bool {
        if t.from_state() != self.state {
            return false;
        }
        let spec = self.motion_for(t);
        self.ensure_mode(matches!(spec, AnimatorSpec::Tween { .. }));

        match spec {
            AnimatorSpec::Spring(db) => {
                let p = SpringParams::from_duration_bounce(db.duration, db.bounce, db.mass);
                for a in self.anims() {
                    a.configure_spring(p);
                }
            }
            AnimatorSpec::Tween { duration, easing } => {
                for a in self.anims() {
                    a.configure_tween(duration, easing);
                }
            }
        }

        let g = self.geometry(t.to_state());
        let v = [g.0, g.1, g.2, g.3, g.4, g.5];
        for (i, a) in self.anims().iter_mut().enumerate() {
            a.set(v[i]);
        }
        self.state = t.to_state();
        true
    }

    /// 按可见性 + 是否展开,挑一段合法过渡喵
    pub fn go(&mut self, want_visible: bool, want_expanded: bool) {
        let want = match (want_visible, want_expanded) {
            (false, _) => IslandState::Hidden,
            (true, false) => IslandState::Island,
            (true, true) => IslandState::Expanded,
        };
        if want == self.state {
            return;
        }
        let t = match (self.state, want) {
            (IslandState::Hidden, IslandState::Island) => IslandTransition::Summon,
            (IslandState::Island, IslandState::Hidden) => IslandTransition::Dismiss,
            (IslandState::Island, IslandState::Expanded) => IslandTransition::Expand,
            (IslandState::Expanded, IslandState::Island) => IslandTransition::Collapse,
            (IslandState::Hidden, IslandState::Expanded) => IslandTransition::SummonExpanded,
            (IslandState::Expanded, IslandState::Hidden) => IslandTransition::DismissExpanded,
            _ => return,
        };
        self.transition(t);
    }

    /// 热更新配置喵
    pub fn set_config(&mut self, next: IslandConfig, animate: bool) {
        let only_xy = (self.config.x != next.x || self.config.y != next.y)
            && self.config.width == next.width
            && self.config.height == next.height
            && self.config.input_ratio == next.input_ratio
            && self.config.expanded_width == next.expanded_width
            && self.config.expanded_height == next.expanded_height
            && self.config.expanded_radius == next.expanded_radius
            && self.config.pad_x == next.pad_x
            && self.config.pad_y == next.pad_y
            && self.config.slot_height == next.slot_height
            && self.config.summon_squash == next.summon_squash
            && self.config.margin == next.margin
            && self.config.anim_fps == next.anim_fps
            && self.config.motion_mode == next.motion_mode
            && self.config.springs == next.springs
            && self.config.reduce_motion == next.reduce_motion
            && self.config.easing == next.easing;
        self.config = next;
        if only_xy {
            self.sync_position();
            return;
        }
        self.sync(animate);
    }

    /// 当前状态下能走哪几条
    pub fn available(&self) -> Vec<IslandTransition> {
        IslandTransition::ALL
            .iter()
            .copied()
            .filter(|t| t.from_state() == self.state)
            .collect()
    }

    /// 推进一帧
    pub fn step(&mut self, dt: f64) -> IslandFrame {
        let dt = dt.min(MAX_FRAME);
        for a in self.anims() {
            a.step(dt);
        }
        self.frame()
    }

    pub fn settled(&self) -> bool {
        self.springs.w.settled()
            && self.springs.h.settled()
            && self.springs.cx.settled()
            && self.springs.ty.settled()
            && self.springs.opacity.settled()
            && self.springs.morph.settled()
    }

    /// 参数改完之后调一下，把动画器目标对齐到当前状态。
    /// animate=true 用软弹簧跟随（拖滑块手感），false 直接归位。
    pub fn sync(&mut self, animate: bool) {
        if self.stage_w <= 0.0 {
            return;
        }
        let g = self.geometry(self.state);
        if !animate {
            let v = [g.0, g.1, g.2, g.3, g.4, g.5];
            for (i, a) in self.anims().iter_mut().enumerate() {
                a.jump(v[i]);
            }
            return;
        }
        if self.settled() {
            match &mut self.springs.w {
                Animator::Spring(s) => {
                    s.configure(SpringParams::from_duration_bounce(0.28, 0.0, 1.0));
                }
                Animator::Tween(t) => {
                    t.duration = 0.28;
                    t.easing = self.config.easing.as_str();
                }
            }
        }
        let v = [g.0, g.1, g.2, g.3, g.4, g.5];
        for (i, a) in self.anims().iter_mut().enumerate() {
            a.set(v[i]);
        }
    }

    /// 只把位置动画器吸附到当前配置，其余通道原样不动。
    /// 拖拽时用这个：完全跟手，且不打断正在跑的形状动画。
    pub fn sync_position(&mut self) {
        if self.stage_w <= 0.0 {
            return;
        }
        let g = self.geometry(self.state);
        self.springs.cx.jump(g.2);
        self.springs.ty.jump(g.3);
    }

    /// 当前形态进度（0 = 胶囊，1 = 扩展）
    pub fn morph(&self) -> f64 {
        self.springs.morph.value()
    }

    /// 算出这一帧该怎么画
    pub fn frame(&self) -> IslandFrame {
        let c = &self.config;
        let s = &self.springs;
        let w = s.w.value().max(0.0);
        let h = s.h.value().max(0.0);
        let m = clamp(s.morph.value(), 0.0, 1.0);

        let (left, top, hit_bottom, hit_x) = self.resolve_bounds(s.cx.value(), s.ty.value(), w, h);

        let slot_w = lerp(w * c.input_ratio, (w - c.pad_x * 2.0).max(56.0), m);
        let slot_h = lerp(h, c.slot_height, m);
        let slot_y = lerp(0.0, c.pad_y, m);
        let panel_y = slot_y + slot_h + 12.0;

        IslandFrame {
            width: w,
            height: h,
            left,
            top,
            opacity: s.opacity.value(),
            radius: lerp(h / 2.0, c.expanded_radius, m),
            visible: !(s.opacity.value() < 0.01 && w < 0.5),
            morph: s.morph.value(),
            slot: Rect {
                x: (w - slot_w) / 2.0,
                y: slot_y,
                w: slot_w,
                h: slot_h,
            },
            panel: Rect {
                x: c.pad_x,
                y: panel_y,
                w: (w - c.pad_x * 2.0).max(0.0),
                h: (h - panel_y - c.pad_y).max(0.0),
            },
            panel_opacity: smoothstep(0.42, 0.98, s.morph.value()),
            panel_shift: lerp(12.0, 0.0, m),
            hit_bottom,
            hit_x,
        }
    }

    /// 把当前尺寸的岛塞进安全区 —— 边界反弹全在这里发生。
    ///
    /// · Y：顶边先钉在 ty，于是高度只往 +Y 长；
    ///      一旦底边越过安全线，就把整块往上推（等价于反向向 −Y 补足）
    /// · X：以 cx 为中心向两侧长，哪边贴线就往反方向推回
    fn resolve_bounds(&self, cx: f64, ty: f64, w: f64, h: f64) -> (f64, f64, bool, bool) {
        let mut min_x = self.config.margin;
        let mut max_x = self.stage_w - self.config.margin;
        let mut min_y = self.config.margin;
        let mut max_y = self.stage_h - self.config.margin;

        /* 安全区被压没了就退化成居中，避免出现负尺寸 */
        if max_x < min_x {
            min_x = self.stage_w / 2.0;
            max_x = min_x;
        }
        if max_y < min_y {
            min_y = self.stage_h / 2.0;
            max_y = min_y;
        }

        let mut left = cx - w / 2.0;
        let mut hit_x = false;
        if w >= max_x - min_x {
            left = min_x; // 比可用宽度还宽，只能撑满安全区
        } else if left < min_x {
            left = min_x; // 贴左边界 → 向右反弹
            hit_x = true;
        } else if left + w > max_x {
            left = max_x - w; // 贴右边界 → 向左反弹
            hit_x = true;
        }

        let mut top = ty;
        let mut hit_bottom = false;
        if h >= max_y - min_y {
            top = min_y; // 比可用高度还高，只能撑满安全区
            hit_bottom = ty + h > max_y;
        } else if ty + h > max_y {
            top = max_y - h; // 触底 → 反向向上补足高度
            hit_bottom = true;
        }
        if top < min_y {
            top = min_y; // 顶到天花板
        }

        (left, top, hit_bottom, hit_x)
    }

    /// 某个状态下六条通道各自的目标值：w, h, cx, ty, opacity, morph
    fn geometry(&self, state: IslandState) -> (f64, f64, f64, f64, f64, f64) {
        let c = &self.config;
        let cx = c.x / 100.0 * self.stage_w;
        let cy = c.y / 100.0 * self.stage_h;
        /* 生长锚点取胶囊的顶边：默认往 +Y 长，撞到安全线由 resolve_bounds 往上补 */
        let ty = cy - c.height / 2.0;

        match state {
            IslandState::Expanded => {
                (c.expanded_width, c.expanded_height, cx, ty, 1.0, 1.0)
            }
            IslandState::Island => (c.width, c.height, cx, ty, 1.0, 0.0),
            IslandState::Hidden => (0.0, c.height * c.summon_squash, cx, ty, 0.0, 0.0),
        }
    }

    fn spring_for(&self, t: IslandTransition) -> DurationBounce {
        match t {
            IslandTransition::Summon => self.config.springs.summon,
            IslandTransition::Dismiss => self.config.springs.dismiss,
            IslandTransition::Expand => self.config.springs.expand,
            IslandTransition::Collapse => self.config.springs.collapse,
            IslandTransition::SummonExpanded => self.config.springs.summon_expanded,
            IslandTransition::DismissExpanded => self.config.springs.dismiss_expanded,
        }
    }

    fn motion_for(&self, t: IslandTransition) -> AnimatorSpec {
        let db = self.spring_for(t);
        match self.config.motion_mode {
            MotionMode::Instant => {
                AnimatorSpec::Tween { duration: 0.001, easing: "linear" }
            }
            MotionMode::Linear => {
                AnimatorSpec::Tween { duration: db.duration * 0.8, easing: "linear" }
            }
            MotionMode::Ease => {
                AnimatorSpec::Tween { duration: db.duration, easing: self.config.easing.as_str() }
            }
            MotionMode::Spring => {
                if self.config.reduce_motion {
                    AnimatorSpec::Spring(DurationBounce {
                        duration: (db.duration * 0.32).min(0.16),
                        bounce: 0.0,
                        mass: db.mass,
                    })
                } else {
                    AnimatorSpec::Spring(db)
                }
            }
        }
    }

    /// 切换 spring / tween 引擎时整体换掉动画器，但当前值必须接住
    fn ensure_mode(&mut self, tween: bool) {
        if self.springs.w.is_tween() == tween {
            return;
        }
        let cur = [
            self.springs.w.value(),
            self.springs.h.value(),
            self.springs.cx.value(),
            self.springs.ty.value(),
            self.springs.opacity.value(),
            self.springs.morph.value(),
        ];
        self.springs = IslandSprings::new(cur, tween);
    }

    fn anims(&mut self) -> [&mut Animator; 6] {
        let s = &mut self.springs;
        [
            &mut s.w, &mut s.h, &mut s.cx, &mut s.ty, &mut s.opacity, &mut s.morph,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(island: &mut DynamicIsland) {
        for _ in 0..3000 {
            if island.settled() {
                break;
            }
            island.step(1.0 / 240.0);
        }
    }

    #[test]
    fn 六条转换的合法性() {
        let mut i = DynamicIsland::new(IslandConfig::default());
        i.set_stage(900.0, 500.0);
        for t in IslandTransition::ALL {
            i.state = t.from_state();
            i.sync(false);
            assert!(i.transition(t), "{t:?} 应该放行");

            let wrong = IslandTransition::ALL
                .iter()
                .copied()
                .find(|x| x.from_state() != t.from_state())
                .unwrap();
            i.state = wrong.from_state();
            assert!(!i.transition(t), "{t:?} 从错误起点应被拦截");
        }
    }

    #[test]
    fn 唤出后的胶囊几何() {
        let mut i = DynamicIsland::new(IslandConfig::default());
        i.set_stage(900.0, 500.0);
        i.transition(IslandTransition::Summon);
        settle(&mut i);

        let f = i.frame();
        assert_eq!(i.state, IslandState::Island);
        assert!((f.width - 300.0).abs() < 0.01, "w={}", f.width);
        assert!((f.height - 46.0).abs() < 0.01, "h={}", f.height);
        assert!((f.opacity - 1.0).abs() < 0.001);
        assert!((f.radius - 23.0).abs() < 0.01, "圆角应为岛高一半 {}", f.radius);
        /* 键入区 = 岛宽 × 0.72，居中 */
        assert!((f.slot.w - 216.0).abs() < 0.01, "slot.w={}", f.slot.w);
        assert!((f.slot.x + f.slot.w / 2.0 - 150.0).abs() < 0.01, "键入区未居中");
    }

    #[test]
    fn 展开时顶边不动且默认向正_y_生长() {
        let mut i = DynamicIsland::new(IslandConfig::default());
        i.set_stage(900.0, 500.0);
        i.transition(IslandTransition::Summon);
        settle(&mut i);
        let a = i.frame();
        let top = a.top;
        let cx = a.left + a.width / 2.0;

        i.transition(IslandTransition::Expand);
        settle(&mut i);
        let b = i.frame();

        assert!((b.width - 560.0).abs() < 0.01, "w={}", b.width);
        assert!((b.height - 268.0).abs() < 0.01, "h={}", b.height);
        assert!((b.top - top).abs() < 0.01, "顶边应保持不变，实际 {} vs {}", b.top, top);
        assert!(b.top + b.height > a.top + a.height, "底边应下移");
        assert!(!b.hit_bottom, "空间充足时不应触底");
        assert!((b.left + b.width / 2.0 - cx).abs() < 0.01, "水平中心漂移了");
        assert!((b.radius - 34.0).abs() < 0.01, "圆角应到 expanded_radius");
        /* 扩展态键入区占满内边距 */
        assert!((b.slot.w - 524.0).abs() < 0.01, "slot.w={}", b.slot.w);
    }

    #[test]
    fn 触底时反向向负_y_补足高度() {
        let mut i = DynamicIsland::new(IslandConfig::default());
        i.set_stage(900.0, 500.0);
        i.config.y = 90.0; // 胶囊很靠下，向 +Y 长必然出界
        i.config.margin = 0.0;
        i.sync(false);
        i.transition(IslandTransition::Summon);
        settle(&mut i);
        i.transition(IslandTransition::Expand);
        settle(&mut i);

        let f = i.frame();
        assert!(
            (f.top + f.height - 500.0).abs() < 0.01,
            "底边应贴住底线 {}",
            f.top + f.height
        );
        assert!(
            (f.top - (500.0 - i.config.expanded_height)).abs() < 0.01,
            "顶边应向上补足 {}",
            f.top
        );
        assert!(f.hit_bottom, "hit_bottom 标志应置位");
    }

    #[test]
    fn 水平贴边被推回安全区() {
        let mut i = DynamicIsland::new(IslandConfig::default());
        i.set_stage(900.0, 500.0);
        i.config.x = 10.0;
        i.config.margin = 0.0;
        i.sync(false);
        i.transition(IslandTransition::SummonExpanded);
        settle(&mut i);

        let f = i.frame();
        assert!((f.left - 0.0).abs() < 0.01, "左缘应贴住舞台左线 {}", f.left);
        assert!((f.width - 560.0).abs() < 0.01, "宽度不受边界影响");
        assert!(f.hit_x, "hit_x 标志应置位");
    }

    #[test]
    fn 安全边距_margin_生效() {
        let mut i = DynamicIsland::new(IslandConfig::default());
        i.set_stage(900.0, 500.0);
        i.config.x = 50.0;
        i.config.y = 95.0;
        i.config.margin = 30.0;
        i.sync(false);
        i.transition(IslandTransition::SummonExpanded);
        settle(&mut i);

        let f = i.frame();
        assert!(
            (f.top + f.height - 470.0).abs() < 0.01,
            "底边应贴住安全线 {}",
            f.top + f.height
        );
        assert!(f.left > 30.0 && f.left + f.width < 870.0, "水平整体在安全区内");
    }

    #[test]
    fn 一步到位的两条转换() {
        let mut i = DynamicIsland::new(IslandConfig::default());
        i.set_stage(900.0, 500.0);

        i.transition(IslandTransition::SummonExpanded);
        settle(&mut i);
        assert_eq!(i.state, IslandState::Expanded);
        assert!((i.frame().width - 560.0).abs() < 0.01);
        assert!((i.frame().height - 268.0).abs() < 0.01);
        assert!((i.morph() - 1.0).abs() < 0.001);

        i.transition(IslandTransition::DismissExpanded);
        settle(&mut i);
        assert_eq!(i.state, IslandState::Hidden);
        assert!(i.frame().width < 0.01);
        assert!(i.frame().opacity < 0.001);
        assert!(!i.frame().visible);
    }

    #[test]
    fn 拖拽位置不打断形状动画() {
        let mut i = DynamicIsland::new(IslandConfig::default());
        i.set_stage(900.0, 500.0);
        i.transition(IslandTransition::Summon);
        settle(&mut i);
        i.transition(IslandTransition::Expand);
        for _ in 0..20 {
            i.step(1.0 / 240.0);
        }
        let w_mid = i.frame().width;
        assert!(w_mid > 300.0 && w_mid < 560.0, "此刻应在动画中间 {}", w_mid);

        i.config.x = 70.0;
        i.config.y = 40.0;
        i.sync_position();
        let w_after = i.frame().width;
        assert!(
            (w_after - w_mid).abs() < 60.0,
            "拖拽把宽度拽飞了：{w_mid} → {w_after}"
        );

        settle(&mut i);
        assert!((i.frame().width - 560.0).abs() < 0.01, "动画应继续跑完");
    }

    #[test]
    fn 过渡引擎切换() {
        let mut i = DynamicIsland::new(IslandConfig::default());
        i.set_stage(900.0, 500.0);

        /* instant：一步到位 */
        i.config.motion_mode = MotionMode::Instant;
        i.transition(IslandTransition::Summon);
        settle(&mut i);
        assert!((i.frame().width - i.config.width).abs() < 0.01, "instant 应瞬间到位");

        /* linear：匀速 */
        i.config.motion_mode = MotionMode::Linear;
        i.transition(IslandTransition::Expand);
        i.step(1.0 / 240.0);
        let a = i.frame().width;
        i.step(0.05);
        let b = i.frame().width - a;
        i.step(0.05);
        let c = i.frame().width - a - b;
        assert!((b - c).abs() < b * 0.02, "linear 应等距推进 {} vs {}", b, c);

        /* ease：收敛到终点 */
        i.config.motion_mode = MotionMode::Ease;
        i.config.easing = EasingName::EaseOutQuint;
        i.transition(IslandTransition::Collapse);
        settle(&mut i);
        assert!((i.frame().width - i.config.width).abs() < 0.01, "ease 应收敛到岛宽");

        /* 半路切回 spring：当前值无缝衔接 */
        i.transition(IslandTransition::Summon);
        for _ in 0..20 {
            i.step(1.0 / 240.0);
        }
        let mid = i.frame().width;
        i.config.motion_mode = MotionMode::Spring;
        i.transition(IslandTransition::Expand);
        assert!((i.frame().width - mid).abs() < 0.01, "切换瞬间值不能跳变");
        settle(&mut i);
        assert!(
            (i.frame().width - i.config.expanded_width).abs() < 0.01,
            "动画照常跑完"
        );
    }

    #[test]
    fn 可用转换随状态变化() {
        let mut i = DynamicIsland::new(IslandConfig::default());
        i.set_stage(900.0, 500.0);
        assert_eq!(i.available().len(), 2); // hidden: summon / summonExpanded
        i.transition(IslandTransition::Summon);
        assert_eq!(i.available().len(), 2); // island: expand / dismiss
        i.transition(IslandTransition::Expand);
        assert_eq!(i.available().len(), 2); // expanded: collapse / dismissExpanded
    }
}
