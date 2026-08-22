---
name: winisland-animation-style
description: >-
  WinIsland 风格动画设计规范。当需要在 Rust + GPUI 项目中实现「灵动岛」质感动效
  （弹簧物理驱动几何、速度感知运动模糊、分级帧调度、可打断动画）时使用。
  参考项目：WinIsland v1.3.0（Rust + Skia 自绘 Windows 灵动岛）。
  触发词：灵动岛、Dynamic Island、弹簧动效、spring animation、胶囊动画、
  展开折叠动画、运动模糊、motion blur、歌词过渡动画。
---

# WinIsland 动画风格设计 Skill

> 本 skill 将 WinIsland v1.3.0 的动效方法论提炼为可复用的设计规范 + 可移植代码。
> 目标平台：Rust + GPUI（跨平台：Windows / macOS / Linux）。
> 核心要求：**动画永远可打断**、**帧率无关**、**性能自适应**。

---

## 1. 设计哲学（三条铁律）

### 1.1 弹簧即几何 —— 唯一动画源
所有动效参数（宽度、高度、圆角、位移、透明度、页面偏移、隐藏进度）都应该是**弹簧值**，
而不是独立的动画状态机。每帧的绘制参数 = f(弹簧当前值)。动画由此获得：
- 自然的**过冲回弹**（iOS 灵动岛质感的来源）
- **任意时刻可打断**（拖拽、切歌、点击都会立刻接管弹簧，无需清理动画队列）

### 1.2 一帧一算 —— 无缓存的中间态
每次重绘都重新计算 dt、弹簧值、透明度、模糊 sigma。**禁止**"先播完 A 动画再播 B 动画"的链式编排；
打断 = 直接改弹簧目标值/直接赋值弹簧值。这是可打断性的前提。

### 1.3 性能自适应 —— 画质让位于帧率
- 集显/独显使用不同参数上限（模糊、帧率、背景刷新）
- 无动画即停帧循环（零功耗）
- 低刷新率显示器自动降级

---

## 2. 动画语言规范（参数基准）

### 2.1 弹簧参数表（WinIsland 实测调校值）

| 弹簧 | stiffness | damping | 用途 | 备注 |
|---|---|---|---|---|
| 几何 w/h/r | **0.10** | **0.68** | 岛体宽/高/圆角形变 | 展开、折叠、歌词宽度自适应 |
| 页面 view | **0.12** | **0.68** | 音乐页 ↔ 小组件页平移 | 稍快、更"跟手" |
| 隐藏 hide（隐藏时） | **0.12** | **0.70** | 滑出屏幕边缘 | 干脆利落 |
| 隐藏 hide（显示时） | **0.08** | **0.78** | 从边缘弹回 | 更绵软，回弹感强 |
| 拖拽中 | — | velocity=0 | 拖拽直接赋值，松手交还弹簧 | 打断的标准动作 |

**阈值约定**：
- 收敛阈值：`|target - value| < 0.005` 视为到达（AnimPool）；弹簧用 `|velocity| < 0.001` 判断静止
- dt 归一化：`dt = (elapsed_secs * 60.0).clamp(0.1, 6.0)` —— 以 60fps 为基准，跨刷新率观感一致
- NaN 兜底：任何非有限值出现时 `value = target; velocity = 0`

### 2.2 缓动（非弹簧场景）

| 场景 | 缓动 | 时长基准 |
|---|---|---|
| hover 反馈 | `ease_in_out` | 150–250ms |
| 一次性渐入 | `ease_out` | 200–300ms |
| 循环（加载/浮动） | `ease_in_out` + repeat | 1.2–2s |
| 数值逼近（进度条等） | 指数平滑 `value += diff * 0.15` | 帧率无关 |
| 恢复隐藏宽度 | 指数 `1 - 0.78^dt` | 帧率无关 |

---

## 3. 弹簧物理实现（可移植代码）

半隐式欧拉积分。**纯 Rust，零依赖，可直接复制**。

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
}

impl Spring {
    pub fn new(value: f32) -> Self {
        Self { value, velocity: 0.0 }
    }

