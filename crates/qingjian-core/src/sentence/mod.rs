//! 离线整句转换：把一串音节转成最可能的词序列（`woxiangqu` → 我想去）。
//!
//! 词图上每个格子放正好覆盖那几个音节的词（每格只留词频最高的几个），Viterbi + 束搜索找最优路径。
//! 打分来自 [`LanguageModel`]（bigram，`qingjian-lm` 实现），模型不认识的词用词库词频（一元）兜底并扣分；
//! 用户选过的词（Learner 的 weight）加分。没接模型时整体退化为一元词频。
//!
//! 个人 n-gram（[`UserNgram`]，二元 + 三元，随上屏在线更新、由 Learner 持有）与静态模型插值，让整句越用越像自己：
//! 静态模型只看前一个词，个人部分看前两个词（[`Context`]），Viterbi 里前二词取最优前驱的回指，不扩状态。
//!
//! 简拼位置（`wxq` 的 `w x q`）按前缀取词，每个格子多留一些候选，全靠语言模型在路径上分辨；
//! 全拼句子末尾落单的声母也按前缀读（`youdiandongx` → 有点东西），落单的单元音（i / u）当作没打完的韵母丢弃，
//! 简拼句子里末尾单字母就是一个音节。
//!
//! 格子查词是最贵的一步，结果放进 [`SpanCache`]：敲键是增量的，每一键只有以它结尾的几个格子是新的。

mod context;
mod conversion;
mod interpolation;
mod language_model;
mod personal;
mod scorer;
mod sentence_word;
mod span;
mod text_segment;
mod user_ngram;
mod viterbi;

pub use context::Context;
pub use conversion::Conversion;
pub use interpolation::Interpolation;
pub use language_model::{LanguageModel, NoLanguageModel};
pub use personal::Personal;
pub use scorer::SentenceScorer;
pub use sentence_word::SentenceWord;
pub use span::{MAX_SPAN_CACHE_ENTRIES, SpanCache, SpanWord};
pub(crate) use text_segment::is_han;
pub use text_segment::{MAX_TEXT_WORD_CHARS, segment_text};
pub use user_ngram::UserNgram;
pub use viterbi::{convert, convert_paths, convert_whole, convert_with};
/// 句首标记：个人 n-gram 里句首词的前词。与 `qingjian-lm` 语料统计用的是同一个记号。
pub const SENTENCE_START: &str = "<s>";

/// 个人概率里 bigram 部分的权重，其余给个人一元。
pub const USER_LAMBDA: f64 = 0.8;

/// 个人插值权重 μ = c(v)/(c(v)+K)：前词见过 K 次时个人数据与静态模型各占一半（再受封顶限制）。
/// 取 8 是为了一次误选翻不过静态 bigram，选两次才翻。
pub const CONFIDENCE_K: f64 = 8.0;

/// 个人插值权重的封顶：用户数据再多，没见过的接续也最多打这个折，保留说新话的余地。
pub const MAX_CONFIDENCE: f64 = 0.5;

/// 个人三元的绝对折扣 D：每条见过的三元接续让出 D 份概率给二元回退，见得少的上文回退得多。
/// 0.75 是 n-gram 平滑的惯用值，个人数据量小、一次见过的接续占多数时它决定了三元能压过二元多少。
pub const TRIGRAM_DISCOUNT: f64 = 0.75;

/// 个人 n-gram 最多存多少条转移（二元对 + 三元条），超过就整体减半（忘掉久远的偏好）。一年的个人输入远到不了这个量。
pub const MAX_USER_TRANSITIONS: usize = 200_000;

/// 一个词最多几个音节；更长的词库里有但极少，限制它让词图规模可控。
pub const MAX_WORD_SYLLABLES: usize = 8;

/// 全拼句子末尾未打完的音节至少要几个字母才参与整句；再短的前缀只有落单声母例外（见 `viterbi`），
/// 落单的 i / u 等单元音范围太大又像没打完的韵母，不参与。
pub const MIN_PARTIAL_LETTERS: usize = 2;

/// 整句转换的搜索规模限制。三个数一起决定「算得多细」与「花多少时间」，
/// 是可以整套换掉的（[`Interpolation`] 那种回放扫参的思路，见 `qingjian-cli --tune`）。
///
/// 这三个数曾经是编译期常量，只能在改代码重建之后才能试别的值；
/// 而它们对准确率和延迟的影响一点也不比插值参数小，实测（域外 2257 句 + 域内 5 句，
/// 均加载 486 万 bigram 的语言模型）：
///
/// | span / abbr / beam | 首选 | 字准确率 | 查询平均 |
/// |---|---|---|---|
/// | 6 / 20 / 8（曾经的缺省） | 7.0% | 67.7% | 1.6 ms |
/// | 12 / 40 / 16（放大一倍） | 6.5% | 66.6% | — |
/// | 4 / 12 / 4 | 7.4% | 68.3% | 1.5 ms |
/// | 3 / 8 / 2（再收紧） | 7.1% | 68.4% | 1.4 ms |
///
/// 两条读数：**放大搜索反而更差**——束更宽会把语言模型高估的错误路径翻出来，
/// 它不是「搜得越全越准」的那种目标函数；**收紧有收益但有下限**——收到 3/8/2 时
/// 域内句子从 100% 掉到 80%，格子候选太少会把要的词挤掉。
/// 所以缺省仍取当年调过的 6/20/8，要动的人在真实回放日志上扫过再定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchLimits {
    /// 每个格子最多留几个词（按词库词频 + 用户加分）。同音词很多，全留会让束搜索白费。
    pub span_candidates: usize,

    /// 有简拼位置的格子最多留几个词：`h` 下有 和 / 好 / 会 / 还 / 很 …… 几十个常用字，
    /// 只留六个会把句子里要的那个挤掉，多留一些让语言模型去挑。
    pub abbreviated_span_candidates: usize,

    /// 每个位置最多保留几条部分路径。
    pub beam_width: usize,
}

impl SearchLimits {
    /// 当前缺省：当年在真实回放日志上调过的一组。
    pub const DEFAULT: Self = Self {
        span_candidates: 6,
        abbreviated_span_candidates: 20,
        beam_width: 8,
    };
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// 语言模型不认识、只能按词库词频兜底的词扣多少分：模型见过的词更可信。
pub const FALLBACK_PENALTY: f64 = -4.0;

/// 模型不认识的词的兜底 log 概率：词库词频占总词频的比例再扣 [`FALLBACK_PENALTY`]。`log_total` 是总词频的对数。
pub fn fallback_log_prob(frequency: u32, log_total: f64) -> f64 {
    (f64::from(frequency) + 1.0).ln() - log_total + FALLBACK_PENALTY
}

/// `log P(word | context)`：先问静态模型（只看前一个词，不认识就用 `fallback`），再与个人 n-gram（看前两个词）插值。
/// 整句路径上的每一步和词级排序的上下文得分都用它。
pub fn transition_log_prob(
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    context: Context<'_>,
    word: &str,
    fallback: f64,
) -> f64 {
    let base = model.log_prob(context.previous, word).unwrap_or(fallback);
    personal.blend(context, word, base)
}
