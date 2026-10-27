//! 材质的种类。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Material {
    /// 玻璃：macOS 26 起的 Liquid Glass，更早的系统退回毛玻璃。
    Glass,

    /// 毛玻璃：macOS 的弹出层材质。
    Blur,
}
