//! 灵动岛状态机与几何喵~ 单元测试喵

use meow_app_launcher::animation::island::{
    DynamicIsland, EasingName, IslandConfig, IslandState, IslandTransition, MotionMode,
    REVEAL_ROW_SPAN, REVEAL_STAGGER, cascade,
};

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
    assert!((i.frame().morph - 1.0).abs() < 0.001);

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

#[test]
fn 呼出与展开都会重放内容级联() {
    let mut i = DynamicIsland::new(IslandConfig::default());
    i.set_stage(900.0, 500.0);
    i.transition(IslandTransition::Summon);
    assert!(i.frame().reveal < 0.001, "呼出应把级联进度拨回起点");
    settle(&mut i);
    assert!((i.frame().reveal - 1.0).abs() < 1e-6, "唤出后内容应已就位");

    i.transition(IslandTransition::Expand);
    assert!(i.frame().reveal < 0.001, "展开应把级联进度拨回起点");
    settle(&mut i);
    assert!((i.frame().reveal - 1.0).abs() < 1e-6, "级联应收尾到 1");
}

#[test]
fn 收场时级联直接定型() {
    let mut i = DynamicIsland::new(IslandConfig::default());
    i.set_stage(900.0, 500.0);
    i.transition(IslandTransition::Summon);
    i.step(1.0 / 240.0);
    assert!(i.frame().reveal < 1.0, "级联应该正在跑");

    /* 半路收起:内容必须立刻定型,不然会和淡出一层层叠影喵 */
    i.transition(IslandTransition::Dismiss);
    assert_eq!(i.frame().reveal, 1.0);
}

#[test]
fn 降低动效时级联直接落位() {
    let mut i = DynamicIsland::new(IslandConfig::default());
    i.set_stage(900.0, 500.0);
    i.config.reduce_motion = true;
    i.transition(IslandTransition::Summon);
    assert_eq!(i.frame().reveal, 1.0, "降低动效应直接落位");
}

#[test]
fn 瞬时档位不播放级联() {
    let mut i = DynamicIsland::new(IslandConfig::default());
    i.set_stage(900.0, 500.0);
    i.config.motion_mode = MotionMode::Instant;
    i.sync(false);
    i.transition(IslandTransition::SummonExpanded);
    assert!((i.frame().reveal - 1.0).abs() < 1e-6, "瞬时档位应直接落位");
}

#[test]
fn 级联按行错峰自上而下() {
    let p: Vec<f64> = (0..6)
        .map(|i| cascade(0.5, i, 6, REVEAL_STAGGER, REVEAL_ROW_SPAN))
        .collect();
    for w in p.windows(2) {
        assert!(w[0] >= w[1], "错峰顺序反了: {p:?}");
    }
    assert!(p[0] > 0.0, "首行该露头了: {p:?}");
    assert!(p[5] < 1.0, "末行还不该到位: {p:?}");

    /* 两端必须是干净的 0 / 1,渲染层才好彻底跳过或彻底落笔 */
    assert_eq!(cascade(0.0, 0, 6, REVEAL_STAGGER, REVEAL_ROW_SPAN), 0.0);
    assert_eq!(cascade(1.0, 5, 6, REVEAL_STAGGER, REVEAL_ROW_SPAN), 1.0);
    /* 只有一行时没有错峰可言,整行一起浮现 */
    assert_eq!(cascade(1.0, 0, 1, REVEAL_STAGGER, REVEAL_ROW_SPAN), 1.0);
}
