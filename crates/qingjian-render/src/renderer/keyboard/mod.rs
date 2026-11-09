//! 键位提示键盘：几排圆角键错位排开，按状态换填充与文字颜色。双拼键左上键名、右上角标、下方几行小字（`syllables.rs`），
//! 五笔键是字根网格、底下一行键名与区位号（`roots.rs`）。
//! 排法固定，尺寸与颜色取主题的 `keyboard` 分节；整块的透明度由壳设在窗口上，这里不管。

use bamboo_slip::taffy::NodeId;
use bamboo_slip::taffy::{Dimension, LengthPercentageAuto, Position, Rect, Size, Style};
use bamboo_slip::{Color, Effect, FontSpec, RenderError, Scene, TextStyle, Visual};

mod roots;
mod syllables;

use super::{Rendered, Renderer};
use crate::keyboard::{Keyboard, KeycapState};
use crate::theme::file::ColorSpec;
use crate::theme::file::node::EffectSpec;
use crate::theme::{TextSizes, Theme};

/// 第二、三排相对第一排右移几个键宽，与常见键盘的错位一致。
const ROW_OFFSETS: [f32; 3] = [0.0, 0.3, 0.8];

/// 用户设的大小为 100% 时键盘再放大的倍数：主题里的尺寸按这个倍数画出来才看得清（真机试下来 1.3 合适）。
const BASE_ZOOM: f32 = 1.3;

/// 键内文字离键边的距离（点）。
pub(super) const INSET: f32 = 4.0;

/// 一次渲染里按倍数换算好的参数。
pub(super) struct Metrics<'a> {
    pub(super) theme: &'a Theme,

    pub(super) scale: f32,

    pub(super) key_font: FontSpec,

    pub(super) label_font: FontSpec,

    pub(super) root_font: FontSpec,
}

impl Metrics<'_> {
    pub(super) fn px(&self, points: f32) -> f32 {
        points * self.scale
    }

    pub(super) fn style(&self, font: FontSpec, color: Color) -> TextStyle {
        TextStyle::new(
            font.scaled(self.scale),
            font.size,
            color,
            self.theme.text_gamma(),
        )
    }
}

/// 一个键各部分的颜色。
pub(super) struct Paint {
    pub(super) fill: Color,

    pub(super) key: Color,

    pub(super) corner: Color,

    pub(super) lines: Color,
}

impl Renderer {
    /// 画键位提示键盘。整块跟着候选字的比例缩放，再乘 `BASE_ZOOM` 与 `zoom`（用户设的大小，1 为原大）。
    /// 位图仍按 `scale` 标记，壳换回点时放大的部分留在点数里。
    pub fn render_keyboard(
        &mut self,
        keyboard: &Keyboard,
        theme: &Theme,
        scale: f32,
        zoom: f32,
    ) -> Result<Rendered, RenderError> {
        let screen_scale = scale;
        let scale = scale * theme.candidate_scale() * BASE_ZOOM * zoom;
        let theme = &theme.with_text_sizes(TextSizes::default());
        self.stage.text().use_families(theme.families());
        let spec = &theme.file().keyboard;
        let m = Metrics {
            theme,
            scale,
            key_font: theme.font_ref(&spec.key_font),
            label_font: theme.font_ref(&spec.label_font),
            root_font: theme.font_ref(&spec.root_font),
        };
        // 五笔键要放下一格格字根，比双拼键大
        let roots = keyboard
            .rows
            .iter()
            .flatten()
            .any(|cap| !cap.roots.is_empty());
        let (key_width, key_height) = if roots {
            (spec.root_key_width, spec.root_key_height)
        } else {
            (spec.key_width, spec.key_height)
        };
        let (key_w, key_h, gap, padding) = (
            m.px(key_width),
            m.px(key_height),
            m.px(spec.gap),
            m.px(spec.padding),
        );

        let mut scene = Scene::new();
        let mut keys = Vec::new();
        let mut width: f32 = 0.0;
        for (row_index, row) in keyboard.rows.iter().enumerate() {
            let offset = ROW_OFFSETS.get(row_index).copied().unwrap_or(0.0) * key_w;
            let top = padding + row_index as f32 * (key_h + gap);
            for (i, cap) in row.iter().enumerate() {
                let left = padding + offset + i as f32 * (key_w + gap);
                let place = (left, top);
                let size = (key_w, key_h);
                keys.push(if cap.roots.is_empty() {
                    syllables::key(&mut scene, cap, &m, place, size)?
                } else {
                    roots::key(&mut scene, cap, &m, place, size)?
                });
                width = width.max(left + key_w);
            }
        }
        let content_width = (width + padding).ceil();
        let rows = keyboard.rows.len() as f32;
        let content_height = (padding * 2.0 + rows * key_h + (rows - 1.0).max(0.0) * gap).ceil();
        let root = scene.node(
            Style {
                size: Size {
                    width: Dimension::length(content_width),
                    height: Dimension::length(content_height),
                },
                ..Style::default()
            },
            Visual::solid(theme.color(&spec.background), m.px(spec.radius)),
            &keys,
        )?;
        let effects = spec
            .effects
            .iter()
            .map(|effect| effect_of(effect, &m))
            .collect();
        scene.set_effects(root, effects);
        scene.layout(root, self.stage.text())?;
        Ok(Rendered::new(
            self.stage
                .rasterize(&scene, root, (content_width, content_height), screen_scale)?,
            Vec::new(),
        ))
    }
}

