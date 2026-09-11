//! Engine：Core 对外的唯一门面。
//!
//! 平台层只跟这里打交道：喂按键、拿候选、上屏。翻译与学习通过 trait 注入，
//! 默认实现都是空操作，所以单元测试和 CLI 不需要真实词典也能跑。

mod alignment;
mod annotation;
mod commit;
mod composing;
mod correcting;
mod extras;
mod gloss;
mod input_log;
mod learning;
mod marked;
mod mode_keys;
mod prediction;
mod query;
mod setup;
mod statistics;
mod timings;
mod translator;
mod vocabulary;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use qingjian_dictionary::{Dictionary, Match, WordList};

pub use alignment::Alignment;
pub use annotation::AnnotationReport;
pub use commit::{LastCommit, Transition};
pub use gloss::{FilledGloss, GlossFiller, NoGlossFiller};
pub use input_log::{
    CommitEntry, InputLogEntry, InputLogger, InputSource, LOGGED_CANDIDATES, NoInputLogger,
};
pub use learning::{Forgotten, Learner, NoLearner};
pub use marked::{MarkedKind, MarkedSegment};
pub use mode_keys::{ModeKeys, QUESTION_PREFIX};
pub use prediction::{
    CloudWord, NoPredictor, Prediction, PredictionKind, PredictionPolicy, PredictionRequest,
    Predictor, SurroundingText,
};

pub use query::Query;
pub use statistics::{BOOKS, Book, NoUsageMeter, Usage, UsageMeter, UsageSummary, book_scale};
pub use timings::Timings;
pub use translator::{NoTranslator, Translator};
pub use vocabulary::{
    FRESH_UNTIL, LevelCount, NoVocabularyTracker, VocabularySummary, VocabularyTracker,
};

use crate::candidate::{Candidate, CandidateKind, CandidateList, Language, Translation};
use crate::composition::Composition;
use crate::correction::{self, Correction, typo};
use crate::emoji::EmojiTable;
use crate::english;
use crate::fuzzy::{Expanded, FuzzyRules};
use crate::history::InputHistory;
use crate::parser::{self, ParseError, Segmentation};
use crate::punctuation::Punctuation;
use crate::ranking::{self, Scored};
use crate::sentence::{self, Conversion, LanguageModel, NoLanguageModel, SentenceScorer};
use crate::shortcut;
use crate::shuangpin::{Decoded, Scheme};

use commit::CommitChain;

pub struct Engine {
    /// 静态词库。
    dictionary: Dictionary,

    /// 译文提供方，缺省为 [`NoTranslator`]。
    translator: Box<dyn Translator>,

    /// 前缀模式键（表达式 / 问字）。
    modes: ModeKeys,

    /// 附加词库（领域词库、用户导入的），与主词库一起查词、一起进整句词图；不参与语言模型（它们没有 bigram，
    /// 走词频兜底）。壳按用户目录 `dicts/` 与配置 `[dictionaries]` 装配。
    extra_dictionaries: Vec<Dictionary>,

    /// 英文候选的释义（英→中），缺省为 [`NoTranslator`]。英文候选的辅助语言是主语言中文，
    /// 与中文候选查学习语言的表分开，仍是「一个候选只显示一种辅助语言」。
    english_translator: Box<dyn Translator>,

    /// 用户词频，缺省为 [`NoLearner`]。
    learner: Box<dyn Learner>,

    /// 当前拼音缓冲区。
    composition: Composition,

    /// 英文词表，中英混输用；没有就不出英文候选。
    english: Option<WordList>,

    /// 英文模式（壳里 Caps Lock 亮着）：缓冲区里的字母不当拼音，候选来自英文词表的补全与纠正。
    english_mode: bool,

    /// 全角标点与引号配对状态。
    punctuation: Punctuation,

    /// 联想提供方，缺省为 [`NoPredictor`]。
    predictor: Box<dyn Predictor>,

    /// 整句转换的语言模型，缺省为 [`NoLanguageModel`]（退化成一元词频）。
    language_model: Box<dyn LanguageModel>,

    /// 整句路径的神经重打分器（字级 Transformer）；没接就只用词级模型的路径得分。
    sentence_scorer: Option<Box<dyn SentenceScorer>>,

    /// 重打分时神经得分的权重 λ：最终分 = 路径分 + λ·(神经分 − 静态分)。
    neural_weight: f64,

