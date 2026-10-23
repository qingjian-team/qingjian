//! 选词模型的档位：比几条候选、看多长的前文。两样都直接决定一次前向的序列长度，也就决定了耗时与耗电。

/// 设置里「省电 / 均衡 / 准确」三档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChoiceTier {
    Saver,
    #[default]
    Balanced,
    /// 照知意输入法的 laya：前 18 条、上文 128 字。
    Accurate,
}

impl ChoiceTier {
    /// 只比前这么多条候选。
    pub fn window(self) -> usize {
        match self {
            Self::Saver => 5,
            Self::Balanced => 9,
            Self::Accurate => super::WINDOW,
        }
    }

    /// 中文上文最多留这么多字；英文多留一半。
    pub fn context_chars(self, english: bool) -> usize {
        let chinese = match self {
            Self::Saver => 32,
            Self::Balanced => 64,
            Self::Accurate => 128,
        };
        if english { chinese * 3 / 2 } else { chinese }
    }
}
