//! 整句转换一次搜索里不变的输入：词库、语言模型、个人数据、打分函数、声调与搜索规模。

use qingjian_dictionary::Dictionary;

use super::{LanguageModel, Personal, SearchLimits};

/// 整句转换一次搜索的输入，束搜索的每一层都原样往下传。
/// `weight(词)` 是用户选择次数，`cost(位置, 命中的音节)` 是那个位置命中这种写法要扣的分（敲的原样 0）。
pub struct SearchInput<'a, W, C> {
    pub dictionaries: &'a [&'a Dictionary],

    pub model: &'a dyn LanguageModel,

    /// 个人 n-gram 与插值参数；没有个人数据就是 [`Personal::NONE`]。
    pub personal: Personal<'a>,

    pub weight: W,

    pub cost: C,

    /// 注音每个位置敲的声调（与音节位置对齐，可以更短；全拼 / 双拼为空）。
    pub tones: &'a [Option<u8>],

    pub limits: SearchLimits,
}

impl<'a, W, C> SearchInput<'a, W, C>
where
    W: Fn(&str) -> u32,
    C: Fn(usize, &str) -> f64,
{
    /// 没有声调、搜索规模用缺省的输入。
    pub fn new(
        dictionaries: &'a [&'a Dictionary],
        model: &'a dyn LanguageModel,
        personal: Personal<'a>,
        weight: W,
        cost: C,
    ) -> Self {
        Self {
            dictionaries,
            model,
            personal,
            weight,
            cost,
            tones: &[],
            limits: SearchLimits::DEFAULT,
        }
    }

    pub fn with_tones(self, tones: &'a [Option<u8>]) -> Self {
        Self { tones, ..self }
    }

    pub fn with_limits(self, limits: SearchLimits) -> Self {
        Self { limits, ..self }
    }
}
