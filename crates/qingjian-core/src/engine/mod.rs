//! Engine：Core 对外的唯一门面。
//!
//! 平台层只跟这里打交道：喂按键、拿候选、上屏。翻译与学习通过 trait 注入，
//! 默认实现都是空操作，所以单元测试和 CLI 不需要真实词典也能跑。

mod alignment;
mod annotation;
mod aux_code;
mod backspace;
mod caches;
mod candidate_menu;
mod caret;
mod choice;
mod code_keys;
mod commit;
mod composing;
mod contacts;
mod correcting;
mod decoded;
mod dictionaries;
mod extras;
mod follow_up;
mod gloss;
mod held;
mod input;
mod input_log;
mod journal;
mod key_hint;
mod learning;
mod marked;
mod mode_keys;
mod pairing;
mod panel;
pub(crate) mod prediction;
mod privacy;
mod query;
mod raw;
mod reading;
mod rescoring;
mod scheme;
mod session;

mod settings;
mod setup;
mod spacing;
mod statistics;
mod timings;
mod translator;
mod vocabulary;
mod voice;

use std::collections::HashMap;
use std::sync::Arc;

use qingjian_dictionary::{AuxCodeLookup, Dictionary, WordList};

pub use alignment::Alignment;
pub use annotation::AnnotationReport;
pub use aux_code::is_valid_aux_code_key;
pub use backspace::Backspace;
pub use choice::{CharTable, ChoiceModel, ChoiceScore, ChoiceTier};
pub use commit::{LastCommit, Transition};
pub use gloss::{FilledGloss, GlossFiller, NoGlossFiller};
pub use input_log::{
    ChoiceNote, CommitEntry, INPUT_LOG_VERSION, InputLogEntry, InputLogger, InputSource,
    LOGGED_CANDIDATES, NoInputLogger,
};
pub use learning::{Forgotten, Learner, NoLearner};
pub use marked::{AuxSegment, MarkedKind, MarkedSegment};
pub use mode_keys::{ModeKeys, QUESTION_PREFIX};
pub use prediction::{
    CloudWord, NoPredictor, Prediction, PredictionKind, PredictionPolicy, PredictionRequest,
    Predictor, SurroundingText,
};

pub use query::Query;
pub use raw::RawPreedit;
pub use rescoring::Wake;
pub use session::EngineSession;
pub use statistics::{BOOKS, Book, NoUsageMeter, Usage, UsageMeter, UsageSummary, book_scale};
pub use timings::Timings;
pub use translator::{NoTranslator, Translator};
pub use vocabulary::{
    Encounter, FRESH_UNTIL, LevelCount, NoVocabularyTracker, VocabularySummary, VocabularyTracker,
};

