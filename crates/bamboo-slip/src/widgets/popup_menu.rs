//! 独立弹出窗里的菜单（输入法候选的右键菜单这类）：一列选项，悬停项垫底色。
//! 不在 [`crate::Ui`] 里，不登记交互：使用方自己记悬停哪一项、按返回的各项节点算点击区域。
//! 卡片与行的样子与 [`super::Menu`] 同一套，两种菜单看起来一样。

use taffy::{FlexDirection, LengthPercentageAuto, NodeId, Size, Style};

use super::WidgetTheme;
use super::parts::{padding, row, solid};
use crate::error::RenderError;
use crate::scene::{Effect, EffectKind, Scene, Visual};

/// 菜单最窄（点）：项都很短时也别缩成一条。
const MIN_WIDTH: f32 = 120.0;

pub struct PopupMenu<'a> {
    items: &'a [&'a str],

    hovered: Option<usize>,
}

impl<'a> PopupMenu<'a> {
    pub fn new(items: &'a [&'a str]) -> Self {
        Self {
            items,
            hovered: None,
        }
    }

    pub fn hovered(self, hovered: Option<usize>) -> Self {
        Self { hovered, ..self }
    }

    /// 建卡片，返回根节点（放在窗口根上，大小由内容撑开）与各项的节点（按 `items` 的顺序）。
    pub fn build(
        &self,
        scene: &mut Scene,
        theme: &WidgetTheme,
    ) -> Result<(NodeId, Vec<NodeId>), RenderError> {
        let colors = &theme.colors;
        let mut rows = Vec::with_capacity(self.items.len());
        for (index, item) in self.items.iter().enumerate() {
            let text = scene.node(
                Style::default(),
                Visual::Text {
                    text: (*item).to_owned(),
                    style: theme.text(theme.font.size, 400, colors.text),
                },
                &[],
            )?;
            let fill = (self.hovered == Some(index)).then_some(colors.hover);
            rows.push(scene.node(item_style(theme), item_visual(theme, fill), &[text])?);
        }
        let root = scene.node(
            Style {
                flex_direction: FlexDirection::Column,
                padding: padding(theme, 4.0, 4.0),
                min_size: Size {
                    width: LengthPercentageAuto::length(theme.px(MIN_WIDTH)),
                    height: LengthPercentageAuto::auto(),
                },
                ..Style::default()
            },
            panel_visual(theme),
            &rows,
        )?;
        scene.set_effects(root, vec![panel_shadow(theme)]);
        Ok((root, rows))
    }
}

/// 一项的布局：横排、上下 5 左右 8 点内边距。
pub(super) fn item_style(theme: &WidgetTheme) -> Style {
    Style {
        padding: padding(theme, 5.0, 8.0),
        flex_shrink: 0.0,
        ..row(theme, 6.0)
    }
}

/// 一项的底：悬停 / 按下时垫一层，圆角比卡片小一圈。
pub(super) fn item_visual(theme: &WidgetTheme, fill: Option<crate::color::Color>) -> Visual {
    solid(fill, None, theme.px(theme.radius) - theme.px(2.0))
}

/// 卡片：弹出层底色、细边、控件圆角。
pub(super) fn panel_visual(theme: &WidgetTheme) -> Visual {
    solid(
        Some(theme.colors.popup),
        Some((theme.px(1.0), theme.colors.line)),
        theme.px(theme.radius),
    )
}

/// 卡片下的投影。
pub(super) fn panel_shadow(theme: &WidgetTheme) -> Effect {
    Effect {
        kind: EffectKind::DropShadow,
        x: 0.0,
        y: theme.px(6.0),
        blur: theme.px(16.0),
        spread: 0.0,
        color: theme.colors.shadow,
    }
}
