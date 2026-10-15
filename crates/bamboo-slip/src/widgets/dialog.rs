//! 对话框：盖住整窗的半透明底（挡住底下的点击），中间一张卡片。是浮层（[`crate::Ui::overlay`]），不被滚动区裁掉。
//! 开关由使用方决定：要显示的那一帧调 [`dialog`]，内容自己建好传进来。Esc、取消按钮这些由使用方接。

use std::hash::Hash;

use taffy::{AlignItems, FlexDirection, JustifyContent, NodeId, Style};

use super::WidgetTheme;
use super::parts::{overlay, padding, size, solid};
use crate::error::RenderError;
use crate::scene::{Effect, EffectKind, Visual};
use crate::ui::{Interact, Ui};

/// `width` 是卡片宽度（点），`children` 竖排在卡片里。
pub fn dialog<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    local: impl Hash,
    width: f32,
    children: &[NodeId],
) -> Result<(), RenderError> {
    let key = ui.key(local);
    let colors = &theme.colors;
    let card = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            size: size(Some(theme.px(width)), None),
            padding: padding(theme, 18.0, 20.0),
            gap: super::parts::gap(theme, 12.0),
            ..Style::default()
        },
        solid(
            Some(colors.popup),
            Some((theme.px(1.0), colors.line)),
            theme.px(theme.radius * 1.5),
        ),
        children,
    )?;
    ui.scene().set_effects(
        card,
        vec![Effect {
            kind: EffectKind::DropShadow,
            x: 0.0,
            y: theme.px(10.0),
            blur: theme.px(30.0),
            spread: 0.0,
            color: colors.shadow,
        }],
    );
    ui.interact(card, key.child("card"), Interact::new());
    let scrim = ui.scene().node(
        Style {
            justify_content: Some(JustifyContent::CENTER),
            align_items: Some(AlignItems::CENTER),
            ..overlay()
        },
        Visual::solid(colors.shadow, 0.0),
        &[card],
    )?;
    // 底下的东西点不到：命中先落在这层上
    ui.interact(scrim, key, Interact::new());
    ui.overlay(scrim);
    Ok(())
}
