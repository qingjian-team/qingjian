//! 正体 / 斜体。字体没有斜体面时由渲染器把正体倾斜 14° 代替。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FontStyle {
    #[default]
    Normal,

    Italic,
}
