//! 设置里列出的一个双拼方案。

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ShuangpinChoice {
    /// 存进设置的方案名（Core 的 `ShuangpinScheme::key`，与 PC 配置 `[general] shuangpin` 同一种写法）。
    pub key: &'static str,

    pub label: &'static str,
}
