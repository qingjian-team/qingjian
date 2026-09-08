use qingjian_dictionary::{Dictionary, WordList};

use super::Forgotten;
use crate::candidate::Candidate;
use crate::sentence::{Context, UserNgram};

/// 用户词频学习。
pub trait Learner: Send {
    /// 用户上屏了某个候选。
    fn record(&mut self, candidate: &Candidate);

    /// 该词被用户选择过的次数，没记录返回 0。
    fn weight(&self, text: &str) -> u32;

    /// 用户在输入串 `input`（候选覆盖的那段拼音，不含分隔符）下选了 `text`。
    /// 词级排序里同一输入串下选过的词排最前（`mgs` 选过 美国式，下次 `mgs` 它就是首选），与不分输入的 [`Self::weight`] 分开记：
    /// `ba` 下选的是 吧，`bazhege` 下选的是 把，混在一起数就分不清。
    fn record_choice(&mut self, _input: &str, _text: &str) {}

    /// `text` 在输入串 `input` 下被选过的次数，没记录返回 0。
    fn choice_weight(&self, _input: &str, _text: &str) -> u32 {
        0
    }

    /// 用户对输入串 `input` 按了回车原样上屏，而当时拼写纠错正生效：这个串就是要原样打的，以后不纠。
    fn record_raw(&mut self, _input: &str) {}

    /// `input` 被原样上屏过几次（见 [`Self::record_raw`]）。
    fn raw_count(&self, _input: &str) -> u32 {
        0
    }

    /// 撤销一次 [`Self::record`]：用户上屏后马上整个删掉重选了别的词，刚才那次不算数。
    fn unrecord(&mut self, _text: &str) {}

    /// 撤销一次 [`Self::record_choice`]。
    fn unrecord_choice(&mut self, _input: &str, _text: &str) {}

    /// 撤销 `times` 份 [`Self::record_transition`]。
    fn unrecord_transition(&mut self, _context: Context<'_>, _word: &str, _times: u32) {}

    /// 记一个词库里没有的词（来自云联想或以后的自动造词），带全拼音节；下次直接从本地出。
    fn learn_word(&mut self, _text: &str, _syllables: &[String]) {}

    /// 用户词组成的小词库，与主词库一起查；没有就返回 `None`。
    fn user_words(&self) -> Option<&Dictionary> {
        None
    }

    /// 用户原样上屏了一个像英文词的字母串（中文模式按回车、英文模式空格 / 回车直通），或选了一个英文候选：
    /// 记进个人英文词表，下次它就是英文候选，而且排在随包词表的同形词前面。随包词表里没有的词（`gist`）只能靠这里学。
    fn learn_english(&mut self, _word: &str) {}

    /// 个人英文词表，与随包词表一起给英文候选（它在前）；没有就返回 `None`。
    fn user_english(&self) -> Option<&WordList> {
        None
    }

    /// 记一条词序列转移：`word` 在上文 `context`（前一个词与再前一个词，句首都是 `None`）之后上屏。
    /// 整句上屏按路径上的词逐条记，连续选词上屏也记；喂个人 n-gram（二元与三元一起记）。
    /// `times` 是这次记几份：用户自己点选的词记双份（[`EXPLICIT_TRANSITION_WEIGHT`]），整句路径里顺带的记一份，
    /// 否则一次误按空格上屏的整句要用户改选两次才能翻回来。
    ///
    /// [`EXPLICIT_TRANSITION_WEIGHT`]: super::EXPLICIT_TRANSITION_WEIGHT
    fn record_transition(&mut self, _context: Context<'_>, _word: &str, _times: u32) {}

    /// 个人 n-gram，整句转换与词级排序用它与静态模型插值；没有就返回 `None`。
    fn user_ngram(&self) -> Option<&UserNgram> {
        None
    }

    /// 用户接受了一处音节级的敲错纠正：把 `typed`（敲的那段字母）当成了 `intended`（候选的音节）上屏。
    /// 记进个人敲错表，以后词图里这条边更便宜（见 `correction::typo_cost`）。
    fn record_typo(&mut self, _typed: &str, _intended: &str) {}

    /// 撤销一次 [`Self::record_typo`]。
    fn unrecord_typo(&mut self, _typed: &str, _intended: &str) {}

    /// `typed` 被当成 `intended` 接受过几次。
    fn typo_count(&self, _typed: &str, _intended: &str) -> u32 {
        0
    }

    /// 用户要求删掉这个词：它是用户词就删掉，同时清掉选择次数、各输入串下的选择、个人 n-gram 里与它有关的转移。
    /// 返回实际清掉了什么。
    fn forget(&mut self, _text: &str) -> Forgotten {
        Forgotten::default()
    }

    /// 用户要求删掉一个个人英文词。返回是否真有这个词。
    fn forget_english(&mut self, _word: &str) -> bool {
        false
    }

    /// 把学习数据落盘。壳在退出或停用输入法时调用；内存实现留空即可。
    ///
    /// 失败只记日志不返回错误：输入优先于学习，持久化失败不该影响壳的流程。
    fn flush(&mut self) {}
}

/// 不学习。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLearner;

impl Learner for NoLearner {
    fn record(&mut self, _candidate: &Candidate) {}

    fn weight(&self, _text: &str) -> u32 {
        0
    }
}
