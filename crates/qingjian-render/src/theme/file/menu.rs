//! 候选右键菜单的样式（Windows 与 macOS 共用）。Android 的长按气泡是壳自己画的（ChoicePopup），不走这条渲染路径，也不读这里。
//!
//! 菜单画在 bamboo-slip 的弹出菜单控件上，配色默认用控件自己那套（浅色 / 深色跟着系统），
//! 主题写了这一节就用主题的颜色与圆角——「墨岩」这类方角主题把 `radius` 写成 0 就跟着方了。
//! 每一项都是可选的：不写就退回默认值，老主题不受影响。

use serde::Deserialize;

use super::ColorRef;

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct MenuSpec {
    /// 菜单的底：要不透明，盖住底下的内容。
    #[serde(default)]
    pub(crate) background: Option<ColorRef>,

    /// 鼠标停在某一项上时那一行的底。
    #[serde(default)]
    pub(crate) hover: Option<ColorRef>,

    /// 正文（各项的文字）。
    #[serde(default)]
    pub(crate) text: Option<ColorRef>,

    /// 边框与分隔线。
    #[serde(default)]
    pub(crate) line: Option<ColorRef>,

    /// 边框宽度；0 就是没有边框。
    #[serde(default)]
    pub(crate) border_width: Option<f32>,

    /// 圆角。方角主题写 0；不写用控件默认值。
    #[serde(default)]
    pub(crate) radius: Option<f32>,
}
