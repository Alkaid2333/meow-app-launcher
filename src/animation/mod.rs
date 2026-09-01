//! 动画内核喵~
//!
//! 原项目 island-spring 逐位移植: 弹簧即几何,永远可打断,停稳即停帧喵。

pub mod island;
pub mod springs;

pub use island::{
    DynamicIsland, IslandConfig, IslandFrame, IslandState, IslandTransition,
};
pub use springs::DurationBounce;

/// 把距上帧的秒数钳到安全范围喵
pub fn clamp_dt(elapsed_secs: f32) -> f32 {
    elapsed_secs.clamp(0.0, 1.0 / 15.0)
}
