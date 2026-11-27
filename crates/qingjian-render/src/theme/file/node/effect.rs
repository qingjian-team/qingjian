//! 节点效果：`"effects": [{ "type": "drop-shadow", "x": 0, "y": 6, "blur": 12, "spread": 0, "color": "#0000005a" }]`，
//! 另有 `inner-shadow`。阴影的形状取节点自己画出来的 alpha（圆角框、九宫格图片、文字都行），不含子节点。

use bamboo_slip::{Color, Effect, EffectKind};
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

    /// 按倍数换成像素；颜色由调用方按数据与外观取好。
    pub(crate) fn to_effect(&self, scale: f32, color: Color) -> Effect {
        let shadow = self.shadow();
        Effect {
            kind: match self {
                Self::DropShadow(_) => EffectKind::DropShadow,
                Self::InnerShadow(_) => EffectKind::InnerShadow,
            },
            x: shadow.x * scale,
            y: shadow.y * scale,
            blur: shadow.blur * scale,
            spread: shadow.spread * scale,
            color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EffectSpec;
    use bamboo_slip::{Color, EffectKind};

    #[test]
    fn both_kinds_parse_with_zero_defaults_and_a_required_color() {
        let drop: EffectSpec = serde_json::from_str(
            r##"{ "type": "drop-shadow", "y": 6, "blur": 12, "color": "#0000005a" }"##,
        )
        .unwrap();
        let shadow = drop.shadow();
        assert_eq!(
            (shadow.x, shadow.y, shadow.blur, shadow.spread),
            (0.0, 6.0, 12.0, 0.0)
        );
        let inner: EffectSpec =
            serde_json::from_str(r##"{ "type": "inner-shadow", "color": "#000000" }"##).unwrap();
        assert!(matches!(inner, EffectSpec::InnerShadow(_)));
        assert!(
            serde_json::from_str::<EffectSpec>(r##"{ "type": "glow", "color": "#fff" }"##).is_err()
        );
        assert!(serde_json::from_str::<EffectSpec>(r##"{ "type": "drop-shadow" }"##).is_err());
    }

    #[test]
    fn to_effect_scales_all_lengths_and_keeps_the_kind() {
        let spec: EffectSpec = serde_json::from_str(
            r##"{ "type": "drop-shadow", "x": 2, "y": -4, "blur": 8, "spread": 1.5, "color": "#00000000" }"##,
        )
        .unwrap();
        let effect = spec.to_effect(2.0, Color::rgb(0, 0, 0));
        assert!(matches!(effect.kind, EffectKind::DropShadow));
        assert_eq!(
            (effect.x, effect.y, effect.blur, effect.spread),
            (4.0, -8.0, 16.0, 3.0)
        );
        assert_eq!((effect.color.r, effect.color.g, effect.color.b), (0, 0, 0));
        let inner: EffectSpec =
            serde_json::from_str(r##"{ "type": "inner-shadow", "color": "#00000000" }"##).unwrap();
        assert!(matches!(
            inner.to_effect(1.0, Color::rgb(1, 1, 1)).kind,
            EffectKind::InnerShadow
        ));
    }
}
