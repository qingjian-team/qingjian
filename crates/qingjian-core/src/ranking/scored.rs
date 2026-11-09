use std::cmp::Reverse;

use qingjian_dictionary::Match;

/// 一条待排序的词库命中。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scored<'a> {
    /// 词库命中，含词、拼音、词频与是否精确。
    pub hit: Match<'a>,

    /// 输入的最后一个音节是完整的，且与词的对应音节相同（而非前缀扩展）。
    pub full_last: bool,

    /// 词覆盖了输入开头多少个字母（不含 `'`）。按字母而不是音节算，不同切分之间才可比：
    /// `xian` 的 先（1 音节）和 西安（2 音节）覆盖同样 4 个字母。前缀词（`kaifazhe` → 开发）覆盖得少。
    pub coverage: usize,

    /// 切分里非末尾的简拼音节数（`kai f a` 是 1，`kai fa` 是 0）：越少越像用户敲的原话，
    /// 否则 `kaifa` 会因 `kai f a` 这种切法让 开放啊 排到 开放 前面。
    pub abbreviated: usize,

    /// 用户选择过的次数，来自 Learner。
    pub weight: u32,

    /// 靠模糊音 / 敲错变体命中的代价（某个音节与敲的不同；0 是敲的原样）：从上下文得分里扣掉，
    /// 预选时词频按 e^(-代价) 打折，同分排在原样命中后面。
    pub penalty: f64,

    /// 注音敲了声调时的声调代价（[`crate::zhuyin::tone::cost`]，越小越对得上）；全拼 / 双拼 / 形码恒为 0。
    pub tone_cost: u8,

    /// 靠把完整音节拆开才读得出（`shishan` 按 `shi s h an` 读出的 事实、按 `shi sha n` 读出的 是啥呢）：同一段输入有一种全是完整音节的切法时才算。
    /// 有词的读音正好是敲的音节时，这类读法排到首音节的头一页单字之后（[`super::arrange`]），免得几百条把单字挤到翻不到的地方。
    pub split: bool,
}

/// 预选键：结构项之后按用户选择次数与词库词频，命中太多时先用它砍到够排的量（不必算上下文得分）。
/// 打包成一个整数、越大越靠前：单字母简拼一键命中几万条，逐条比元组太慢。
/// 位从高到低：不是拆开的读法(1) 精确(1) 覆盖字母数(8) 简拼数的补(8) 末音节完整(1) 声调代价的补(8) 选择次数(32) 打折后词频(32) 原样命中(1) 字数的补(8)；
/// 与 [`SortKey`] 的前几项同序，只是不拿文本做最后的平手项（预选边界上的平手谁留下无所谓）。
pub type PreselectKey = u128;

/// 排序键，越小越靠前。元组的顺序即排序规则，见模块文档；第五项是注音的声调代价，第六项是同输入串下的选择次数，
/// 第七项是上下文得分（毫分，整数才能比较）。文本借自词库，键可以脱离 `Scored` 存放。
pub type SortKey<'a> = (
    Reverse<bool>,
    Reverse<usize>,
    usize,
    Reverse<bool>,
    u8,
    Reverse<u32>,
    Reverse<i64>,
    bool,
    usize,
    bool,
    &'a str,
);

impl<'a> Scored<'a> {
    /// 排在第一档的精确命中：补充词库的精确命中不算，与主词库的敲错纠正、前缀词一起按打分比，
    /// 否则一百多万条里总有个片段读音正好对上，`zanen` 的 咱们 就被 咋嫩 压在后面。
    fn first_tier(&self) -> bool {
        self.hit.exact && !self.hit.secondary
    }

    /// 命中不是敲的原样（模糊音或敲错变体）。
    pub fn altered(&self) -> bool {
        self.penalty > 0.0
    }

    pub(super) fn preselect_key(&self) -> PreselectKey {
        let frequency = if self.altered() {
            (f64::from(self.hit.frequency) * (-self.penalty).exp()) as u32
        } else {
            self.hit.frequency
        };
        let chars = u128::from(u8::try_from(self.hit.text.chars().count()).unwrap_or(u8::MAX));
        let coverage = u128::from(u8::try_from(self.coverage).unwrap_or(u8::MAX));
        let abbreviated = u128::from(u8::try_from(self.abbreviated).unwrap_or(u8::MAX));
        (u128::from(!self.split) << 99)
            | (u128::from(self.first_tier()) << 98)
            | (coverage << 90)
            | ((0xFF - abbreviated) << 82)
            | (u128::from(self.full_last) << 81)
            | (u128::from(0xFF - self.tone_cost) << 73)
            | (u128::from(self.weight) << 41)
            | (u128::from(frequency) << 9)
            | (u128::from(!self.altered()) << 8)
            | (0xFF - chars)
    }

    /// `choice` 是同输入串下的选择次数，`score` 是上下文得分（log 概率，已含用户加分与模糊音 / 敲错扣分）。
    pub(super) fn key(&self, choice: u32, score: f64) -> SortKey<'a> {
        (
            Reverse(self.first_tier()),
            Reverse(self.coverage),
            self.abbreviated,
            Reverse(self.full_last),
            self.tone_cost,
            Reverse(choice),
            Reverse((score * 1000.0).round() as i64),
            self.altered(),
            self.hit.text.chars().count(),
            // 同一个词两种读法都命中时，去重留下不拆开的那条
            self.split,
            self.hit.text,
        )
    }
}
