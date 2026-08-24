//! 图标管理器喵~
//!
//! 负责: 从应用路径提取图标 → 编码为 PNG 快照缓存到 `./.datas/icons/` → 解码为 Skia Image 供渲染喵。
//! 提取失败返回 None,渲染层用「应用名前两字符 + 品牌色背景」合成兜底图标喵。
//!
//! 图标提取(涉及 SHGetFileInfo + 像素读取)是慢操作,拆分为「缓存查询」与「后台提取」:
//! * 渲染时只走 `cached_image`(内存 + 磁盘快照),绝不阻塞喵
//! * 提取交给 `IconExtractor`(可放入异步阻塞线程池),完成后主线程 `cache_image` 回填喵
//!
//! 编解码统一走 Skia 内置 codec(ICO/PNG/BMP/JPEG 均支持),不引入额外图片库喵。

use super::AppInfo;
use crate::platform::Platform;
use skia_safe::{AlphaType, ColorType, Data, EncodedImageFormat, Image, ImageInfo, images};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 图标目录名(相对数据目录)喵
pub const ICON_DIR_NAME: &str = "icons";

/// 图标管理器喵(主线程持有,维护内存缓存)喵
pub struct IconManager {
    data_dir: PathBuf,
    platform: Arc<dyn Platform>,
    /// 内存缓存: 应用名 → 解码后的 Skia Image(避免反复读盘/解码喵)
    cache: HashMap<String, Option<Image>>,
}

impl IconManager {
    pub fn new(data_dir: PathBuf, platform: Arc<dyn Platform>) -> Self {
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

    /// 获取已缓存的图标 Skia Image 喵(不触发提取,保证渲染不阻塞)喵
    ///
    /// 优先级: 内存缓存 → 自定义图标 → 磁盘快照喵。都无则返回 None(渲染层兜底字符图标)喵。
    /// Skia Image 内部是引用计数,clone 开销极小,可安全传给绘制函数喵。
    pub fn cached_image(&mut self, app: &AppInfo) -> Option<Image> {
        // 1. 内存缓存喵
        if let Some(cached) = self.cache.get(&app.name) {
            return cached.clone();
        }

        // 2. 自定义图标喵
        if let Some(icon) = &app.icon_path {
            let p = PathBuf::from(icon);
            if p.exists()
                && let Some(img) = decode_file(&p)
            {
                self.cache.insert(app.name.clone(), Some(img.clone()));
                return Some(img);
            }
        }

        // 3. 磁盘快照喵
        let snapshot = self.snapshot_path(app);
        if let Some(img) = decode_file(&snapshot) {
            self.cache.insert(app.name.clone(), Some(img.clone()));
            return Some(img);
        }

        None
    }

    /// 回填缓存喵(异步提取完成后由主线程调用)喵
    pub fn cache_image(&mut self, name: String, image: Option<Image>) {
        self.cache.insert(name, image);
    }

    /// 造一个后台提取器喵(克隆平台句柄与数据目录,可安全移入异步任务)喵
    pub fn extractor(&self) -> IconExtractor {
        IconExtractor {
            platform: self.platform.clone(),
            data_dir: self.data_dir.clone(),
        }
    }

    /// 计算快照文件路径喵
    fn snapshot_path(&self, app: &AppInfo) -> PathBuf {
        self.data_dir.join(ICON_DIR_NAME).join(app.icon_snapshot_name())
    }
}

/// 图标提取器喵(独立持有平台句柄与数据目录,可移入异步阻塞线程池)喵
#[derive(Clone)]
pub struct IconExtractor {
    platform: Arc<dyn Platform>,
    data_dir: PathBuf,
}

impl IconExtractor {
    /// 提取应用图标喵,返回 (应用名, 图标) 喵
    ///
    /// 磁盘快照已存在则直接复用(可能被并发填充)喵;否则提取并编码 PNG 快照喵。
    pub fn extract(&self, app: AppInfo) -> (String, Option<Image>) {
        let snapshot = self
            .data_dir
            .join(ICON_DIR_NAME)
            .join(app.icon_snapshot_name());
        if let Some(img) = decode_file(&snapshot) {
            return (app.name, Some(img));
        }
        let img = extract_and_snapshot(&self.platform, &app.path, &snapshot);
        (app.name, img)
    }
}

/// 提取文件图标为 Skia Image,并编码 PNG 快照落盘喵
fn extract_and_snapshot(platform: &Arc<dyn Platform>, path: &str, snapshot: &Path) -> Option<Image> {
    // 提取 BGRA 像素喵
    let pixels = platform.extract_icon_pixels(path)?;

    // BGRA 像素 → Skia Image 喵
    let info = ImageInfo::new(
        (pixels.width as i32, pixels.height as i32),
        ColorType::BGRA8888,
        AlphaType::Unpremul,
        None,
    );
    let data = Data::new_copy(&pixels.bgra);
    let row_bytes = (pixels.width * 4) as usize;
    let image = images::raster_from_data(&info, data, row_bytes)?;

    // 编码 PNG 快照落盘喵(失败只记日志,不阻断返回)喵
    // 注: encode_to_data 的 deprecation 仅针对 GPU 图像,此处是 CPU 光栅图像,
    // 无 GPU 上下文可传,故允许使用喵。
    #[allow(deprecated)]
    let encoded = image.encode_to_data(EncodedImageFormat::PNG);
    if let Some(encoded) = encoded {
        if let Err(e) = std::fs::write(snapshot, encoded.as_bytes()) {
            log::warn!("图标快照保存失败: {e}");
        } else {
            log::debug!("图标快照已保存: {}", snapshot.display());
        }
    }

    Some(image)
}

/// 用 Skia 解码图片文件(ICO/PNG/BMP/JPEG)喵
fn decode_file(path: &Path) -> Option<Image> {
    let bytes = std::fs::read(path).ok()?;
    let image = Image::from_encoded(Data::new_copy(&bytes))?;
    Some(image)
}
