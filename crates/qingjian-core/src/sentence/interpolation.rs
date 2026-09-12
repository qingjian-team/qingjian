use super::{CONFIDENCE_K, MAX_CONFIDENCE, TRIGRAM_DISCOUNT, USER_LAMBDA};

/// 个人 n-gram 与静态模型插值的参数（见 [`UserNgram::blend`]）。缺省值是 `sentence` 模块里的常数，
/// 回放调参（`qingjian-cli --tune`）时可以整组换掉，引擎与壳只用缺省值。
///
/// [`UserNgram::blend`]: super::UserNgram::blend
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interpolation {
    /// 个人概率里 bigram 部分的权重，其余给个人一元（[`USER_LAMBDA`]）。
    pub lambda: f64,

    /// 插值权重 μ = c(v)/(c(v)+K) 里的 K（[`CONFIDENCE_K`]）。
    pub confidence_k: f64,

    /// 插值权重的封顶（[`MAX_CONFIDENCE`]）。
    pub max_confidence: f64,

    /// 个人三元的绝对折扣 D（[`TRIGRAM_DISCOUNT`]）。
    pub trigram_discount: f64,
}

impl Interpolation {
    /// 现在的常数。
    pub const DEFAULT: Self = Self {
        lambda: USER_LAMBDA,
        confidence_k: CONFIDENCE_K,
        max_confidence: MAX_CONFIDENCE,
        trigram_discount: TRIGRAM_DISCOUNT,
    };
}

impl Default for Interpolation {
    fn default() -> Self {
        Self::DEFAULT
    }
}
