//! 节点效果：`"effects": [{ "type": "drop-shadow", "x": 0, "y": 6, "blur": 12, "spread": 0, "color": "#0000005a" }]`，
//! 另有 `inner-shadow`。阴影的形状取节点自己画出来的 alpha（圆角框、九宫格图片、文字都行），不含子节点。

use serde::Deserialize;

use crate::theme::file::ColorSpec;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(crate) enum EffectSpec {
    /// 投影：垫在节点底下。
    DropShadow(ShadowSpec),

    /// 内阴影：画在节点填充之上、子节点之下，只落在节点形状里。
    InnerShadow(ShadowSpec),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) struct ShadowSpec {
    /// 偏移（点），向右、向下为正。
    #[serde(default)]
    pub(crate) x: f32,

    #[serde(default)]
    pub(crate) y: f32,

    /// 模糊（点），与 CSS 的模糊半径大致相当。
    #[serde(default)]
    pub(crate) blur: f32,

    /// 扩展（点）：投影把形状往外扩、内阴影往里缩；只对框生效。
    #[serde(default)]
    pub(crate) spread: f32,

    pub(crate) color: ColorSpec,
}

impl EffectSpec {
    pub(crate) fn shadow(&self) -> &ShadowSpec {
        match self {
            Self::DropShadow(shadow) | Self::InnerShadow(shadow) => shadow,
        }
    }
}
