use super::{Context, Interpolation, UserNgram};

/// 整句转换与词级排序用到的个人部分：个人 n-gram（没有就是 `None`）与它同静态模型插值的参数。
/// 打包成一个值往 Viterbi 里传，免得每层签名都多一个参数。
#[derive(Debug, Clone, Copy)]
pub struct Personal<'a> {
    /// 个人 n-gram；没学习数据时为 `None`，打分原样用静态模型。
    pub ngram: Option<&'a UserNgram>,

    /// 插值参数。
    pub interpolation: Interpolation,
}

impl<'a> Personal<'a> {
    /// 没有个人数据。
    pub const NONE: Personal<'static> = Personal {
        ngram: None,
        interpolation: Interpolation::DEFAULT,
    };

    /// 用缺省插值参数包一个个人 n-gram。
    pub fn new(ngram: Option<&'a UserNgram>) -> Self {
        Self {
            ngram,
            interpolation: Interpolation::DEFAULT,
        }
    }

    /// `word` 在个人数据里出现过几次；没有个人数据就是 0。
    pub fn count(&self, word: &str) -> u32 {
        self.ngram.map_or(0, |ngram| ngram.count(word))
    }

    /// 把静态模型给出的 `log P(word | previous)` 与个人概率插值；没有个人数据原样返回。
    pub fn blend(&self, context: Context<'_>, word: &str, base_log_prob: f64) -> f64 {
        self.ngram.map_or(base_log_prob, |ngram| {
            ngram.blend(context, word, base_log_prob, &self.interpolation)
        })
    }
}
