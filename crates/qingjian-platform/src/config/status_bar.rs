use serde::{Deserialize, Serialize};

/// `[status_bar]` 分节：桌面上常驻、可拖动的悬浮状态条（显示当前中 / 英，可选双拼方案）。
/// 与任务栏的中 / 英指示器（语言栏按钮）并存，各是一条。位置记在这里，重启后回到原处。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusBarConfig {
    /// 是否显示悬浮状态条。缺省关。
    pub enabled: bool,

    /// 记住的屏幕横坐标（内容左上角物理像素）；没拖动过为 `None`，首次按屏幕右下角摆放。
    pub x: Option<i32>,

    /// 记住的屏幕纵坐标（内容左上角物理像素）。
    pub y: Option<i32>,

    /// 大小（百分比，60–150），见 [`Self::size`]。与候选字号无关。
    pub size: u8,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            x: None,
            y: None,
            size: 100,
        }
    }
}

impl StatusBarConfig {
    /// 缩放倍数，写出范围的收到 60%–150%。
    pub fn size(&self) -> f32 {
        f32::from(self.size.clamp(60, 150)) / 100.0
    }
}