    /// 只有路径分与最优路径差距在这么多 nat 以内的路径才参与重排：差距大的多半是个人 n-gram 拉开的，通用模型不该翻盘。
    neural_margin: f64,

    /// 重打分给模型看的前文长度（本会话最近上屏的字符数），0 为不给前文。
    neural_context: usize,

    /// 最近几次上屏各记了哪些学习、之后退格了几个字；用户把它们删掉重选时把学习退回去（见 [`Self::note_backspace`]）。
    /// 最新的在末尾，最多留 [`RECENT_COMMITS`] 条。
    recent_commits: Vec<LastCommit>,

    /// 本次 commit 里记下的词转移，commit 结束时搬进 `last_commit`。
    recording: Vec<Transition>,

    /// 输入日志的落盘方；缺省不记。
    logger: Box<dyn InputLogger>,

    /// 输入日志条目的序号。
    log_sequence: u64,

    /// 输入统计的累计方（打了多少字）；缺省不记。
    meter: Box<dyn UsageMeter>,

    /// 学习语言的词汇记录（见过 / 上屏过哪些译词）；缺省不记也不标生词。
    vocabulary: Box<dyn VocabularyTracker>,

    /// 释义兜底：释义表里没有的词上屏后问云端；缺省不问。
    gloss_filler: Box<dyn GlossFiller>,

    /// 候选窗口当前页上的译词（壳每次画完告知），上屏时记成「看到过」。
    displayed: Vec<(Language, String)>,

    /// 上一次查询的摘要，上屏时写进输入日志。
    last_query: std::cell::RefCell<Option<query::QuerySnapshot>>,

    /// 上一次算过的拼写纠正：(作用域, 结果)。query 算一次，commit / take_raw 复用，别再跑一遍变体枚举。
    correction_cache: std::cell::RefCell<Option<(String, Option<Correction>)>>,

    /// 整句转换的格子候选缓存：跨按键复用，学习数据一变就清（见 [`Self::forget_span_cache`]）。
    span_cache: std::cell::RefCell<sentence::SpanCache>,

    /// 本次会话经我们上屏的文本，应用不给上下文时用它联想。
    history: InputHistory,

    /// 最近一次联想请求的序号，0 表示还没发过。
    prediction_sequence: u64,

    /// 最近一次联想请求的种类：只有组句联想的结果要按拼音校验。
    last_prediction_kind: PredictionKind,

    /// 最近一次问字请求里本地把问题拼音转成的汉字，用来剔掉模型复述问题的「答案」。
    last_question_guess: String,

    /// 连续上屏的链，个人 n-gram 与自动造词靠它。
    chain: CommitChain,

    /// 模糊音开关，缺省全关。
    fuzzy: FuzzyRules,

    /// 双拼方案，`None` 为全拼。开着时缓冲区里是双拼键，查词前先解成全拼（见 [`crate::shuangpin`]）。
    shuangpin: Option<Scheme>,

    /// emoji 表，没有就不出 emoji 候选。
    emoji: Option<EmojiTable>,
}

/// 英文补全最多几条（`compa` → company / compare / …）。
const ENGLISH_COMPLETIONS: usize = 3;

/// 原样上屏的字母串至少几个字母才当英文词学：单字母（`a`、`I`）不值得记。
const MIN_ENGLISH_WORD_LETTERS: usize = 2;

/// 英文模式一次最多给几条候选：两页足够，再往后没人翻。
const ENGLISH_MODE_CANDIDATES: usize = 18;

/// 英文补全至少要几个字母：太短的前缀谁都像。
const MIN_COMPLETION_LETTERS: usize = 3;

/// 整段末尾当英文词的尾段至少几个字母，前面的拼音头至少几个字母（见 `query::EnglishTail`）。
const MIN_ENGLISH_TAIL_LETTERS: usize = 2;
const MIN_ENGLISH_TAIL_HEAD_LETTERS: usize = 2;

/// 尾段自己也是合法拼音时（`fan`、`database`）至少几个字母才考虑英文读法：三个字母的拼音音节太多。
const MIN_PINYIN_LIKE_TAIL_LETTERS: usize = 4;

/// 句中切到英文的代价（log 概率）：尾段像拼音时英文读法要比拼音读法高出这么多才胜出。拍的，攒够日志后用 `--replay` 调。
const ENGLISH_SWITCH_PENALTY: f64 = 3.0;

/// 英文词频的下限（Zipf）：没有词频的词按百万分之一算。
const ENGLISH_ZIPF_FLOOR: f64 = 3.0;

