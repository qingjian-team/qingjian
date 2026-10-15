//! 图上的一格或一根柱。

/// 深浅或高度（0–1，0 画成空槽）与悬停时的说明。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Datum {
    pub level: f32,

    /// 悬停说明，空串不浮出。
    pub tip: String,
}

impl Datum {
    pub fn new(level: f32, tip: impl Into<String>) -> Self {
        Self {
            level,
            tip: tip.into(),
        }
    }
}
