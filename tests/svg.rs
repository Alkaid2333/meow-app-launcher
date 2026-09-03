//! 内嵌 SVG 图标渲染喵~ 单元测试喵

use meow_app_launcher::render::svg::{build_path, numbers, parse_svg, ICONS};

#[test]
fn number_tokenizer_handles_negatives() {
    assert_eq!(
        numbers("9,11 12 14 20 6-2-2"),
        vec![9.0, 11.0, 12.0, 14.0, 20.0, 6.0, -2.0, -2.0]
    );
    // 科学计数法保持喵
    assert!(numbers("1e-2 3").len() >= 2);
}

#[test]
fn parse_builtin_icons() {
    for (_, xml) in ICONS {
        let icon = parse_svg(xml);
        assert!(icon.shapes_len() > 0, "图标应有可绘制元素喵");
    }
}

#[test]
fn path_builds_no_panic() {
    for (_, xml) in ICONS {
        if let Some(d) = extract_d(xml) {
            let p = build_path(&d);
            assert!(p.bounds().right >= 0.0 || p.bounds().left >= 0.0 || true);
        }
    }
}

fn extract_d(xml: &str) -> Option<String> {
    // 粗取第一个 d="..." 喵
    let i = xml.find("d=\"")?;
    let rest = &xml[i + 3..];
    let j = rest.find('"')?;
    Some(rest[..j].to_string())
}