use crate::candidate::Language;
use crate::composition::Composition;
use crate::correction::TypoCosts;
use crate::emoji::EmojiTable;
use crate::history::InputHistory;
use crate::panel::{NoPanelHistory, PanelGroup, PanelHistory, SymbolPanel, TranslationCache};
use crate::parser::{self, ParseError, Segmentation};
use crate::punctuation::Punctuation;
use crate::sentence::{self, Interpolation, LanguageModel, NoLanguageModel, SentenceScorer};
use crate::shuangpin::Layout;
use crate::traditional::Traditional;

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

    /// 从三元模型词表补出的词（补充词库），壳在接上三元模型时装配。单放一格：附加词库随导入 / 开关整个换掉，它不跟着换。
    model_words: Option<Dictionary>,

    /// 内置的符号、emoji 名字（尖括号、笑死），带上 emoji 表时启用：词库里没有它们，不收就打不出这个词、也就配不上符号。
    companion_words: Option<&'static Dictionary>,

    /// 英文候选的释义（英→中），缺省为 [`NoTranslator`]。英文候选的辅助语言是主语言中文，
    /// 与中文候选查学习语言的表分开，仍是「一个候选只显示一种辅助语言」。
    english_translator: Box<dyn Translator>,

    /// 用户词频，缺省为 [`NoLearner`]；私密输入期间只读不写（[`learning::MutedLearner`]）。
    learner: learning::MutedLearner,

    /// 当前拼音缓冲区。
    composition: Composition,

    /// 英文词表，中英混输用；没有就不出英文候选。
    english: Option<WordList>,

    /// 英文模式（壳里 Caps Lock 亮着）：缓冲区里的字母不当拼音，候选来自英文词表的补全与纠正。
    english_mode: bool,

    /// 全角标点与引号配对状态。
    punctuation: Punctuation,

    /// 中文标点转换开关。
    full_width_punctuation: bool,

    /// 中文标点开着也保持半角的符号（`[general] half_width_symbols`，#893）。
    half_width_symbols: Vec<char>,

    /// 成对符号开关：中文与日文模式一份、英文模式一份（见 [`Self::pair_punctuation`]）。
    pair_punctuation: (bool, bool),

    /// 用户定义的短语，按输入码精确匹配、按权重排在最前。
    custom_phrases: Vec<crate::CustomPhrase>,

    /// 屏蔽词：候选文本里含有其中任何一个就整条丢掉。自定义短语不过这一关。
    blocked: Vec<String>,

    /// 通讯录（壳读系统通讯录交来，用户在设置里开了才有）：名字并进查词，见 [`Self::set_contacts`]。
    contacts: Option<crate::ContactBook>,
    settings: settings::EngineSettings,
    caches: caches::EngineCaches,
    journal: journal::EngineJournal,
    input: input::EngineInput,
    scheme: scheme::EngineScheme,

    /// 联想提供方，缺省为 [`NoPredictor`]。
    predictor: Box<dyn Predictor>,

    /// 显式选区翻译的独立提供方；未设置时沿用联想服务。
    translation_predictor: Option<Box<dyn Predictor>>,

    /// 整句转换的语言模型，缺省为 [`NoLanguageModel`]（退化成一元词频）。
    language_model: Box<dyn LanguageModel>,

    /// 整句路径的同步神经重打分器（字级 Transformer，查询里当场打分；CLI 评测用）。
    sentence_scorer: Option<Box<dyn SentenceScorer>>,

    /// 异步重打分：后台线程里的打分器，壳在停顿后送任务、轮询结果（见 [`rescoring`]）。
    rescorer: Option<rescoring::RescoreWorker>,

    /// 接的打分器是生成器（[`SentenceScorer::generates`]，含章·通变）：整句以生成为主、不重排词图路径。
    generative: bool,

    /// 后台模型交结果时叫醒壳（[`Engine::set_rescoring_wake`]），两个后台线程共用。
    wake: rescoring::WakeSlot,

    /// 选词模型（试验）：候选窗前几条交给它重排一次，见 [`choice`]。
    choice: Option<choice::ChoiceRanker>,

    /// 选词模型比几条、看多长前文。
    choice_tier: choice::ChoiceTier,

    /// 壳给的应用里光标前的文本；`None` 时前文用本会话历史。
    rescoring_before: Option<String>,

    /// 重打分时神经得分的权重 λ：最终分 = 路径分 + λ·(神经分 − 静态分)。
    neural_weight: f64,

    /// 只有路径分与最优路径差距在这么多 nat 以内的路径才参与重排：差距大的多半是个人 n-gram 拉开的，通用模型不该翻盘。
    neural_margin: f64,

    /// 重打分给模型看的前文长度（本会话最近上屏的字符数），0 为不给前文。
    neural_context: usize,

    /// 个人 n-gram 与静态模型插值的参数；只有回放调参会改（`set_interpolation`），壳用缺省值。
    interpolation: Interpolation,

    /// 整句转换的搜索规模；同上，只有回放调参会改（`set_search_limits`）。见 [`sentence::SearchLimits`]。
    search_limits: sentence::SearchLimits,

    /// 敲错纠正的代价；同上，只有回放调参会改（`set_typo_costs`）。
    typo_costs: TypoCosts,

    /// 最近几次上屏各记了哪些学习、之后退格了几个字；用户把它们删掉重选时把学习退回去（见 [`Self::note_backspace`]）。

    /// 学习语言的词汇记录（见过 / 上屏过哪些译词）；缺省不记也不标生词。
    vocabulary: Box<dyn VocabularyTracker>,

    /// 释义兜底：释义表里没有的词上屏后问云端；缺省不问。
    gloss_filler: Box<dyn GlossFiller>,

    /// 上屏过带译词的词、等那句话写完当例句。
    examples: vocabulary::ExampleCollector,

    /// 短语级语言模型（知意的三元 n-gram）：选词重排里给整段候选打 ln P，没接时用二元模型逐词累加。
    phrase_model: Option<std::sync::Arc<dyn sentence::PhraseModel>>,

    /// 本次会话经我们上屏的文本，按应用分开记；应用不给光标前文时给模型当前文。
    history: InputHistory,

    /// 最近一次联想请求的序号，0 表示还没发过。
    prediction_sequence: u64,

    /// 尚未返回的请求。组句联想与整句候选翻译可同时在飞，不能只记最后一条。
    pending_predictions: HashMap<u64, prediction::PendingPrediction>,

    /// 连续上屏的链，个人 n-gram 与自动造词靠它。
    chain: CommitChain,

    /// emoji 表，没有就不出 emoji 候选。
    emoji: Option<EmojiTable>,

    /// 内置的颜文字表，带上 emoji 表时一起启用。
    kaomoji: Option<&'static EmojiTable>,

    /// 内置的符号表，带上 emoji 表时一起启用。
    symbols: Option<&'static EmojiTable>,

    /// 打开着的符号面板；`None` 是没开。上屏与清空时关掉。
    panel: Option<SymbolPanel>,

    /// 面板条目的使用记录：打开面板时排「常用」，从面板上屏时记一次；缺省不记。
    panel_history: Box<dyn PanelHistory>,

    /// 面板上次关掉时停在哪组：没在组句时打开就回到那组。
    panel_group: Option<PanelGroup>,

    /// 翻译组上次选的目标语言；没选过跟学习语言走。
    panel_language: Option<Language>,

    /// 翻译组的译文缓存，换联想提供方时清空。
    panel_translations: TranslationCache,

    /// 辅码态：`None` 是拼音态，`Some` 是辅码态（空串 = 刚敲下触发键、码段还没开始）。
    /// 码段不进 `composition`：它与拼音分段记账、分段画（见 [`AuxSegment`]）。
    aux_code: Option<String>,

    /// 辅码码表，壳按用户目录 `codes/` 与配置装配；空表示没装码表（辅码态筛不出任何词）。
    aux_codes: Vec<Arc<dyn AuxCodeLookup>>,

    /// 繁体输出：候选出 Core 时换成繁体、上屏时换回简体，缺省不转（见 [`crate::traditional`]）。
    traditional: Traditional,
}

