//! 主题文件 `theme.json` 的结构，见 `docs/design/theme.md`。只管反序列化，解析引用、实例化在别处。

mod adaptive;
mod color;
mod font;
mod meta;
pub(crate) mod node;
mod status;
mod text;
mod windows;

use std::collections::HashMap;

use serde::Deserialize;

pub(crate) use adaptive::Adaptive;
pub(crate) use color::{ColorRef, ColorSpec, ColorValue};
pub(crate) use font::{BundledFont, FamilyList, FontRef, FontStyle};
pub(crate) use meta::{LockedAppearance, Meta};
pub(crate) use status::StatusSpec;
pub(crate) use text::TextSettings;
pub(crate) use windows::Windows;

use node::NodeSpec;

/// 当前认得的最高格式版本。
pub(crate) const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ThemeFile {
    /// 格式版本，只在不兼容改动时加一。
    pub(crate) schema: u32,

    pub(crate) meta: Meta,

    /// 命名颜色，节点里用 `@名字` 引用。
    #[serde(default)]
    pub(crate) variables: HashMap<String, Adaptive<ColorValue>>,

    pub(crate) text: TextSettings,

    /// 随主题带的字体文件。
    #[serde(default)]
    pub(crate) fonts: Vec<BundledFont>,

    /// 可复用的节点（组件），`use` / `repeat` 按名字引用。
    #[serde(default)]
    pub(crate) components: HashMap<String, NodeSpec>,

    pub(crate) windows: Windows,

    pub(crate) status: StatusSpec,
}
