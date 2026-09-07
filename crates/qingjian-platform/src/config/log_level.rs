use serde::{Deserialize, Serialize};

/// 日志级别。缺省 info 不含用户敲的内容；debug 会把敲的拼音与上屏的文字记进日志，只在配合排查问题时开。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// 只记启动、加载、错误与配置变化。
    #[default]
    Info,

    /// 逐键、逐次上屏都记。
    Debug,
}

impl LogLevel {
    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}
