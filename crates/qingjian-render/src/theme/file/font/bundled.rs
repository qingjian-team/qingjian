//! 随主题带的字体文件：`"fonts": [{ "file": "fonts/LXGWWenKai.ttf" }]`。
//! 字族名从字体文件里读，样式的 `family` 写这个名字；只能带许可允许再分发的字体。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BundledFont {
    /// 相对 `theme.json` 的路径（TTF / OTF / TTC）。
    pub(crate) file: String,
}
