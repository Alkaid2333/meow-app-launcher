//! 日志初始化喵~
//!
//! 基于 logforth 的自定义日志系统喵:
//! * 控制台: 彩色输出,loguru 风格格式喵
//! * 文件: 落盘到 `./.datas/logs/`,分离 info 和 error 喵
//!
//! 控制台格式(loguru 风格)喵:
//! ```text
//! 2026-08-22 22:30:15.123 | INFO     | meowal::views::search_bar - 呼出搜索浮窗喵~
//! ```
//! 时间灰白、level 彩色(INFO绿/WARN黄/ERROR红/DEBUG蓝)、模块青色、消息按级别着色喵。

use logforth::append;
use logforth::filter::FilterResult;
use logforth::record::{FilterCriteria, Level, Record};
use logforth::{Diagnostic, Error, Filter, Layout};
use std::num::NonZeroUsize;
use std::path::Path;

/// 日志目录名(相对数据目录)喵
const LOG_DIR_NAME: &str = "logs";
/// info 日志文件名喵
const INFO_LOG_NAME: &str = "info.log";
/// error 日志文件名喵
const ERROR_LOG_NAME: &str = "error.log";
/// 滚动大小: 512KB 喵
const ROLL_SIZE: usize = 512 * 1024;
/// 保留份数: 3 喵
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

    // 控制台级别喵
    let console_level = if verbose {
        Level::Debug
    } else {
        Level::Info
    };

    // 三个 dispatch 喵:
    // 1. 控制台彩色喵
    // 2. info 文件(Warn 及以下)喵
    // 3. error 文件(Error 及以上)喵
    logforth::starter_log::builder()
        .dispatch(|d| {
            d.filter(MinLevelFilter::new(console_level))
                .append(append::Stdout::default().with_layout(MeowLayout::new(true)))
        })
        .dispatch(|d| {
            d.filter(RangeFilter::new(Level::Trace, Level::Warn))
                .append(file_appender(&logs_dir.join(INFO_LOG_NAME)))
        })
        .dispatch(|d| {
            d.filter(MinLevelFilter::new(Level::Error))
                .append(file_appender(&logs_dir.join(ERROR_LOG_NAME)))
        })
        .apply();

    log::info!(
        "日志初始化完成喵,日志目录: {}",
        logs_dir.display()
    );
}

/// 构建滚动文件 appender 喵(无彩色布局)喵
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
        .layout(MeowLayout::new(false))
        .build()
        .expect("初始化文件日志失败喵")
}

// ---------------------------------------------------------------------------
// 自定义布局: loguru 风格喵
// ---------------------------------------------------------------------------

/// loguru 风格彩色文本布局喵
#[derive(Debug, Clone)]
pub struct MeowLayout {
    /// 是否启用 ANSI 彩色喵
    color: bool,
}

impl MeowLayout {
    pub fn new(color: bool) -> Self {
        Self { color }
    }
}

impl Layout for MeowLayout {
    fn format(
        &self,
        record: &Record,
        _diags: &[Box<dyn Diagnostic>],
    ) -> Result<Vec<u8>, Error> {
        use colored::Colorize;

        // 时间: YYYY-MM-DD HH:mm:ss.SSS 喵
        let ts = jiff::Timestamp::try_from(record.time())
            .map_err(|e| Error::new(e.to_string()))?;
        let tz = jiff::tz::TimeZone::system();
        let zoned = ts.to_zoned(tz);
        let time = zoned
            .strftime("%Y-%m-%d %H:%M:%S%.3f")
            .to_string();

        // level 左对齐 8 宽喵
        let level_name = format!("{:<8}", record.level().name());
        let module = record.target();
        let message = format!("{}", record.payload());

        let line = if self.color {
            format!(
                "{} | {} | {} - {}",
                time.dimmed(),
                colored_level(record.level(), &level_name),
                module.cyan(),
                colored_message(record.level(), &message),
            )
        } else {
            format!("{time} | {level_name} | {module} - {message}")
        };
        Ok(line.into_bytes())
    }
}

/// 按级别返回彩色 level 文本喵
fn colored_level(level: Level, text: &str) -> String {
    use colored::Colorize;
    match level {
        Level::Trace | Level::Trace2 | Level::Trace3 | Level::Trace4 => text.magenta().to_string(),
        Level::Debug | Level::Debug2 | Level::Debug3 | Level::Debug4 => text.blue().to_string(),
        Level::Info | Level::Info2 | Level::Info3 | Level::Info4 => text.green().to_string(),
        Level::Warn | Level::Warn2 | Level::Warn3 | Level::Warn4 => text.yellow().to_string(),
        Level::Error | Level::Error2 | Level::Error3 | Level::Error4 => {
            text.red().to_string()
        }
        Level::Fatal | Level::Fatal2 | Level::Fatal3 | Level::Fatal4 => {
            text.bright_red().bold().to_string()
        }
    }
}

/// 按级别返回彩色消息文本喵
fn colored_message(level: Level, text: &str) -> String {
    use colored::Colorize;
    match level {
        Level::Trace | Level::Trace2 | Level::Trace3 | Level::Trace4 => {
            text.magenta().to_string()
        }
        Level::Debug | Level::Debug2 | Level::Debug3 | Level::Debug4 => {
            text.blue().to_string()
        }
        Level::Info | Level::Info2 | Level::Info3 | Level::Info4 => text.green().to_string(),
        Level::Warn | Level::Warn2 | Level::Warn3 | Level::Warn4 => text.yellow().to_string(),
        Level::Error | Level::Error2 | Level::Error3 | Level::Error4 => text.red().to_string(),
        Level::Fatal | Level::Fatal2 | Level::Fatal3 | Level::Fatal4 => {
            text.bright_red().bold().to_string()
        }
    }
}

// ---------------------------------------------------------------------------
// 自定义过滤器喵
// ---------------------------------------------------------------------------

/// 最小级别过滤器: 只放行 >= min 的记录喵
#[derive(Debug)]
pub struct MinLevelFilter {
    min: Level,
}

impl MinLevelFilter {
    pub fn new(min: Level) -> Self {
        Self { min }
    }
}

impl Filter for MinLevelFilter {
    fn enabled(
        &self,
        criteria: &FilterCriteria,
        _diags: &[Box<dyn Diagnostic>],
    ) -> FilterResult {
        if criteria.level() >= self.min {
            FilterResult::Accept
        } else {
            FilterResult::Reject
        }
    }
}

/// 级别区间过滤器: 只放行 [min, max] 范围内的记录喵
#[derive(Debug)]
pub struct RangeFilter {
    min: Level,
    max: Level,
}

impl RangeFilter {
    pub fn new(min: Level, max: Level) -> Self {
        Self { min, max }
    }
}

impl Filter for RangeFilter {
    fn enabled(
        &self,
        criteria: &FilterCriteria,
        _diags: &[Box<dyn Diagnostic>],
    ) -> FilterResult {
        let lv = criteria.level();
        if lv >= self.min && lv <= self.max {
            FilterResult::Accept
        } else {
            FilterResult::Reject
        }
    }
}