/// 英文补全最多几条（`compa` → company / compare / …）。
const ENGLISH_COMPLETIONS: usize = 3;

/// 中文模式下英文候选的缺省上限：1 条精确词 + 3 条补全，与以前的行为一致。
pub const ENGLISH_CANDIDATES_MAX: usize = ENGLISH_COMPLETIONS + 1;

/// 原样上屏的字母串至少几个字母才当英文词学：单字母（`a`、`I`）不值得记。
const MIN_ENGLISH_WORD_LETTERS: usize = 2;

/// 原样上屏的字母串最多几个字母才当英文词学：再长的多半不是一个词，是一整句打不出来的输入
/// （`yongdockerbushuhenfangbian`），学进去会反过来堵住混输那条路。
/// 15 是 2026-09-25 按数据定的：用户个人表 1102 条里超过 15 个字母的 9 条全是失败的拼音、无一真词；
/// 随包英文词表 94568 条里超过 15 个字母的只占 0.69%，而那些词基本都已经在随包表里、不靠学。
/// 英文模式下不设这个上限：那时敲的本来就是英文。
const MAX_ENGLISH_WORD_LETTERS: usize = 15;

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

/// emoji、颜文字、符号只配给前几个候选，每个词最多几个、每样一次最多几个。
/// 颜文字宽，一个词只跟一个，免得把横排撑长。
const EMOJI_SCAN: usize = 5;
const EMOJI_PER_WORD: usize = 2;
const KAOMOJI_PER_WORD: usize = 1;
const EMOJI_TOTAL: usize = 3;

/// 「日期 / 时间 / 星期」这几个词排在前几条里才跟快捷写法（同 emoji 配词的范围；九宫格 7474 的日期常排第四）：再往后多半是长输入的前缀词。
const KEYWORD_SCAN: usize = 5;

/// 快捷写法最多几条紧跟第一个词（sj 的十五条全插在前面，会把简拼原来第一页的
/// 词——世纪、时间——整片推到后面去）。其余垫到候选末尾。
const INLINE_SHORTCUTS: usize = 3;

/// 自动造词：用户连着选出的两个词，合起来不在词库里、且这条接续已记过这么多次，就记成用户词。
/// 同一段拼音里连着选出来的（`qingjian` 选 青 再选 简）是「用户把它当一个词打」的强信号，一次就造：
/// 下次整段与简拼都排在前面，不用逐字再选；选错造出的词只在敲同样的音时出现，可以删掉。
const AUTO_WORD_THRESHOLD_SAME_BUFFER: u32 = 1;

