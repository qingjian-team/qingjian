//! 画单个节点：节点自己的画面（盒子、文字、图标）与它的投影、内阴影。按树序遍历在 `paint.rs`。

mod effect;
mod rect;
mod visual;

pub(super) use effect::{EffectMask, effect_mask};
pub(super) use rect::draw_box;
pub(super) use visual::{draw_shape, draw_visual};
