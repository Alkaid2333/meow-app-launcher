//! 配置项数据驱动注册表喵~
//!
//! 把「开关 / 数值 / 循环选项」三类可调配置项的全部元数据(身份、标签、归属、
//! 读写函数)收进一张 [`RowDesc`] 表喵。新增一个配置项只需要在 [`data_rows`]
//! 里注册一次,页面构建、命中处理、脏判定、恢复默认全部自动生效喵——
//! 把「改 4 处」的成本压到「改 1 处」喵(ROADMAP D5 决策)。
//!
//! 动作按钮 / 输入框 / 应用行等结构型行不在此表,仍由页面层手工编排喵。

use crate::animation::island::{EasingName, MotionMode};
use crate::app::config::{
    AppConfig, AppLayout, ElevateModifier, RenderBackend, SearchMode, ThemePreset, WebEngine,
};
use crate::platform::ShellKind;

use crate::render::settings::RowId;

// ---------------------------------------------------------------------------
// 页面归属常量喵(与 build_pages 的页面顺序一致)喵
// ---------------------------------------------------------------------------

/// 常规页索引喵
pub const PAGE_GENERAL: usize = 0;
/// 灵动岛页索引喵
pub const PAGE_ISLAND: usize = 1;
/// 弹簧页索引喵
pub const PAGE_SPRING: usize = 2;

/// 数据行的取值喵(覆盖 开关/数值/选项 三类)喵
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RowValue {
    /// 布尔开关喵
    Bool(bool),
    /// 数值喵(步进行展示与提交都用它)喵
    Num(i32),
    /// 循环选项的档位下标喵
    Index(usize),
}

/// 数值行的调节规格喵
#[derive(Debug, Clone, Copy)]
pub enum RowKind {
    /// 布尔开关喵
    Switch,
    /// 数值步进喵
    Stepper {
        /// 允许的最小值喵
        min: i32,
        /// 允许的最大值喵
        max: i32,
        /// 每次点击的步幅喵
        step: i32,
        /// 单位文案喵
        unit: &'static str,
    },
    /// 循环选项喵(点击在档位间轮换)喵
    Choice {
        /// 各档位显示名喵(与枚举 ALL 顺序一致)喵
        options: &'static [&'static str],
    },
}

/// 配置项描述喵: 一个可调项的全部元数据喵
#[derive(Clone, Copy)]
pub struct RowDesc {
    /// 行身份喵(命中测试与编辑槽的 key)喵
    pub id: RowId,
    /// 行标签喵
    pub label: &'static str,
    /// 归属页索引喵
    pub page: usize,
    /// 归属分组标题喵
    pub group: &'static str,
    /// 参数下标喵(弹簧行为 0..6,其余为 0)喵
    pub index: usize,
    /// 调节规格喵
    pub kind: RowKind,
    /// 读配置喵
    pub get: fn(&AppConfig, usize) -> RowValue,
    /// 写配置喵,返回是否变化喵
    pub set: fn(&mut AppConfig, usize, RowValue) -> bool,
}

impl RowDesc {
    /// 当前显示值喵
    pub fn value(&self, cfg: &AppConfig) -> RowValue {
        (self.get)(cfg, self.index)
    }

    /// 循环选项的当前档位名喵(仅 Choice 行有意义)喵
    pub fn choice_text(&self, cfg: &AppConfig) -> String {
        match (self.kind, self.value(cfg)) {
            (RowKind::Choice { options }, RowValue::Index(i)) => {
                options.get(i).copied().unwrap_or("?").to_string()
            }
            _ => String::new(),
        }
    }

