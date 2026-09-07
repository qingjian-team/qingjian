//! 进程级基础设施：bundle 信息、日志、数据路径、配置文件的运行时状态。与 IMK、窗口都无关。

pub mod bundle;
pub mod input_source;
pub mod logging;
pub mod paths;
mod settings;

pub use bundle::BundleInfo;
pub use settings::Settings;