/// emoji 只配给前几个候选，每个词最多几个、一次最多几个。
const EMOJI_SCAN: usize = 5;
const EMOJI_PER_WORD: usize = 2;
const EMOJI_TOTAL: usize = 3;

/// 自动造词：用户连着选出的两个词，合起来不在词库里、且这条接续已记过这么多次，就记成用户词。
/// 同一段拼音里连着选出来的（`qingjian` 选 青 再选 简）是「用户把它当一个词打」的强信号，两次就够；
/// 第一次可能是误选或偶然。
const AUTO_WORD_THRESHOLD_SAME_BUFFER: u32 = 2;

/// 分两段打的（`qing` 选 青、再打 `jian` 选 简）信号弱一些，要三次，免得 了我 这类虚词接续也成词。
const AUTO_WORD_THRESHOLD: u32 = 3;

/// 退格撤销最多回看几次上屏：删掉「沃德 书」两个词再重打时，要能找到两个词之前的那一次。
const RECENT_COMMITS: usize = 4;

/// 自动造出的词最多几个字：再长就不是词而是短语了。
const AUTO_WORD_MAX_CHARS: usize = 4;

/// 用户自己点选的词，转移记几份；整句路径里顺带的记一份。
/// 整句是模型自己算出来的，按空格接受它会把这条路径喂回模型，形成自我强化；用户明确改选的词要能压过这种回声。
pub const EXPLICIT_TRANSITION_WEIGHT: u32 = 2;

/// 拼音短于这个字母数不联想：一两个字母的意图太模糊，白花一次请求。
const MIN_PREDICTION_LETTERS: usize = 2;

/// 随联想请求附带的本地候选条数。
const PREDICTION_CANDIDATE_HINTS: usize = 5;

/// 拼写纠错的编辑代价（log 概率）：纠正后的整句得分要比原样转出的高出这么多才纠。
/// 相当于「敲错一个键」的先验约 1/150；原样是合法简拼（`nhao` → 你好）时两边路径一样，纠正不会赢。
const CORRECTION_PENALTY: f64 = 5.0;

/// 一次查询最多给壳多少条候选。同音字最多的音节也不到这个数，再往后都是长词，没人会翻到。
const MAX_CANDIDATES: usize = 500;

/// 神经重打分看 Viterbi 的前几条路径：束宽是 8，再多也没有。
const RESCORE_PATHS: usize = 6;

/// 神经重打分的缺省权重 λ（见 `Engine::neural_weight`）：整句评测集上 0.5 到 1.0 一样好、0.75 最高（见 docs/notes/neural-rescoring.md），
/// 取 0.5 给个人 n-gram 留余量；回放里看到的「λ 大整句掉」是那把尺子的偏差。
pub const NEURAL_WEIGHT: f64 = 0.5;

/// 神经重打分的缺省门槛（nat）：路径分落后最优路径超过这么多的不参与重排。缺省不设（4 nat 试过没帮助），留作调参的旋钮。
pub const NEURAL_MARGIN: f64 = f64::INFINITY;

/// 重打分给模型看的前文：本次会话最近上屏的这么多个字符。
pub const RESCORE_CONTEXT_CHARS: usize = 64;

impl Engine {
    pub fn new(dictionary: Dictionary) -> Self {
        Self {
            dictionary,
            extra_dictionaries: Vec::new(),
            translator: Box::new(NoTranslator),
            english_translator: Box::new(NoTranslator),
            modes: ModeKeys::default(),
            learner: Box::new(NoLearner),
            composition: Composition::default(),
            english: None,
            english_mode: false,
            punctuation: Punctuation::default(),
            predictor: Box::new(NoPredictor),
            language_model: Box::new(NoLanguageModel),
            sentence_scorer: None,
            neural_weight: NEURAL_WEIGHT,
            neural_margin: NEURAL_MARGIN,
            neural_context: RESCORE_CONTEXT_CHARS,
            correction_cache: std::cell::RefCell::new(None),
            span_cache: std::cell::RefCell::new(sentence::SpanCache::default()),
            recent_commits: Vec::new(),
            logger: Box::new(NoInputLogger),
            log_sequence: 0,
            meter: Box::new(NoUsageMeter),
            vocabulary: Box::new(NoVocabularyTracker),
            gloss_filler: Box::new(NoGlossFiller),
            displayed: Vec::new(),
            last_query: std::cell::RefCell::new(None),
            recording: Vec::new(),
            history: InputHistory::default(),
            prediction_sequence: 0,
            last_prediction_kind: PredictionKind::Compose,
            last_question_guess: String::new(),
            chain: CommitChain::default(),
            fuzzy: FuzzyRules::default(),
            shuangpin: None,
            emoji: None,
        }
    }
}

