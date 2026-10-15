//! 滚动区：视口裁掉超出的内容，内容按滚动位置往上挪；指针在区域上时右边浮出一条细滚动条（不占布局宽度，同 macOS 的叠加滚动条）。
//! 惯性由系统给（触控板的惯性阶段照样是滚动事件），引擎不自己算。

use std::hash::Hash;

use taffy::{FlexDirection, LengthPercentageAuto, NodeId, Overflow, Point, Position, Rect, Style};

use super::WidgetTheme;
use super::parts::{size, solid};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, Ui};

/// 滚动条的粗细、离视口边的距离与最短长度（点）。
const BAR: f32 = 6.0;

const BAR_INSET: f32 = 3.0;

const BAR_MIN: f32 = 28.0;

pub struct ScrollArea {
    /// 视口自己的布局样式（通常是 `flex_grow: 1` 占满剩下的地方）；溢出、方向由滚动区设。
    style: Style,
}

impl ScrollArea {
    pub fn new(style: Style) -> Self {
        Self { style }
    }

    /// 包住 `content`（已建好的节点，竖着滚）。返回视口节点。
    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        content: NodeId,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let ((offset_x, offset_y), extent) = ui.scroll(key);
        let moved = ui.scene().node(
            Style {
                position: Position::Relative,
                inset: Rect {
                    left: LengthPercentageAuto::length(-offset_x),
                    top: LengthPercentageAuto::length(-offset_y),
                    right: LengthPercentageAuto::auto(),
                    bottom: LengthPercentageAuto::auto(),
                },
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                ..Style::default()
            },
            Visual::Group,
            &[content],
        )?;
        let mut children = vec![moved];
        if let Some(extent) = extent.filter(|extent| extent.max().1 > 0.0) {
            let (viewport, total) = (extent.viewport.1, extent.content.1);
            let inset = theme.px(BAR_INSET);
            let track = viewport - inset * 2.0;
            let length = (track * viewport / total).max(theme.px(BAR_MIN)).min(track);
            let top = inset + (track - length) * (offset_y / extent.max().1).clamp(0.0, 1.0);
            let color = theme.colors.secondary;
            let shown = if state.hovered {
                color
            } else {
                color.transparent()
            };
            let bar = ui.scene().node(
                Style {
                    position: Position::Absolute,
                    size: size(Some(theme.px(BAR)), Some(length)),
                    inset: Rect {
                        right: LengthPercentageAuto::length(inset),
                        top: LengthPercentageAuto::length(top),
                        left: LengthPercentageAuto::auto(),
                        bottom: LengthPercentageAuto::auto(),
                    },
                    ..Style::default()
                },
                solid(Some(shown), None, theme.px(BAR / 2.0)),
                &[],
            )?;
            ui.fade(bar, key.child("bar"), theme.flip, Easing::Ease);
            children.push(bar);
        }
        let viewport = ui.scene().node(
            Style {
                overflow: Point {
                    x: Overflow::Hidden,
                    y: Overflow::Hidden,
                },
                flex_direction: FlexDirection::Column,
                min_size: taffy::Size {
                    width: LengthPercentageAuto::length(0.0),
                    height: LengthPercentageAuto::length(0.0),
                },
                ..self.style
            },
            Visual::Group,
            &children,
        )?;
        ui.scene().set_clip(viewport, 0.0);
        ui.interact(viewport, key, Interact::new().scrolls(moved));
        Ok(viewport)
    }
}
