//! 窗口层喵~ 启动器窗口、配置窗口、应用管理中心、系统托盘都住这里喵。

pub mod app_manager;
pub mod launcher;
pub mod settings;
pub mod settings_data;
pub mod tray;

pub use app_manager::AppManagerWindow;
pub use launcher::Launcher;
pub use settings::SettingsWindow;
pub use tray::Tray;
