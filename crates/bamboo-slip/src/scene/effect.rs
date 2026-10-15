//! 节点效果（像素、颜色由使用方算好）：投影垫在节点底下，内阴影压在节点填充上。

use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effect {
    pub kind: EffectKind,

    /// 偏移。
    pub x: f32,

    pub y: f32,

    pub blur: f32,

    /// 扩展：投影外扩、内阴影内缩，只对盒子生效。
    pub spread: f32,

    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    DropShadow,
    InnerShadow,
}

impl Effect {
    /// 投影伸出节点盒子的最远距离；窗口根节点按它在四周留边。
    pub fn overhang(&self) -> f32 {
        match self.kind {
            EffectKind::DropShadow => {
                crate::shadow::reach(self.blur) + self.spread + self.x.abs().max(self.y.abs())
            }
            EffectKind::InnerShadow => 0.0,
        }
    }
}
