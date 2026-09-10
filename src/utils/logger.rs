//! 日志初始化喵~
//!
//! 控制台走 logforth 自带彩色文本布局;文件落盘到 `data_dir/logs/` 喵。

use logforth::append;
use logforth::layout::TextLayout;
use logforth::record::{Level, LevelFilter};
use std::num::NonZeroUsize;
use std::path::Path;

const LOG_DIR_NAME: &str = "logs";
const INFO_LOG_NAME: &str = "info.log";
const ERROR_LOG_NAME: &str = "error.log";
const ROLL_SIZE: usize = 512 * 1024;
const ROLL_KEEP: usize = 3;

/// 初始化全局日志喵
///
/// * `data_dir` - 数据目录(日志文件放 data_dir/logs/)喵
/// * `verbose` - 是否输出 debug 级别日志喵
pub fn init(data_dir: &Path, verbose: bool) {
    let logs_dir = data_dir.join(LOG_DIR_NAME);
    if let Err(e) = std::fs::create_dir_all(&logs_dir) {
        eprintln!("创建日志目录失败: {e}");
    }

    let console_level = if verbose {
        Level::Debug
    } else {
        Level::Info
    };

    logforth::starter_log::builder()
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(console_level))
                .append(append::Stdout::default().with_layout(TextLayout::default()))
        })
        .dispatch(|d| {
            d.filter(LevelFilter::MoreVerboseEqual(Level::Warn))
                .append(file_appender(&logs_dir.join(INFO_LOG_NAME)))
        })
        .dispatch(|d| {
            d.filter(LevelFilter::MoreSevereEqual(Level::Error))
                .append(file_appender(&logs_dir.join(ERROR_LOG_NAME)))
        })
        .apply();

    log::info!("日志初始化完成喵,日志目录: {}", logs_dir.display());
}

fn file_appender(path: &Path) -> append::file::File {
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app.log")
        .to_string();
    append::file::FileBuilder::new(base, name)
        .rollover_size(NonZeroUsize::new(ROLL_SIZE).unwrap())
        .max_log_files(NonZeroUsize::new(ROLL_KEEP).unwrap())
        .layout(TextLayout::default().no_color())
        .build()
        .expect("初始化文件日志失败喵")
}