/// 分两段打的（`qing` 选 青、再打 `jian` 选 简）信号弱一些，要三次，免得 了我 这类虚词接续也成词。
const AUTO_WORD_THRESHOLD: u32 = 3;

/// 退格撤销最多回看几次上屏：删掉「沃德 书」两个词再重打时，要能找到两个词之前的那一次。
const RECENT_COMMITS: usize = 4;

/// 全拼选过的词按简拼另记一份选择时键的前缀（`^ss`）：与真在那串键下的选择分开，排序时排在它们后面，
/// 中英文谁在前也不看它。真的键只有小写字母，不会撞上。
const ABBREVIATION_CHOICE_PREFIX: char = '^';

/// 前后两个词自动合成的词最多几个字：再长就不是词而是短语了。
const AUTO_WORD_MAX_CHARS: usize = 4;

/// 一次选中的整句（几个多字词拼成，交易 + 中心）记成用户词的最多几个字：一次接受的长句多半是模型给的首选，
/// 下次一字不差再打的机会也小（分几次选完的见 [`BUFFER_PHRASE_MAX_CHARS`]）。冻结回放里 4 字与 6 字逐条相同，取宽的。
const SENTENCE_WORD_MAX_CHARS: usize = 6;

/// 一段拼音分几次选完、合起来记成用户词的最多几个字（整句记忆）：用户逐字纠正过的整句，下次同样的全拼或简拼直接出。
/// 再长的多半是一大段话，下次一字不差再打一遍的机会很小。
const BUFFER_PHRASE_MAX_CHARS: usize = 20;

/// 用户自己点选的词，转移记几份；整句路径里顺带的记一份。
/// 整句是模型自己算出来的，按空格接受它会把这条路径喂回模型，形成自我强化；用户明确改选的词要能压过这种回声。
pub const EXPLICIT_TRANSITION_WEIGHT: u32 = 2;

/// 拼音短于这个字母数不联想：一两个字母的意图太模糊，白花一次请求。
const MIN_PREDICTION_LETTERS: usize = 2;

/// 辅码触发键的缺省值（`[general] aux_code_key`）。
pub const DEFAULT_AUX_CODE_KEY: char = ';';

/// 随联想请求附带的本地候选条数。
const PREDICTION_CANDIDATE_HINTS: usize = 5;

/// 一次查询最多给壳多少条候选。同音字最多的音节也不到这个数，再往后都是长词，没人会翻到。
const MAX_CANDIDATES: usize = 500;

/// 神经重打分看 Viterbi 的前几条路径：束宽是 8，再多也没有。
const RESCORE_PATHS: usize = 6;

/// 长输入最多给几条整句候选（重排后的前几条路径）：一条不够用，长句错一个字就得拆开重打。
const SENTENCE_CANDIDATES: usize = 3;

/// 接了选词模型时长输入给几条整句候选：词图交出前这么多条不同的整句，由选词模型带上文整句重排，
/// 重排后只显示前 [`SENTENCE_CANDIDATES`] 条（`trim_sentence_alternates`）。
const CHOICE_SENTENCE_CANDIDATES: usize = 16;

/// 结构上打平的切分（`shang ou` / `shan gou`）最多比几种，见 [`Engine::sentence_segmentation`]。
const TIED_SEGMENTATIONS: usize = 3;

/// 短于这么多音节的不给备选整句：那时候选表里的词级候选比另一种读法有用。
const ALTERNATE_MIN_SYLLABLES: usize = 4;

/// 短于这么多字母的不让模型直接生成整句：那么短的输入词级候选够用，生成的几十毫秒不值。
const MIN_GENERATED_LETTERS: usize = 6;

