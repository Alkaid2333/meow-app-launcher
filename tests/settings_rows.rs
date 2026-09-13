//! 配置项数据驱动注册表喵~ 单元测试喵
//!
//! 验证 RowDesc 注册表的核心不变量: 身份唯一、脏判定、恢复默认、
//! 步进钳制与循环选项轮换喵(交互层与页面构建全部查同一张表)喵。

use meow_app_launcher::app::config::AppConfig;
use meow_app_launcher::render::settings::RowId;
use meow_app_launcher::window::settings_data::{
    apply_data_value, data_rows, dirty_data_ids, find_row, restore_data_row, RowValue,
};

#[test]
fn 数据行身份唯一喵() {
    let rows = data_rows();
    assert!(rows.len() >= 40, "注册表应覆盖全部可调项");
    let mut ids: Vec<RowId> = rows.iter().map(|d| d.id).collect();
    let n = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), n, "RowId 不应重复注册");
}

#[test]
fn 开关写入与脏判定喵() {
    let mut cfg = AppConfig::default();
    // 默认 reduce_motion = false,写 true 后应标记为脏喵
    assert!(apply_data_value(&mut cfg, RowId::ReduceMotion, RowValue::Bool(true)));
    assert!(dirty_data_ids(&cfg).contains(&RowId::ReduceMotion));
    // 重复写相同值应为 no-op(返回 false 且不脏上加脏)喵
    assert!(!apply_data_value(&mut cfg, RowId::ReduceMotion, RowValue::Bool(true)));
    assert!(dirty_data_ids(&cfg).contains(&RowId::ReduceMotion));
    // 恢复默认后干净喵
    assert!(restore_data_row(&mut cfg, RowId::ReduceMotion));
    assert!(!dirty_data_ids(&cfg).contains(&RowId::ReduceMotion));
}

#[test]
fn 步进行钳制与恢复默认喵() {
    let mut cfg = AppConfig::default();
    let (min, max, step, _) = find_row(RowId::IslandW)
        .and_then(|d| d.stepper_meta())
        .expect("胶囊宽应为步进行");
    assert_eq!(step, 4);
    // 越界值应被钳制到 min/max 喵
    assert!(apply_data_value(&mut cfg, RowId::IslandW, RowValue::Num(9999)));
    assert_eq!(
        find_row(RowId::IslandW).unwrap().value(&cfg),
        RowValue::Num(max)
    );
    assert!(apply_data_value(&mut cfg, RowId::IslandW, RowValue::Num(-1)));
    assert_eq!(
        find_row(RowId::IslandW).unwrap().value(&cfg),
        RowValue::Num(min)
    );
    // 恢复默认后不再脏喵
    assert!(restore_data_row(&mut cfg, RowId::IslandW));
    assert!(!dirty_data_ids(&cfg).contains(&RowId::IslandW));
}

#[test]
fn 循环选项按档位轮换喵() {
    let mut cfg = AppConfig::default();
    let desc = find_row(RowId::Theme).expect("主题应为循环选项行");
    let start = match desc.value(&cfg) {
        RowValue::Index(i) => i,
        _ => panic!("主题行取值应为档位下标"),
    };
    // 档位数从描述里取,不硬编码(主题现为 4 档)喵
    let n = match desc.kind {
        meow_app_launcher::window::settings_data::RowKind::Choice { options } => options.len(),
        _ => panic!("主题行应为循环选项"),
    };
    assert!(n >= 4, "主题应包含深色档");
    assert!(apply_data_value(
        &mut cfg,
        RowId::Theme,
        RowValue::Index((start + 1) % n)
    ));
    match desc.value(&cfg) {
        RowValue::Index(i) => assert_eq!(i, (start + 1) % n),
        _ => panic!("主题行取值应为档位下标"),
    }
    // 深色档可直达喵
    assert!(apply_data_value(&mut cfg, RowId::Theme, RowValue::Index(n - 1)));
    assert_eq!(desc.value(&cfg), RowValue::Index(n - 1));
    // 循环选项行当前档位名应能取到喵
    assert!(!desc.choice_text(&cfg).is_empty());
}

#[test]
fn 弹簧行带参读写往返喵() {
    let mut cfg = AppConfig::default();
    for i in 0..6u8 {
        let id = RowId::SpringDuration(i);
        // 百分数写回喵
        assert!(apply_data_value(&mut cfg, id, RowValue::Num(80)));
        assert_eq!(find_row(id).unwrap().value(&cfg), RowValue::Num(80));
        let bid = RowId::SpringBounce(i);
        assert!(apply_data_value(&mut cfg, bid, RowValue::Num(50)));
        assert_eq!(find_row(bid).unwrap().value(&cfg), RowValue::Num(50));
    }
    // 全部恢复默认喵
    for i in 0..6u8 {
        assert!(restore_data_row(&mut cfg, RowId::SpringDuration(i)));
        assert!(restore_data_row(&mut cfg, RowId::SpringBounce(i)));
    }
    assert!(dirty_data_ids(&cfg).is_empty());
}

#[test]
fn 默认配置零脏行喵() {
    let cfg = AppConfig::default();
    assert!(dirty_data_ids(&cfg).is_empty(), "默认配置不应有任何脏行");
}
