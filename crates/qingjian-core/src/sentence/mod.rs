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
//! 全拼句子末尾没打完的单字母不参与，简拼句子里末尾单字母就是一个音节。
//!
//! 格子查词是最贵的一步，结果放进 [`SpanCache`]：敲键是增量的，每一键只有以它结尾的几个格子是新的。

mod context;
mod conversion;
mod language_model;
mod scorer;
mod sentence_word;
mod span;
mod text_segment;
mod user_ngram;
mod viterbi;

pub use context::Context;
pub use conversion::Conversion;
pub use language_model::{LanguageModel, NoLanguageModel};
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

/// 末尾未打完的音节至少要几个字母才参与整句：单个字母的前缀范围太大（`s` 匹配所有 s 开头的音节），不值得扫。
pub const MIN_PARTIAL_LETTERS: usize = 2;

/// 每个格子最多留几个词（按词库词频 + 用户加分）。同音词很多，全留会让束搜索白费。
pub const SPAN_CANDIDATES: usize = 6;

/// 有简拼位置的格子最多留几个词：`h` 下有 和 / 好 / 会 / 还 / 很 …… 几十个常用字，
/// 只留六个会把句子里要的那个挤掉，多留一些让语言模型去挑。
pub const ABBREVIATED_SPAN_CANDIDATES: usize = 20;

/// 每个位置最多保留几条部分路径。
pub const BEAM_WIDTH: usize = 8;

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
    personal: Option<&UserNgram>,
    context: Context<'_>,
    word: &str,
    fallback: f64,
) -> f64 {
    let base = model.log_prob(context.previous, word).unwrap_or(fallback);
    personal.map_or(base, |p| p.blend(context, word, base))
}
