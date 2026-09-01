//! 字体管理喵~
//!
//! 缓存 Typeface,按需产出不同字号的 Skia Font 喵。
//! 默认用「微软雅黑」保证中文应用名渲染清晰,缺失时回落系统默认字体喵。
//! 产出字体统一关闭亚像素、开启微对齐,保证透明分层窗口上文字锐利不虚喵。

use skia_safe::{Font, FontHinting, FontMgr, FontStyle, Typeface};

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

    /// 按字号产出一个字体喵(字号取整,笔画落格更锐利)喵
    pub fn font(&self, size: f32) -> Font {
        // 非整数字号会让字形轮廓落在亚像素上,中文笔画发虚;取整后配合
        // 关闭亚像素 + 轻微 hinting,在透明分层窗口上保持锐利喵
        let size = size.round().max(1.0);
        let mut font = Font::new(self.typeface.clone(), size);
        font.set_subpixel(false);
        font.set_hinting(FontHinting::Slight);
        font
    }
}

impl Default for FontCache {
    fn default() -> Self {
        Self::new()
    }
}
