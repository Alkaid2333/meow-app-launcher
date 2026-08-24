//! 字体管理喵~
//!
//! 缓存 Typeface,按需产出不同字号的 Skia Font 喵。
//! 默认用「微软雅黑」保证中文应用名渲染清晰,缺失时回落系统默认字体喵。

use skia_safe::{Font, FontMgr, FontStyle, Typeface};

/// 字体缓存喵
pub struct FontCache {
    typeface: Typeface,
}

impl FontCache {
    /// 造一个字体缓存喵
    pub fn new() -> Self {
        // 优先微软雅黑(中文渲染),缺失回落系统默认喵
        let mgr = FontMgr::default();
        let typeface = mgr
            .match_family_style("Microsoft YaHei", FontStyle::normal())
            .or_else(|| mgr.match_family_style("Segoe UI", FontStyle::normal()))
            .or_else(|| mgr.legacy_make_typeface(None, FontStyle::normal()))
            .expect("无法获取系统字体喵~");
        Self { typeface }
    }

    /// 按字号产出一个字体喵
    pub fn font(&self, size: f32) -> Font {
        Font::new(self.typeface.clone(), size)
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}
