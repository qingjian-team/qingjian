//! 定位方式：缺省参与父框布局，`absolute` 不占位、按 `inset` 相对父框定位。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Position {
    Absolute,
}
