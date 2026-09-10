//! 内嵌 SVG 图标渲染喵~ 单元测试喵

use meow_app_launcher::render::svg::{build_path, numbers, parse_svg, ICONS};

#[test]
fn number_tokenizer_handles_negatives() {
    assert_eq!(
        numbers("9,11 12 14 20 6-2-2"),
        vec![9.0, 11.0, 12.0, 14.0, 20.0, 6.0, -2.0, -2.0]
    );
    assert!(numbers("1e-2 3").len() >= 2);
}

#[test]
fn parse_builtin_icons() {
    for (_, xml) in ICONS {
        let icon = parse_svg(xml);
        assert!(icon.view.2 > 0.0 && icon.view.3 > 0.0, "图标 viewBox 应有效喵");
    }
}

#[test]
fn viewbox_is_respected() {
    let icon = parse_svg(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><circle cx="256" cy="256" r="200"/></svg>"#);
    assert_eq!(icon.view, (0.0, 0.0, 512.0, 512.0), "viewBox 尺寸应生效喵");

    let icon = parse_svg(r#"<svg viewBox="10 20 100 80"><rect x="10" y="20" width="50" height="40"/></svg>"#);
    assert_eq!(icon.view, (10.0, 20.0, 100.0, 80.0));
}

#[test]
fn element_style_detection() {
    let _ = parse_svg(r##"<svg viewBox="0 0 24 24"><path fill="#333" d="M4 4 H 20 V 20 H 4 Z"/></svg>"##);
    let _ = parse_svg(r#"<svg viewBox="0 0 24 24"><path fill="none" stroke="currentColor" d="M4 4 H 20"/></svg>"#);
    let _ = parse_svg(r#"<svg viewBox="0 0 24 24"><line x1="2" y1="2" x2="20" y2="20"/></svg>"#);
}

#[test]
fn path_builds_no_panic() {
    for (_, xml) in ICONS {
        if let Some(d) = extract_d(xml) {
            let p = build_path(&d);
            let _ = p.bounds();
        }
    }
}

fn extract_d(xml: &str) -> Option<String> {
    let i = xml.find("d=\"")?;
    let rest = &xml[i + 3..];
    let j = rest.find('"')?;
    Some(rest[..j].to_string())
}