    /// target: 目标值; stiffness: 刚度; damping: 阻尼; dt: 归一化步长(60fps基准)
    pub fn update_dt(&mut self, target: f32, stiffness: f32, damping: f32, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        // 力 = 位移 * 刚度 * dt（半隐式欧拉：先更新速度，再更新位置）
        let force = (target - self.value) * stiffness * dt;
        self.velocity = (self.velocity + force) * damping.powf(dt);
        self.value += self.velocity * dt;
        // 数值兜底
        if !self.value.is_finite() {
            self.value = target;
            self.velocity = 0.0;
        }
        if !self.velocity.is_finite() {
            self.velocity = 0.0;
        }
    }

    /// 打断：直接设置值并清零速度（拖拽、点击接管时调用）
    pub fn snap(&mut self, value: f32) {
        self.value = value;
        self.velocity = 0.0;
    }

    pub fn is_still(&self) -> bool {
        self.velocity.abs() <= 0.001
    }
}
```

**调参直觉**：
- `stiffness` 越大越快到达；`damping` 越小越"弹"（过冲多），接近 1.0 越粘滞
- 常见组合：`(0.10, 0.68)` 轻微过冲；`(0.08, 0.78)` 几乎不过冲的柔和收敛
- 若需要强烈弹跳（如通知横幅弹出）：`(0.18, 0.60)` 并在 1–2 个周期内收敛

---

## 4. 运动模糊规范（速度感知）

### 4.1 sigma 计算（WinIsland 实测公式）

```rust
/// 由速度计算高斯模糊 sigma（用于渲染层模糊或拖影强度）
pub fn motion_sigmas(
    vel_w: f32,      // 宽度变化速度
    vel_h: f32,      // 高度变化速度
    vel_view: f32,   // 页面切换速度（0..1 弹簧值）
    current_w: f32,  // 当前岛宽
    max_sigma: (f32, f32), // (x, y) 上限
) -> (f32, f32) {
    let view_px_vel = vel_view.abs() * current_w; // 页面位移换算为像素速度
    let sx = (vel_w.abs() * 0.3 + view_px_vel * 0.4).min(max_sigma.0);
    let sy = (vel_h.abs() * 0.3).min(max_sigma.1);
    (sx, sy)
}
```

### 4.2 GPU 分级上限

| GPU | 最大 sigma (x, y) | 依据 |
|---|---|---|
| 集显 | (4.0, 3.5) | 模糊是 GPU 杀手，集显降档 |
| 独显 | (12.0, 10.0) | 高画质拖影 |

### 4.3 渲染层策略
- **有实时模糊能力**（如 Skia image filter / 平台自绘）：`sigma > 0.1` 才启用模糊，静止时 sigma=0 自动关闭
- **无实时模糊能力**（GPUI 默认）：用"速度驱动的透明度 + 拖影副本"近似——移动中岛体主层透明度降到 ~0.9，叠加一层快速淡出的残影层（残影 alpha 与 sigma 线性映射）

---

## 5. 分级帧调度规范

| 场景 | 帧率/间隔 | 说明 |
|---|---|---|
| 弹簧动画中 | 显示器刷新率（默认 144Hz，集显封顶 60Hz） | 由 `any_animating()` 决定循环存续 |
| 媒体播放（进度/频谱） | 60Hz（16.7ms） | 事件驱动优先，必要时定时 |
| 交互悬停 | ~60Hz | 事件处理器内更新 |
| 空闲 | **不调度**（0 Hz） | 事件驱动架构天然达成 |
| 隐藏/不可见 | **不调度** + 暂停音频门控 | 最低功耗 |

**实现要点（GPUI）**：
- 帧循环用 `cx.spawn_in` + `request_animation_frame` 对齐垂直同步；弹簧 `any_animating() == false` 时**退出循环**，下次动画通过 `cx.notify()` 或事件重新启动
- 更新帧率时读取显示器 `refresh_rate_millihertz`，与 144Hz 默认值取实际值
- 空闲 ≥30s 可触发平台级内存修剪（Windows: `SetProcessWorkingSetSize`）

---

## 6. 动效配方库

> 每种配方给出：触发条件、参数、实现要点。**数值层（可测试纯函数）与渲染层（元素/样式）分离**。

### 6.1 岛体几何形变（展开/折叠）★核心配方
- **触发**：点击岛体 / 媒体状态变化 / 通知到达
- **参数**：`w/h/r` 三弹簧，`stiffness 0.10, damping 0.68`
- **目标值**：紧凑态 `(base_w, base_h, base_h/2)`；展开态 `(expanded_w, expanded_h, 32)`（scale 后）
- **实现**：
  - 紧凑/展开两套子视图同时存在，用**双 alpha 交叉淡化**切换：
    - `expanded_alpha = (expansion_progress.powf(2.0)).clamp(0,1) * (1 - hide_progress)`
    - `mini_alpha = (1 - expansion_progress * 1.5).clamp(0,1) * (1 - hide_progress)`
    - 其中 `expansion_progress` 由 w 弹簧归一化：`(w - base_w) / (expanded_w - base_w)`
  - 圆角同步从胶囊形（h/2）过渡到展开态圆角
- **验收**：展开时内容先淡入、岛体慢速撑开带轻微过冲；折叠时内容先淡出、岛体快速收缩

### 6.2 页面切换（音乐页 ↔ 小组件页）
- **触发**：音乐不可用时自动切到小组件页；音乐恢复时切回
- **参数**：`view` 弹簧，`stiffness 0.12, damping 0.68`
- **实现**：整页 `translate_x = view.value * current_w`（`page_shift`），两页并排，弹簧驱动水平滑入/滑出
- **验收**：切换中途可反向，无跳变

### 6.3 歌词切换过渡
- **触发**：歌词行变化
- **结构**：`LyricState { current_text, old_text, transition, scroll_offset, scroll_pause }`
- **实现**：
  - `transition_to(text)`：旧词存入 `old_text`，`transition = 0`
  - 每帧 `transition += 0.05 * dt`，clamp 到 1.0；`transition >= 1.0` 后清空 `old_text`
  - 渲染：旧词 alpha `1 - transition`，新词 alpha `transition`，垂直微位移（新词从下方 20% 进入）
  - 超长歌词：`scroll_offset += 0.8 * dt`（像素/帧），到 overflow 停止；滚动前有 `scroll_pause` 停顿
- **验收**：切词 250ms 内完成淡入淡出；长歌词先停顿再平滑滚动

### 6.4 隐藏到边缘 / 显现
- **触发**：空闲超时（auto_hide_delay）/ 全屏 / 手动上滑拖拽
- **参数**：`hide` 弹簧——隐藏 `(0.12, 0.70)`，显示 `(0.08, 0.78)`；隐藏时给 `velocity = -0.65` 制造弹入边缘的惯性
- **实现**：
  - 计算最近屏幕边缘（Top/Bottom/Left/Right 四距离取最小）
  - 隐藏位移 = `hide.value * hide_distance`（hide_distance = 边缘方向岛体尺寸 - 保留可见宽度 + TOP_OFFSET）
  - 隐藏后保留 **1px 热区**（悬停可唤起，hit-test 用）
  - 拖拽直接操纵：`spring.snap(拖拽进度)`，松手按当前值决定隐藏/恢复
- **验收**：滑出有"被吸进边缘"的弹性；悬停 1px 热区立刻弹出

### 6.5 封面翻转（切歌动效）
- **触发**：歌曲变更
- **实现**：封面沿 X 轴压扁（scaleX 0→0.05→1）+ 切换中间点换图 + 快速模糊（sigma 峰值）
  - 相位：0→0.5 旧图压扁，0.5 换新图，0.5→1.0 新图展开
  - 时长 ~300ms，用 GPUI `with_animation` 或手动计时
- **验收**：翻转过程有"翻面"感，模糊峰值出现在压扁最深时

### 6.6 进度条跟随
- **触发**：播放进度更新 / 拖拽 seek
- **实现**：指数平滑——`smooth += (raw - smooth) * 0.15`；**拖动中直接跟手**（禁用平滑），松手后平滑恢复
  - 特殊规则：`raw` 跳变过大（切歌重置）时直接 `smooth = raw`，避免拖尾
- **验收**：秒级进度无肉眼跳变；拖拽绝对跟手

### 6.7 动态背景（专辑封面漂移）
- **参数**（独显）：旋转 `0.03 rad/s`，漂移 `sin(t*0.15)*20px / cos(t*0.12)*15px`；集显降半速降半幅
- **实现**：封面模糊图放大 1.3 倍对角线尺寸，中心旋转 + 正弦漂移，盖 120 alpha 的暗色遮罩；无封面时回落毛玻璃/纯色
- **验收**：慢速呼吸感，不抢前景内容

---

## 7. GPUI 实现映射

### 7.1 弹簧值 → GPUI 样式

```rust
use gpui::*;

