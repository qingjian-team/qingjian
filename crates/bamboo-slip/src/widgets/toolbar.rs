//! 列表上方的一行操作：分组标题下、卡片之前，左边说明或分段选择，右边按钮（刷新、添加）。
//! 这一行管的是下面整张列表，所以不放进卡片里和列表项并排。

use taffy::{JustifyContent, NodeId, Style};

use super::WidgetTheme;
use super::parts::{padding, row};
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::Ui;

/// 两头对齐的一行，左右缩进与卡片里的文字对齐。
pub fn toolbar<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    items: &[NodeId],
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style {
            justify_content: JustifyContent::SPACE_BETWEEN,
            padding: padding(theme, 0.0, 4.0),
            ..row(theme, 12.0)
        },
        Visual::Group,
        items,
    )
}
