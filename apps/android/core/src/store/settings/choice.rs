//! 设置里下拉或多选的一项。

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Choice {
    /// 存进设置的写法，与 PC 配置文件同一种。
    pub key: &'static str,

    pub label: String,
}