    /// 数值步进规格喵(仅 Stepper 行有意义)喵
    pub fn stepper_meta(&self) -> Option<(i32, i32, i32, &'static str)> {
        match self.kind {
            RowKind::Stepper { min, max, step, unit } => Some((min, max, step, unit)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// 枚举 ↔ 档位下标 的读写辅助喵
// ---------------------------------------------------------------------------

/// 帧率档位表喵(0 = 跟随屏刷)喵
const FPS_STEPS: [u32; 6] = [0, 30, 60, 90, 120, 144];
/// 帧率档位显示名喵(与 FPS_STEPS 对齐)喵
const FPS_OPTIONS: [&str; 6] = ["自动(屏刷)", "30 Hz", "60 Hz", "90 Hz", "120 Hz", "144 Hz"];

/// 枚举转档位下标喵
fn idx_of<T: PartialEq>(all: &[T], cur: T) -> RowValue {
    RowValue::Index(all.iter().position(|v| *v == cur).unwrap_or(0))
}

/// 档位下标写回枚举喵(非法值不写,档位未变时 no-op)喵
fn write_idx<T: Copy + PartialEq>(all: &[T], v: RowValue, cur: T, f: impl FnOnce(T)) -> bool {
    let RowValue::Index(i) = v else {
        return false;
    };
    let Some(t) = all.get(i) else {
        return false;
    };
    *t != cur && {
        f(*t);
        true
    }
}

/// 布尔值写回喵(仅接受 Bool,值未变时 no-op)喵
fn write_bool(v: RowValue, cur: bool, f: impl FnOnce(bool)) -> bool {
    let RowValue::Bool(b) = v else {
        return false;
    };
    cur != b && {
        f(b);
        true
    }
}

/// 数值写回喵(带钳制,f64 目标字段)喵
fn write_f64(v: RowValue, min: f64, max: f64, cur: f64, f: impl FnOnce(f64)) -> bool {
    match v {
        RowValue::Num(n) => {
            let t = (n as f64).clamp(min, max);
            if (cur - t).abs() > f64::EPSILON {
                f(t);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

/// 比率字段写回喵(百分数形式)喵
fn write_percent_f64(v: RowValue, min: f64, max: f64, cur: f64, f: impl FnOnce(f64)) -> bool {
    match v {
        RowValue::Num(n) => {
            let t = (n as f64 / 100.0).clamp(min, max);
            if (cur - t).abs() > f64::EPSILON {
                f(t);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// 弹簧下标换算喵
// ---------------------------------------------------------------------------

/// 六段过渡下标 → 过渡类型喵
pub fn spring_at(i: usize) -> crate::animation::IslandTransition {
    use crate::animation::IslandTransition as T;
    match i % 6 {
        0 => T::Summon,
        1 => T::Dismiss,
        2 => T::Expand,
        3 => T::Collapse,
        4 => T::SummonExpanded,
        _ => T::DismissExpanded,
    }
}

/// 六段过渡的显示名喵
pub fn spring_label(i: usize) -> &'static str {
    use crate::animation::IslandTransition as T;
    match spring_at(i) {
        T::Summon => "唤出",
        T::Dismiss => "收回",
        T::Expand => "展开",
        T::Collapse => "收起",
        T::SummonExpanded => "直达展开",
        T::DismissExpanded => "直达收回",
    }
}

// ---------------------------------------------------------------------------
// 弹簧行读写喵(index = 过渡下标)喵
// ---------------------------------------------------------------------------

fn get_spring_duration(c: &AppConfig, i: usize) -> RowValue {
    RowValue::Num((c.island.springs.get(spring_at(i)).duration * 100.0).round() as i32)
}

fn set_spring_duration(c: &mut AppConfig, i: usize, v: RowValue) -> bool {
    let RowValue::Num(n) = v else {
        return false;
    };
    let tune = c.island.springs.get_mut(spring_at(i));
    let target = (n as f64 / 100.0).clamp(0.05, 2.0);
    if (tune.duration - target).abs() > f64::EPSILON {
        tune.duration = target;
        true
    } else {
        false
    }
}

fn get_spring_bounce(c: &AppConfig, i: usize) -> RowValue {
    RowValue::Num((c.island.springs.get(spring_at(i)).bounce * 100.0).round() as i32)
}

fn set_spring_bounce(c: &mut AppConfig, i: usize, v: RowValue) -> bool {
    let RowValue::Num(n) = v else {
        return false;
    };
    let tune = c.island.springs.get_mut(spring_at(i));
    let target = (n as f64 / 100.0).clamp(0.0, 0.95);
    if (tune.bounce - target).abs() > f64::EPSILON {
        tune.bounce = target;
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// 注册表本体喵(新增配置项 = 在这里加一条)喵
// ---------------------------------------------------------------------------

/// 全部数据驱动的配置项喵(注册顺序 = 页面显示顺序)喵
pub fn data_rows() -> Vec<RowDesc> {
    let mut rows = vec![
        // ---- 常规 · 外观 ----
        RowDesc {
            id: RowId::Theme,
            label: "主题",
            page: PAGE_GENERAL,
            group: "外观",
            index: 0,
            kind: RowKind::Choice {
                options: &["毛玻璃", "云母", "不透明", "深色"],
            },
            get: |c, _| idx_of(&ThemePreset::ALL, c.theme.preset),
            set: |c, _, v| write_idx(&ThemePreset::ALL, v, c.theme.preset, |t| c.theme.preset = t),
        },
        // ---- 常规 · 热键 ----
        RowDesc {
            id: RowId::HotkeyEnabled,
            label: "启用全局热键",
            page: PAGE_GENERAL,
            group: "热键",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.hotkey.enabled),
            set: |c, _, v| write_bool(v, c.hotkey.enabled, |b| c.hotkey.enabled = b),
        },
        RowDesc {
            id: RowId::ElevateModifier,
            label: "提权启动修饰键",
            page: PAGE_GENERAL,
            group: "热键",
            index: 0,
            kind: RowKind::Choice {
                options: &["关闭", "Shift", "Ctrl", "Alt", "Win"],
            },
            get: |c, _| idx_of(&ElevateModifier::ALL, c.elevate_modifier),
            set: |c, _, v| {
                write_idx(&ElevateModifier::ALL, v, c.elevate_modifier, |m| {
                    c.elevate_modifier = m
                })
            },
        },
        RowDesc {
            id: RowId::ScanHotkeyEnabled,
            label: "启用扫描热键",
            page: PAGE_GENERAL,
            group: "热键",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.scan_hotkey.enabled),
            set: |c, _, v| write_bool(v, c.scan_hotkey.enabled, |b| c.scan_hotkey.enabled = b),
        },
        // ---- 常规 · 显示 ----
        RowDesc {
            id: RowId::ShowRecent,
            label: "显示最近打开",
            page: PAGE_GENERAL,
            group: "显示",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.window.show_recent),
            set: |c, _, v| write_bool(v, c.window.show_recent, |b| c.window.show_recent = b),
        },
        RowDesc {
            id: RowId::ShowFavorites,
            label: "显示收藏",
            page: PAGE_GENERAL,
            group: "显示",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.window.show_favorites),
            set: |c, _, v| write_bool(v, c.window.show_favorites, |b| c.window.show_favorites = b),
        },
        RowDesc {
            id: RowId::ShowFrequent,
            label: "显示最常用",
            page: PAGE_GENERAL,
            group: "显示",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.window.show_frequent),
            set: |c, _, v| write_bool(v, c.window.show_frequent, |b| c.window.show_frequent = b),
        },
        RowDesc {
            id: RowId::ShowAll,
            label: "始终显示全部",
            page: PAGE_GENERAL,
            group: "显示",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.window.show_all),
            set: |c, _, v| write_bool(v, c.window.show_all, |b| c.window.show_all = b),
        },
        RowDesc {
            id: RowId::IconSize,
            label: "图标大小",
            page: PAGE_GENERAL,
            group: "显示",
            index: 0,
            kind: RowKind::Stepper { min: 24, max: 64, step: 2, unit: "px" },
            get: |c, _| RowValue::Num(c.window.icon_size as i32),
            set: |c, _, v| match v {
                RowValue::Num(n) => {
                    let t = (n as f32).clamp(24.0, 64.0);
                    if (c.window.icon_size - t).abs() > f32::EPSILON {
                        c.window.icon_size = t;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
        },
        RowDesc {
            id: RowId::SearchMode,
            label: "默认模式",
            page: PAGE_GENERAL,
            group: "显示",
            index: 0,
            kind: RowKind::Choice {
                options: &["名称", "标签 t:", "首字母 i:"],
            },
            get: |c, _| idx_of(&SearchMode::ALL, c.search.default_mode),
            set: |c, _, v| write_idx(&SearchMode::ALL, v, c.search.default_mode, |t| c.search.default_mode = t),
        },
        RowDesc {
            id: RowId::AppLayout,
            label: "结果排版",
            page: PAGE_GENERAL,
            group: "显示",
            index: 0,
            kind: RowKind::Choice { options: &["网格", "列表"] },
            get: |c, _| idx_of(&AppLayout::ALL, c.window.layout),
            set: |c, _, v| write_idx(&AppLayout::ALL, v, c.window.layout, |t| c.window.layout = t),
        },
        RowDesc {
            id: RowId::SettingsAnim,
            label: "配置面板动效",
            page: PAGE_GENERAL,
            group: "显示",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.settings_anim),
            set: |c, _, v| write_bool(v, c.settings_anim, |b| c.settings_anim = b),
        },
        // ---- 常规 · Web 搜索 ----
        RowDesc {
            id: RowId::WebEngine,
            label: "搜索引擎",
            page: PAGE_GENERAL,
            group: "Web 搜索",
            index: 0,
            kind: RowKind::Choice {
                options: &["百度", "必应", "搜狗", "谷歌", "DuckDuckGo"],
            },
            get: |c, _| idx_of(&WebEngine::ALL, c.search.web_engine),
            set: |c, _, v| write_idx(&WebEngine::ALL, v, c.search.web_engine, |t| c.search.web_engine = t),
        },
        // ---- 常规 · 指令模块(shell 选择,与指令卡片同组显示)----
        RowDesc {
            id: RowId::ShellKind,
            label: "指令执行 Shell",
            page: PAGE_GENERAL,
            group: "指令模块",
            index: 0,
            kind: RowKind::Choice {
                options: &["PowerShell", "Cmd"],
            },
            get: |c, _| idx_of(&ShellKind::ALL, c.search.shell),
            set: |c, _, v| write_idx(&ShellKind::ALL, v, c.search.shell, |t| c.search.shell = t),
        },
        // ---- 常规 · 系统 ----
        RowDesc {
            id: RowId::AutoStart,
            label: "开机自启",
            page: PAGE_GENERAL,
            group: "系统",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.auto_start),
            set: |c, _, v| write_bool(v, c.auto_start, |b| c.auto_start = b),
        },
        RowDesc {
            id: RowId::RenderBackend,
            label: "渲染后端",
            page: PAGE_GENERAL,
            group: "系统",
            index: 0,
            kind: RowKind::Choice { options: &["CPU", "GPU"] },
            get: |c, _| idx_of(&RenderBackend::ALL, c.render_backend),
            set: |c, _, v| write_idx(&RenderBackend::ALL, v, c.render_backend, |t| c.render_backend = t),
        },
        // ---- 灵动岛 · 几何 ----
        RowDesc {
            id: RowId::IslandW,
            label: "胶囊宽",
            page: PAGE_ISLAND,
            group: "几何",
            index: 0,
            kind: RowKind::Stepper { min: 160, max: 640, step: 4, unit: "px" },
            get: |c, _| RowValue::Num(c.island.width as i32),
            set: |c, _, v| write_f64(v, 160.0, 640.0, c.island.width, |t| c.island.width = t),
        },
        RowDesc {
            id: RowId::IslandH,
            label: "胶囊高",
            page: PAGE_ISLAND,
            group: "几何",
            index: 0,
            kind: RowKind::Stepper { min: 32, max: 80, step: 2, unit: "px" },
            get: |c, _| RowValue::Num(c.island.height as i32),
            set: |c, _, v| write_f64(v, 32.0, 80.0, c.island.height, |t| c.island.height = t),
        },
        RowDesc {
            id: RowId::IslandX,
            label: "水平锚点",
            page: PAGE_ISLAND,
            group: "几何",
            index: 0,
            kind: RowKind::Stepper { min: 2, max: 98, step: 1, unit: "%" },
            get: |c, _| RowValue::Num(c.island.x as i32),
            set: |c, _, v| write_f64(v, 2.0, 98.0, c.island.x, |t| c.island.x = t),
        },
        RowDesc {
            id: RowId::IslandY,
            label: "垂直锚点",
            page: PAGE_ISLAND,
            group: "几何",
            index: 0,
            kind: RowKind::Stepper { min: 2, max: 98, step: 1, unit: "%" },
            get: |c, _| RowValue::Num(c.island.y as i32),
            set: |c, _, v| write_f64(v, 2.0, 98.0, c.island.y, |t| c.island.y = t),
        },
        RowDesc {
            id: RowId::ExpandedW,
            label: "展开宽",
            page: PAGE_ISLAND,
            group: "几何",
            index: 0,
            kind: RowKind::Stepper { min: 280, max: 900, step: 10, unit: "px" },
            get: |c, _| RowValue::Num(c.island.expanded_width as i32),
            set: |c, _, v| {
                write_f64(v, 280.0, 900.0, c.island.expanded_width, |t| c.island.expanded_width = t)
            },
        },
        RowDesc {
            id: RowId::ExpandedH,
            label: "展开高",
            page: PAGE_ISLAND,
            group: "几何",
            index: 0,
            kind: RowKind::Stepper { min: 160, max: 720, step: 8, unit: "px" },
            get: |c, _| RowValue::Num(c.island.expanded_height as i32),
            set: |c, _, v| {
                write_f64(v, 160.0, 720.0, c.island.expanded_height, |t| c.island.expanded_height = t)
            },
        },
        RowDesc {
            id: RowId::ExpandedR,
            label: "展开圆角",
            page: PAGE_ISLAND,
            group: "几何",
            index: 0,
            kind: RowKind::Stepper { min: 8, max: 48, step: 2, unit: "px" },
            get: |c, _| RowValue::Num(c.island.expanded_radius as i32),
            set: |c, _, v| {
                write_f64(v, 8.0, 48.0, c.island.expanded_radius, |t| c.island.expanded_radius = t)
            },
        },
        RowDesc {
            id: RowId::InputRatio,
            label: "输入槽占比",
            page: PAGE_ISLAND,
            group: "几何",
            index: 0,
            kind: RowKind::Stepper { min: 20, max: 100, step: 1, unit: "%" },
            get: |c, _| RowValue::Num((c.island.input_ratio * 100.0).round() as i32),
            set: |c, _, v| {
                write_percent_f64(v, 0.2, 1.0, c.island.input_ratio, |t| c.island.input_ratio = t)
            },
        },
        RowDesc {
            id: RowId::Margin,
            label: "安全边距",
            page: PAGE_ISLAND,
            group: "几何",
            index: 0,
            kind: RowKind::Stepper { min: 0, max: 80, step: 2, unit: "px" },
            get: |c, _| RowValue::Num(c.island.margin as i32),
            set: |c, _, v| write_f64(v, 0.0, 80.0, c.island.margin, |t| c.island.margin = t),
        },
        RowDesc {
            id: RowId::Squash,
            label: "收起压扁",
            page: PAGE_ISLAND,
            group: "几何",
            index: 0,
            kind: RowKind::Stepper { min: 5, max: 100, step: 1, unit: "%" },
            get: |c, _| RowValue::Num((c.island.summon_squash * 100.0).round() as i32),
            set: |c, _, v| {
                write_percent_f64(v, 0.05, 1.0, c.island.summon_squash, |t| c.island.summon_squash = t)
            },
        },
        // ---- 灵动岛 · 行为 ----
        RowDesc {
            id: RowId::AutoMorph,
            label: "输入自动展开",
            page: PAGE_ISLAND,
            group: "行为",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.island.auto_morph),
            set: |c, _, v| write_bool(v, c.island.auto_morph, |b| c.island.auto_morph = b),
        },
        RowDesc {
            id: RowId::Draggable,
            label: "允许拖拽定位",
            page: PAGE_ISLAND,
            group: "行为",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.island.draggable),
            set: |c, _, v| write_bool(v, c.island.draggable, |b| c.island.draggable = b),
        },
        RowDesc {
            id: RowId::ReduceMotion,
            label: "减少动效",
            page: PAGE_ISLAND,
            group: "行为",
            index: 0,
            kind: RowKind::Switch,
            get: |c, _| RowValue::Bool(c.island.reduce_motion),
            set: |c, _, v| write_bool(v, c.island.reduce_motion, |b| c.island.reduce_motion = b),
        },
        RowDesc {
            id: RowId::AnimFps,
            label: "动画帧率",
            page: PAGE_ISLAND,
            group: "行为",
            index: 0,
            kind: RowKind::Choice { options: &FPS_OPTIONS },
            get: |c, _| {
                RowValue::Index(FPS_STEPS.iter().position(|&f| f == c.island.anim_fps).unwrap_or(0))
            },
            set: |c, _, v| match v {
                RowValue::Index(i) => {
                    let fps = FPS_STEPS.get(i).copied().unwrap_or(0);
                    if c.island.anim_fps != fps {
                        c.island.anim_fps = fps;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
        },
        RowDesc {
            id: RowId::MotionMode,
            label: "运动引擎",
            page: PAGE_ISLAND,
            group: "行为",
            index: 0,
            kind: RowKind::Choice {
                options: &["弹簧", "缓动", "线性", "瞬切"],
            },
            get: |c, _| idx_of(&MotionMode::ALL, c.island.motion_mode),
            set: |c, _, v| write_idx(&MotionMode::ALL, v, c.island.motion_mode, |t| c.island.motion_mode = t),
        },
        RowDesc {
            id: RowId::Easing,
            label: "缓动曲线",
            page: PAGE_ISLAND,
            group: "行为",
            index: 0,
            kind: RowKind::Choice {
                options: &[
                    "线性",
                    "二次缓出",
                    "四次缓出",
                    "五次缓出",
                    "指数缓出",
                    "三次缓入缓出",
                    "回弹缓出",
                ],
            },
            get: |c, _| idx_of(&EasingName::ALL, c.island.easing),
            set: |c, _, v| write_idx(&EasingName::ALL, v, c.island.easing, |t| c.island.easing = t),
        },
    ];

    // ---- 弹簧页 · 六段过渡喵(时长 + 弹性各 6 行) ----
    for i in 0..6usize {
        rows.push(RowDesc {
            id: RowId::SpringDuration(i as u8),
            label: "时长",
            page: PAGE_SPRING,
            group: "过渡参数",
            index: i,
            kind: RowKind::Stepper { min: 5, max: 200, step: 10, unit: "ms" },
            get: get_spring_duration,
            set: set_spring_duration,
        });
        rows.push(RowDesc {
            id: RowId::SpringBounce(i as u8),
            label: "弹性",
            page: PAGE_SPRING,
            group: "过渡参数",
            index: i,
            kind: RowKind::Stepper { min: 0, max: 95, step: 5, unit: "%" },
            get: get_spring_bounce,
            set: set_spring_bounce,
        });
    }
    rows
}

/// 按 id 查描述喵
pub fn find_row(id: RowId) -> Option<RowDesc> {
    data_rows().into_iter().find(|d| d.id == id)
}

/// 写回配置值喵(按 id 查表),返回是否变化喵
pub fn apply_data_value(cfg: &mut AppConfig, id: RowId, value: RowValue) -> bool {
    match find_row(id) {
        Some(desc) => (desc.set)(cfg, desc.index, value),
        None => false,
    }
}

/// 计算偏离默认值的数据行喵(通用: 当前值 ≠ 默认值即脏)喵
pub fn dirty_data_ids(cfg: &AppConfig) -> Vec<RowId> {
    let d = AppConfig::default();
    data_rows()
        .into_iter()
        .filter(|desc| desc.value(cfg) != desc.value(&d))
        .map(|desc| desc.id)
        .collect()
}

/// 恢复单个数据行到默认值喵,返回是否变化喵
pub fn restore_data_row(cfg: &mut AppConfig, id: RowId) -> bool {
    let d = AppConfig::default();
    match find_row(id) {
        Some(desc) => {
            let default_v = desc.value(&d);
            if desc.value(cfg) == default_v {
                return false;
            }
            (desc.set)(cfg, desc.index, default_v)
        }
        None => false,
    }
}

/// 读取某数据行当前值喵(数值框进入编辑态时取显示值)喵
pub fn data_value(cfg: &AppConfig, id: RowId) -> Option<RowValue> {
    find_row(id).map(|d| d.value(cfg))
}

/// 判断是否数据驱动行喵(命中分发用: 表里查得到就是)喵
pub fn is_data_row(id: RowId) -> bool {
    find_row(id).is_some()
}
