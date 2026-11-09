//! 候选窗口的绘制方式（青简渲染器 / 系统原生），对应配置项 `candidate_renderer`。

use serde::{Deserialize, Serialize};

use crate::i18n::tr;

/// 候选窗口由谁绘制：青简自己的渲染器（各平台一致，主题走它）还是系统的原生绘制。
/// 原生绘制是过渡期的退路：渲染器有问题时用户能切回去继续用，稳定一个版本后删掉。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CandidateRenderer {
    /// 青简渲染器出位图，壳只贴图。
    #[default]
    Qingjian,

    /// 平台原生绘制（macOS AppKit / Windows GDI）。
    System,
}

impl CandidateRenderer {
    /// 全部取值，设置界面按这个顺序列出。
    pub const ALL: [Self; 2] = [Self::Qingjian, Self::System];

    /// 配置文件里的写法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Qingjian => "qingjian",
            Self::System => "system",
        }
    }

    /// 界面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Qingjian => tr("renderer.qingjian"),
            Self::System => tr("renderer.system"),
        }
    }
}