// 渲染层只做"数值翻译"：弹簧值 → 样式
impl Render for IslandView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let w = self.springs.w.value;
        let h = self.springs.h.value;
        let r = self.springs.r.value;
        let hide = self.springs.hide.value;
        div()
            .size(px(w), px(h))
            .rounded(px(r))
            .bg(rgb(0x0d0f14))
            .opacity(1.0 - hide) // 隐藏时整体淡出（可选）
            .child(self.render_content()) // 内部元素同样由数值驱动
    }
}
```

### 7.2 一次性/循环动画 → GPUI Animation

```rust
use gpui::{animation::AnimationExt, Animation, ease_in_out};

// hover 呼吸放大
div().with_animation(
    "hover-scale",
    Animation::new(Duration::from_millis(200)).with_easing(ease_in_out),
    |this, delta| this.scale(1.0 + 0.04 * delta),
)
```

### 7.3 帧循环（弹簧驱动）

```rust
// 启动弹簧帧循环（仅在动画活跃时运行）
fn ensure_frame_loop(&mut self, cx: &mut Context<Self>) {
    if self.frame_loop_active { return; }
    self.frame_loop_active = true;
    cx.spawn_in(|this, cx| async move {
        loop {
            let dt = /* 距上帧秒数 * 60，clamp(0.1, 6.0) */;
            let still = this.update(cx, |view, cx| {
                let animating = view.springs.tick(dt);
                cx.notify();
                !animating
            }).unwrap_or(true);
            if still { break; }
            // 等待下一帧（GPUI 提供 request_animation_frame 则用之）
            cx.background_spawn(async { /* 等 ~1/刷新率 */ }).await;
        }
        this.update(cx, |view, _| view.frame_loop_active = false).ok();
    }).detach();
}
```

### 7.4 运动模糊近似（GPUI 无 Skia blur filter 时）

```rust
// 拖影副本：主层下方渲染一个速度方向的低透明度副本，随速度淡出
div()
    .child(primary_layer.clone().opacity(0.9 - 0.2 * normalized_sigma))
    .child(trail_layer.opacity(0.25 * normalized_sigma)) // 残影
