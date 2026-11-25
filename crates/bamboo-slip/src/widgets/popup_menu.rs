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

    /// 卡片与行的圆角（点）；`None` 用主题的控件圆角。
    radius: Option<f32>,

    /// 卡片边框宽度（点）；`None` 用 1 点。写 0 就是没有边框。
    border_width: Option<f32>,
}

impl<'a> PopupMenu<'a> {
    pub fn new(items: &'a [&'a str]) -> Self {
        Self {
            items,
            hovered: None,
            radius: None,
            border_width: None,
        }
    }

    pub fn hovered(self, hovered: Option<usize>) -> Self {
        Self { hovered, ..self }
    }

    /// 卡片与行的圆角。用方角外观的使用方传 `Some(0.0)`；不写跟主题的控件圆角。
    pub fn radius(self, radius: f32) -> Self {
        Self {
            radius: Some(radius),
            ..self
        }
    }

    /// 卡片边框宽度。`0` 是不要边框（方角主题常这么配）。
    pub fn border_width(self, width: f32) -> Self {
        Self {
            border_width: Some(width),
            ..self
        }
    }

    /// 卡片圆角（点）：给了就用给的，`t` 是点 → 像素的倍数。
    fn corner(&self, theme: &WidgetTheme) -> f32 {
        self.radius
            .map_or(theme.px(theme.popup_radius), |radius| theme.px(radius))
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
            // 行比卡片小一圈，方角时两边都是 0，不会出现负数
            let corner = (self.corner(theme) - theme.px(2.0)).max(0.0);
            rows.push(scene.node(item_style(theme), solid(fill, None, corner), &[text])?);
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
            panel_visual(
                theme,
                self.corner(theme).max(0.0),
                self.border_width.map_or(theme.px(1.0), |w| theme.px(w)),
            ),
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
    solid(fill, None, theme.px(theme.popup_radius) - theme.px(2.0))
}

/// 卡片：弹出层底色、细边；`radius` 与 `border` 是调用方定的圆角与边框宽度（像素）。
pub(super) fn panel_visual(theme: &WidgetTheme, radius: f32, border: f32) -> Visual {
    // 宽度为 0 时不画边框（画上去会盖住一个像素的底）
    let edge = (border > 0.0).then_some((border, theme.colors.line));
    solid(Some(theme.colors.popup), edge, radius)
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
