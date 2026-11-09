//! 滚动区：视口裁掉超出的内容，内容按滚动位置往上挪；指针在区域上时边上浮出一条细滚动条（不占布局宽度），能拖，点轨道跳过去。
//! 惯性由系统给（触控板的惯性阶段照样是滚动事件），引擎不自己算。

use std::hash::Hash;

use taffy::{FlexDirection, LengthPercentageAuto, NodeId, Overflow, Point, Position, Rect, Style};

use super::WidgetTheme;
use super::parts::{size, solid};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::{Interact, ScrollbarTrack, Ui};

/// 滚动条的粗细、离视口边的距离与最短长度（点）。
const BAR: f32 = 6.0;

const BAR_INSET: f32 = 3.0;

const BAR_MIN: f32 = 28.0;

pub struct ScrollArea {
    /// 视口自己的布局样式（通常是 `flex_grow: 1` 占满剩下的地方）；溢出、方向由滚动区设。
    style: Style,

    horizontal: bool,
}

impl ScrollArea {
    pub fn new(style: Style) -> Self {
        Self {
            style,
            horizontal: false,
        }
    }

    /// 改成横着滚：内容按自己的宽度排开，滚动条在底边。
    pub fn horizontal(self) -> Self {
        Self {
            horizontal: true,
            ..self
        }
    }

    /// 包住 `content`（已建好的节点，缺省竖着滚）。返回视口节点。
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
        let along = |pair: (f32, f32)| if self.horizontal { pair.0 } else { pair.1 };
        if let Some(extent) = extent.filter(|extent| along(extent.max()) > 0.0) {
            let (viewport, total) = (along(extent.viewport), along(extent.content));
            let inset = theme.px(BAR_INSET);
            let track = viewport - inset * 2.0;
            let length = (track * viewport / total).max(theme.px(BAR_MIN)).min(track);
            let start = inset
                + (track - length)
                    * (along((offset_x, offset_y)) / along(extent.max())).clamp(0.0, 1.0);
            let track_key = key.child("track");
            let track_state = ui.state(track_key);
            let dragging = ui.captured(track_key);
            let thick = if track_state.hovered || dragging {
                BAR
            } else {
                theme.scrollbar_idle.min(BAR)
            };
            let color = theme.colors.secondary;
            let shown = if state.hovered || dragging {
                color
            } else {
                color.transparent()
            };
            let thumb = ui.scene().node(
                Style {
                    position: Position::Absolute,
                    size: if self.horizontal {
                        size(Some(length), Some(theme.px(thick)))
                    } else {
                        size(Some(theme.px(thick)), Some(length))
                    },
                    inset: if self.horizontal {
                        Rect {
                            left: LengthPercentageAuto::length(start),
                            bottom: LengthPercentageAuto::length(inset),
                            right: LengthPercentageAuto::auto(),
                            top: LengthPercentageAuto::auto(),
                        }
                    } else {
                        Rect {
                            right: LengthPercentageAuto::length(inset),
                            top: LengthPercentageAuto::length(start),
                            left: LengthPercentageAuto::auto(),
                            bottom: LengthPercentageAuto::auto(),
                        }
                    },
                    ..Style::default()
                },
                solid(Some(shown), None, theme.px(thick / 2.0)),
                &[],
            )?;
            ui.fade(thumb, key.child("bar"), theme.flip, Easing::Ease);
            // 轨道比滑块宽，好点中；平时看不见，指针放上去滑块才变粗
            let reach = theme.px(BAR) + inset * 2.0;
            let edge = LengthPercentageAuto::length(0.0);
            let track = ui.scene().node(
                Style {
                    position: Position::Absolute,
                    inset: if self.horizontal {
                        Rect {
                            left: edge,
                            right: edge,
                            bottom: edge,
                            top: LengthPercentageAuto::auto(),
                        }
                    } else {
                        Rect {
                            top: edge,
                            bottom: edge,
                            right: edge,
                            left: LengthPercentageAuto::auto(),
                        }
                    },
                    size: if self.horizontal {
                        size(None, Some(reach))
                    } else {
                        size(Some(reach), None)
                    },
                    ..Style::default()
                },
                Visual::Group,
                &[thumb],
            )?;
            ui.interact(
                track,
                track_key,
                Interact::new().scrollbar(ScrollbarTrack {
                    scroller: key,
                    horizontal: self.horizontal,
                    inset,
                    thumb: length,
                }),
            );
            children.push(track);
        }
        let viewport = ui.scene().node(
            Style {
                overflow: Point {
                    x: Overflow::Hidden,
                    y: Overflow::Hidden,
                },
                flex_direction: if self.horizontal {
                    FlexDirection::Row
                } else {
                    FlexDirection::Column
                },
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