```

---

## 8. 性能约束与降级策略

| 约束 | 集显/低端 | 独显/高端 |
|---|---|---|
| 动画帧率上限 | 60Hz | 显示器原生（≤144Hz） |
| 模糊 sigma 上限 | (4.0, 3.5) | (12.0, 10.0) |
| 毛玻璃截图刷新 | 100ms + 3x 降采样 | 33ms + 2x 降采样 |
| 动态背景速率 | 半速半幅 | 全速全幅 |
| 无动画时 | 帧循环退出 + 空闲内存修剪 | 同左 |

**降级判定**：启动时探测 GPU 型号（如 Windows DXGI adapter 描述含 Intel/AMD APU）或用户手动设置；运行中不动态切换，避免抖动。

---

## 9. 验收清单（每次动效合入前自查）

- [ ] 动画在 60Hz 与 144Hz 显示器上观感一致（dt 归一化生效）
- [ ] 动画进行中点击/拖拽可立即打断，无跳变
- [ ] 弹簧静止后帧循环已退出（无 CPU 空转）
- [ ] 集显设备无卡顿（降级参数生效）
- [ ] 运动模糊随速度渐变，静止时完全关闭
- [ ] 隐藏到边缘后 1px 热区可唤起
- [ ] 歌词切换在 250ms 内完成，长歌词先停顿再滚动
- [ ] 数值层（弹簧/alpha/sigma）为纯函数，有单元测试
- [ ] 渲染层无平台条件编译（平台差异全在 effects/media 层）

---

## 10. 参考来源

- WinIsland v1.3.0 源码：`src/utils/physics.rs`（弹簧）、`src/utils/anim.rs`（缓动池）、
  `src/utils/blur.rs`（运动模糊）、`src/window/app/frame.rs`（帧调度）、
  `src/core/render.rs`（双 alpha 交叉淡化）、`src/ui/expanded/music_view.rs`（封面翻转/进度平滑）
- GPUI 官方示例：`gpui/examples/animation.rs`；`docs.rs/gpui` 的 `Animation` / `AnimationExt` / `easing` 模块