/// 接了生成器时，拼音干净且至少这么多音节，生成的整句才排在词图前面；更短的词图在前（带着个人习惯），生成的排第二。
/// 短输入上生成器不懂个人习惯，会盖掉用户常打的词；门槛怎么定的见 `docs/notes/crate-notes.md`（qingjian-core「生成器」）。
const GENERATED_LEAD_MIN_SYLLABLES: usize = 6;

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
            model_words: None,
            companion_words: None,
            translator: Box::new(NoTranslator),
            english_translator: Box::new(NoTranslator),
            modes: ModeKeys::default(),
            learner: learning::MutedLearner::new(Box::new(NoLearner)),
            composition: Composition::default(),
            english: None,
            english_mode: false,
            punctuation: Punctuation::default(),
            full_width_punctuation: true,
            half_width_symbols: Vec::new(),
            pair_punctuation: (true, false),
            custom_phrases: Vec::new(),
            blocked: Vec::new(),
            contacts: None,
            settings: settings::EngineSettings::default(),
            caches: caches::EngineCaches::default(),
            journal: journal::EngineJournal::default(),
            input: input::EngineInput::default(),
            scheme: scheme::EngineScheme::default(),
            predictor: Box::new(NoPredictor),
            translation_predictor: None,
            language_model: Box::new(NoLanguageModel),
            sentence_scorer: None,
            rescorer: None,
            generative: false,
            wake: rescoring::WakeSlot::default(),
            choice: None,
            choice_tier: choice::ChoiceTier::default(),
            rescoring_before: None,
            neural_weight: NEURAL_WEIGHT,
            neural_margin: NEURAL_MARGIN,
            interpolation: Interpolation::DEFAULT,
            typo_costs: TypoCosts::DEFAULT,
            search_limits: sentence::SearchLimits::DEFAULT,
            neural_context: RESCORE_CONTEXT_CHARS,
            phrase_model: None,
            vocabulary: Box::new(NoVocabularyTracker),
            gloss_filler: Box::new(NoGlossFiller),
            examples: vocabulary::ExampleCollector::default(),
            history: InputHistory::default(),
            prediction_sequence: 0,
            pending_predictions: HashMap::new(),
            chain: CommitChain::default(),
            emoji: None,
            kaomoji: None,
            symbols: None,
            panel: None,
            panel_history: Box::new(NoPanelHistory),
            panel_group: None,
            panel_language: None,
            panel_translations: TranslationCache::default(),
            aux_code: None,
            aux_codes: Vec::new(),
            traditional: Traditional::default(),
        }
    }
}

/// 缓冲区是否是英文直输段：含拼音键与 `'` 以外的字符（`no-way`、`a.b`），且不是表达式 / 问字模式。
/// 微软 / 搜狗双拼下 `;` 也是拼音键。
fn is_raw(text: &str, modes: ModeKeys, shuangpin: Option<&Layout>, zhuyin: bool) -> bool {
    let is_key = |c: char| {
        if zhuyin {
            crate::zhuyin::layout::map_key(c).is_some() || c == ' '
        } else {
            match shuangpin {
                Some(scheme) => scheme.is_key(c),
                None => c.is_ascii_lowercase(),
            }
        }
    };
    !text.is_empty()
        && !modes.is_expression(text, zhuyin)
        && !modes.is_question(text, zhuyin)
        && text.chars().any(|c| !(is_key(c) || c == '\''))
}

/// 命中是否靠模糊音：某个音节不被敲的那个模式接受。
/// 原样上屏的字母串像不像一个英文词：纯 ASCII 字母、至少两个。中文模式下还要求它**不能**切成完整的拼音
/// （`hao` 回车多半是要拼音字母本身，`gist` / `python` / `hello` 切不干净才是英文），
/// 且不能长过 [`MAX_ENGLISH_WORD_LETTERS`]；英文模式下敲的全是英文，两条都不用判。
fn looks_like_english_word(raw: &str, english_mode: bool) -> bool {
    if raw.len() < MIN_ENGLISH_WORD_LETTERS || !raw.bytes().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    english_mode
        || (raw.len() <= MAX_ENGLISH_WORD_LETTERS
            && !parser::is_fully_segmentable(&raw.to_ascii_lowercase()))
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
        Ok((segmentations, tail)) => query::join_spelled(&segmentations, tail),
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
/// 这种切分每个音节都完整：敲的拼音照原样就读得通。这时首选只按原样读，拼写纠错的读法只当备选。
fn reads_as_typed(segmentation: &Segmentation) -> bool {
    segmentation.incomplete_count() == 0
}

fn abbreviated_count(patterns: &[qingjian_dictionary::SyllablePattern<'_>]) -> usize {
    patterns
        .iter()
        .rev()
        .skip(1)
        .filter(|p| !p.complete)
        .count()
}

/// 候选读音的学习键：音节无分隔符连起来（`kai fa` → `kaifa`）。
/// 按输入串记选择时与字面键各记一份，换个打法也认账（见 [`Learner::record_choice`]）。
fn reading_key<'a>(syllables: impl IntoIterator<Item = &'a str>) -> String {
    syllables.into_iter().collect()
}

#[cfg(test)]
mod tests;