/// 缓冲区是否是英文直输段：含拼音键与 `'` 以外的字符（`no-way`、`a.b`），且不是表达式 / 问字模式。
/// 微软 / 搜狗双拼下 `;` 也是拼音键。
fn is_raw(text: &str, modes: ModeKeys, shuangpin: Option<Scheme>) -> bool {
    let is_key = |c: char| match shuangpin {
        Some(scheme) => scheme.is_key(c),
        None => c.is_ascii_lowercase(),
    };
    !text.is_empty()
        && !modes.is_expression(text)
        && !modes.is_question(text)
        && text.chars().any(|c| !(is_key(c) || c == '\''))
}

/// 命中是否靠模糊音：某个音节不被敲的那个模式接受。
/// 原样上屏的字母串像不像一个英文词：纯 ASCII 字母、至少两个。中文模式下还要求它**不能**切成完整的拼音
/// （`hao` 回车多半是要拼音字母本身，`gist` / `python` / `hello` 切不干净才是英文）；英文模式下敲的全是英文，不用判。
fn looks_like_english_word(raw: &str, english_mode: bool) -> bool {
    if raw.len() < MIN_ENGLISH_WORD_LETTERS || !raw.bytes().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    english_mode || !parser::is_fully_segmentable(&raw.to_ascii_lowercase())
}

/// 模式的记忆化键：完整音节原样，前缀音节后加 `*`。
fn pattern_key(pattern: &[qingjian_dictionary::SyllablePattern<'_>]) -> String {
    let mut key = String::with_capacity(pattern.len() * 7);
    for p in pattern {
        key.push_str(p.text);
        if !p.complete {
            key.push('*');
        }
        key.push(' ');
    }
    key
}

/// 末尾 `count` 个字符。
fn take_last_chars(text: &str, count: usize) -> String {
    let total = text.chars().count();
    text.chars().skip(total.saturating_sub(count)).collect()
}

/// 开头 `count` 个字符。
fn take_first_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

/// 光标后剩余拼音的显示形式：能切就按音节用 `'` 连上，切不动就原样。
fn marked_rest(rest: &str) -> String {
    if rest.is_empty() {
        return String::new();
    }
    match segment_longest_prefix(rest) {
        Ok((segmentations, tail)) => query::join_marked(&segmentations, tail),
        Err(_) => rest.to_owned(),
    }
}

/// 整段切不动时退而求其次：找能切分的最长前缀，剩余字母作为尾部返回。
/// `kaifv` → (`kai f…` 的切分, `v`)。连第一个字母都切不动才报错。
fn segment_longest_prefix(text: &str) -> Result<(Vec<Segmentation>, &str), ParseError> {
    match parser::segment(text) {
        Ok(segmentations) => Ok((segmentations, "")),
        Err(ParseError::NoSegmentation) => (1..text.len())
            .rev()
            .find_map(|end| {
                parser::segment(&text[..end])
                    .ok()
                    .map(|s| (s, &text[end..]))
            })
            .ok_or(ParseError::NoSegmentation),
        Err(error) => Err(error),
    }
}

/// 候选的音节序列在 `input` 开头覆盖了多少个字节。音节之间允许有 `'`。
///
/// 每个音节吃掉输入里与它相同的最长前缀：全拼 `kaifa` 的 开发 吃 5 个，简拼 `kf` 的 开发 吃 2 个，
/// 未打完的 `kaif` 也吃完。吃不到任何字母说明候选与输入的切分方式不一致，就此停止。
/// 按输入串记选择用的键：作用域开头 `len` 个字节里的字母（去掉分隔符 `'`），
/// 查询时按候选覆盖的字母数截取同一个串，两边才对得上。
fn choice_key(scope: &str, len: usize) -> String {
    scope[..len.min(scope.len())]
        .chars()
        .filter(|c| *c != '\'')
        .collect()
}

/// 切分里非末尾的简拼音节数，见 `ranking::Scored::abbreviated`。
fn abbreviated_count(patterns: &[qingjian_dictionary::SyllablePattern<'_>]) -> usize {
    patterns
        .iter()
        .rev()
        .skip(1)
        .filter(|p| !p.complete)
        .count()
}

#[cfg(test)]
mod tests;
