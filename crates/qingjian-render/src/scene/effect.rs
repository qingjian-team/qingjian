//! 节点效果（像素、颜色已按外观取好）：投影垫在节点底下，内阴影压在节点填充上。

use crate::color::Color;
use crate::theme::file::node::EffectSpec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Effect {
    pub(crate) kind: EffectKind,

    /// 偏移。
    pub(crate) x: f32,

    pub(crate) y: f32,

    pub(crate) blur: f32,

    /// 扩展：投影外扩、内阴影内缩，只对盒子生效。
    pub(crate) spread: f32,

    pub(crate) color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectKind {
    DropShadow,
    InnerShadow,
}

impl Effect {
    /// 主题里的效果按倍数换成像素，颜色由调用方按数据与外观取好。
    pub(crate) fn from_spec(spec: &EffectSpec, scale: f32, color: Color) -> Self {
        let shadow = spec.shadow();
        Self {
            kind: match spec {
                EffectSpec::DropShadow(_) => EffectKind::DropShadow,
                EffectSpec::InnerShadow(_) => EffectKind::InnerShadow,
            },
            x: shadow.x * scale,
            y: shadow.y * scale,
            blur: shadow.blur * scale,
            spread: shadow.spread * scale,
            color,
        }
    }

    /// 投影伸出节点盒子的最远距离；窗口根节点按它在四周留边。
    pub(crate) fn overhang(&self) -> f32 {
        match self.kind {
            EffectKind::DropShadow => {
                crate::shadow::reach(self.blur) + self.spread + self.x.abs().max(self.y.abs())
            }
            EffectKind::InnerShadow => 0.0,
        }
    }
}
