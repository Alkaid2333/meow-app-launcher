//! 图标管理器喵~
//!
//! 负责: 从应用路径提取图标 → 存成 PNG 快照到 `./.datas/icons/` → 返回路径喵。
//! 提取失败返回 None,渲染层用「应用名前两字符 + 默认背景」合成兜底图标喵。

use super::AppInfo;
use crate::platform::PlatformCapabilities;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// 图标目录名(相对数据目录)喵
pub const ICON_DIR_NAME: &str = "icons";

/// 图标管理器喵
pub struct IconManager {
    data_dir: PathBuf,
    platform: Arc<dyn PlatformCapabilities>,
    /// 内存缓存: 应用名 → 图标快照路径(避免反复读盘喵)
    cache: HashMap<String, Option<PathBuf>>,
}

impl IconManager {
    pub fn new(data_dir: PathBuf, platform: Arc<dyn PlatformCapabilities>) -> Self {
        let icons_dir = data_dir.join(ICON_DIR_NAME);
        // 确保图标目录存在喵
        if let Err(e) = std::fs::create_dir_all(&icons_dir) {
            log::error!("创建图标目录失败: {e}");
        }
        Self {
            data_dir,
            platform,
            cache: HashMap::new(),
        }
    }

    /// 获取应用的图标快照路径喵
    ///
    /// 优先用自定义图标路径,其次提取文件图标缓存,最后 None(字符兜底)喵
    pub fn icon_path(&mut self, app: &AppInfo) -> Option<PathBuf> {
        // 1. 自定义图标喵
        if let Some(icon) = &app.icon_path {
            let p = PathBuf::from(icon);
            if p.exists() {
                return Some(p);
            }
            log::warn!("自定义图标不存在: {icon},回退自动提取喵");
        }

        // 2. 内存缓存喵
        if let Some(cached) = self.cache.get(&app.name) {
            return cached.clone();
        }

        // 3. 提取并缓存快照喵
        let result = self.extract_and_snapshot(app);
        self.cache.insert(app.name.clone(), result.clone());
        result
    }

    /// 提取文件图标并存成 PNG 快照喵
    fn extract_and_snapshot(&mut self, app: &AppInfo) -> Option<PathBuf> {
        let snapshot = self.data_dir.join(ICON_DIR_NAME).join(app.icon_snapshot_name());

        // 已存在快照就直接用喵(文件图标一般不会变)喵
        if snapshot.exists() {
            return Some(snapshot);
        }

        // 提取像素喵
        let pixels = self.platform.extract_icon_pixels(&app.path)?;

        // BGRA → RGBA,并顺手按 32x32 缩放(图标过大时)喵
        let rgba = bgra_to_rgba(&pixels.bgra);
        let w = pixels.width;
        let h = pixels.height;

        // 编码成 PNG 喵
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, w, h);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = match encoder.write_header() {
                Ok(w) => w,
                Err(e) => {
                    log::warn!("PNG 编码器初始化失败: {e}");
                    return None;
                }
            };
            if let Err(e) = writer.write_image_data(&rgba) {
                log::warn!("PNG 写入失败: {e}");
                return None;
            }
        }

        // 写盘喵
        if let Err(e) = std::fs::write(&snapshot, out) {
            log::warn!("图标快照保存失败: {e}");
            return None;
        }
        log::debug!("图标快照已保存: {}", snapshot.display());
        Some(snapshot)
    }
}

/// BGRA 像素转 RGBA 喵
fn bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(bgra.len());
    for chunk in bgra.chunks_exact(4) {
        rgba.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
    }
    rgba
}
