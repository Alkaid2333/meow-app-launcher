//! 弹簧引擎物理参数喵~ 单元测试喵

use meow_app_launcher::animation::springs::{Spring, SpringParams};

/// 逐位对齐 js/spring.js（下面这些数字是 node 端 toPrecision(17) 打出来的）
#[test]
fn 与_js_端物理参数逐位一致() {
    let cases = [
        ("summon", 0.55, 0.28, 285.129_891_111_943_21, 17.279_121_362_376_685),
        ("dismiss", 0.40, 0.00, 300.894_930_694_731_61, 34.692_646_523_131_188),
        ("expand", 0.62, 0.14, 153.394_778_303_530_39, 15.662_043_614_039_112),
        ("collapse", 0.48, 0.10, 227.232_209_494_581_92, 20.450_958_502_924_060),
        ("summonExp", 0.74, 0.22, 134.411_382_679_802_59, 12.936_345_176_748_603),
        ("dismissExp", 0.52, 0.04, 158.421_313_941_461_08, 19.435_835_696_521_931),
        ("tune", 0.28, 0.00, 614.071_287_132_105_34, 49.560_923_604_473_125),
    ];
    for (name, d, b, k, c) in cases {
        let p = SpringParams::from_duration_bounce(d, b, 1.0);
        assert!(
            (p.stiffness - k).abs() < 1e-9,
            "{name}: stiffness {} ≠ {k}",
            p.stiffness
        );
        assert!(
            (p.damping - c).abs() < 1e-9,
            "{name}: damping {} ≠ {c}",
            p.damping
        );
    }
}

#[test]
fn duration_与物理参数可往返() {
    let mut d = 0.2;
    while d <= 1.5 {
        let mut b = 0.0;
        while b <= 0.9 {
            let p = SpringParams::from_duration_bounce(d, b, 1.3);
            let back = p.to_duration_bounce();
            assert!((back.duration - d).abs() < 1e-9, "duration {d}");
            assert!((back.bounce - b).abs() < 1e-9, "bounce {b}");
            b += 0.05;
        }
        d += 0.05;
    }
}

#[test]
fn 过冲量随_bounce_单调上升() {
    let mut prev = -1.0;
    let mut b = 0.0;
    while b <= 0.9001 {
        let p = SpringParams::from_duration_bounce(0.5, b, 1.0);
        let mut s = Spring::new(0.0, p, 0.001, 0.001);
        s.set(100.0);
        let mut over: f64 = 0.0;
        for _ in 0..4000 {
            s.step(1.0 / 240.0);
            over = over.max(s.value - 100.0);
        }
        assert!(over >= prev - 0.01, "bounce {b} 过冲 {over} 小于上一档 {prev}");
        prev = over;
        b += 0.15;
    }
    // bounce=0.3 时过冲应在 15% 附近（js 端实测 15.7%）
    let p = SpringParams::from_duration_bounce(0.5, 0.3, 1.0);
    let mut s = Spring::new(0.0, p, 0.001, 0.001);
    s.set(100.0);
    let mut over: f64 = 0.0;
    for _ in 0..4000 {
        s.step(1.0 / 240.0);
        over = over.max(s.value - 100.0);
    }
    assert!((over - 15.7).abs() < 0.5, "bounce=0.3 过冲 {over} 应约 15.7");
}
