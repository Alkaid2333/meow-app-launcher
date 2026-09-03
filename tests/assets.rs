//! 资源资产可用性喵~ 校验内置素材能被正确使用喵

use skia_safe::{Data, Image};

#[test]
fn app_icon_png_decodes() {
    // 配置 GUI 品牌徽标用的 png 必须能被 Skia 解码喵
    let bytes = include_bytes!("../assets/app_icons/128x128.png");
    let img = Image::from_encoded(Data::new_copy(bytes)).expect("应用图标 png 解码失败喵~");
    assert_eq!(img.width(), 128);
    assert_eq!(img.height(), 128);
}

#[test]
fn app_icon_ico_is_real_ico() {
    // 打包 exe 用的 .ico 必须是真正的 ICO(头: reserved=0, type=1, count>0)喵
    let bytes = include_bytes!("../assets/app_icons/app-icon.ico");
    assert_eq!(bytes[0], 0);
    assert_eq!(bytes[1], 0);
    assert_eq!(bytes[2], 1);
    assert_eq!(bytes[3], 0);
    let count = u16::from_le_bytes([bytes[4], bytes[5]]);
    assert!(count >= 3, "ico 应含多个分辨率图标,实际 {count} 喵");
}