/// 键盘没有数据条件，条件颜色取 `else` 分支。
fn effect_of(spec: &EffectSpec, m: &Metrics) -> Effect {
    let color = match &spec.shadow().color {
        ColorSpec::Fixed(color)
        | ColorSpec::Switch {
            otherwise: color, ..
        } => color,
    };
    spec.to_effect(m.scale, m.theme.color(color))
}

pub(super) fn paint(state: &KeycapState, m: &Metrics) -> Paint {
    let spec = &m.theme.file().keyboard;
    let color = |c| m.theme.color(c);
    match state {
        KeycapState::Normal => Paint {
            fill: color(&spec.key_fill),
            key: color(&spec.key_text),
            corner: color(&spec.initial),
            lines: color(&spec.label),
        },
        KeycapState::Pressed => {
            let text = color(&spec.pressed_text);
            Paint {
                fill: color(&spec.pressed_fill),
                key: text,
                corner: text,
                lines: text,
            }
        }
        KeycapState::Next(_) => Paint {
            fill: color(&spec.next_fill),
            key: color(&spec.key_text),
            corner: color(&spec.initial),
            lines: color(&spec.next_text),
        },
        KeycapState::Dimmed => {
            let text = color(&spec.dimmed_text);
            Paint {
                fill: color(&spec.key_fill),
                key: text,
                corner: text,
                lines: text,
            }
        }
    }
}

pub(super) fn text(
    scene: &mut Scene,
    text: &str,
    style: TextStyle,
    layout: Style,
) -> Result<NodeId, RenderError> {
    scene.node(
        layout,
        Visual::Text {
            text: text.to_owned(),
            style,
        },
        &[],
    )
}

/// 绝对定位：左右只给一边，另一边自动；`size` 不给就按内容量。
pub(super) fn absolute(
    (left, right): (Option<f32>, Option<f32>),
    top: Option<f32>,
    size: Option<(f32, f32)>,
) -> Style {
    let edge = |value: Option<f32>| {
        value.map_or(LengthPercentageAuto::auto(), LengthPercentageAuto::length)
    };
    Style {
        position: Position::Absolute,
        inset: Rect {
            left: edge(left),
            right: edge(right),
            top: edge(top),
            bottom: LengthPercentageAuto::auto(),
        },
        size: size.map_or(Size::auto(), |(width, height)| Size {
            width: Dimension::length(width),
            height: Dimension::length(height),
        }),
        ..Style::default()
    }
}
