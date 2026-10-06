use crate::candidate::Candidate;
use crate::candidate::CandidateKind;
use crate::correction;
use crate::engine::ENGLISH_SWITCH_PENALTY;
use crate::engine::ENGLISH_ZIPF_FLOOR;
use crate::engine::Engine;
use crate::engine::MIN_COMPLETION_LETTERS;
use crate::engine::MIN_ENGLISH_TAIL_HEAD_LETTERS;
use crate::engine::MIN_ENGLISH_TAIL_LETTERS;
use crate::engine::MIN_PINYIN_LIKE_TAIL_LETTERS;
use crate::engine::learning::Learner;
use crate::parser;
use crate::parser::Segmentation;
use crate::sentence::Conversion;

/// 「拼音头 + 英文尾」的切法：`woxiangxuehaorust` 切成头 `woxiangxuehao` 与尾 rust。
/// 见 [`Engine::split_english_tail`]。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EnglishTail {
    /// 头段（拼音）占作用域开头多少字节。
    pub head_len: usize,

    /// 尾段对应的英文词，按词表里的写法（`api` → API）。
    pub word: String,

    /// 整段字母也能读成拼音（`database` → da ta ba se，`woxiangxuehaorust` → … ru s… t… 简拼）：两种读法要比分，
    /// 见 [`Engine::mixed_beats_plain`]；整段根本切不成拼音的（`wodeid` 的 i、`woyongvim` 的 v）直接按英文读。
    pub competes: bool,

    /// 这个英文词的 log 概率（按词表词频估），比分用。
    pub log_prob: f64,
}

/// wordfreq 的 Zipf 频率（词表里 ×1000 存）换成 log 概率：Zipf z 是每十亿词里 10^z 次，即 log P = (z − 9)·ln 10。
/// 没有词频的（个人表里的词、`kubectl` 这类技术词）按 Zipf 3（百万分之一）算。
pub(crate) fn english_log_prob(frequency: Option<u32>) -> f64 {
    let zipf = frequency
        .map(|f| f64::from(f) / 1000.0)
        .filter(|z| *z > 0.0)
        .unwrap_or(ENGLISH_ZIPF_FLOOR)
        .max(ENGLISH_ZIPF_FLOOR);
    (zipf - 9.0) * std::f64::consts::LN_10
}

impl Engine {
    /// 整段输入的末尾是不是一个英文词：`woxiangxuehaorust` → 我想学好 + rust。
    ///
    /// 尾段要在英文词表里（个人表或随包表），头段要能切成每个音节都完整的拼音。
    /// 尾段自己就是完整拼音的（`database`、`fan`）要至少 [`MIN_PINYIN_LIKE_TAIL_LETTERS`] 个字母；
    /// 两个字母的尾段只认缩写词（ID / TV / OK）和个人表里的词：`to` / `it` 这类太容易撞上简拼。
    /// 同时满足的取最长的尾段（`wodedatabase` 取 database 不取 base）。双拼、带 `'` 的输入不切；
    /// 整段本身是英文词（`agent`）、拼音不像话时纠错能纠通（`shiide` → 是的）或有英文补全（`releas` → release）的也不切，
    /// 那几条路本来就排第一。
    /// 切出来只说明「可以这么读」，与拼音读法谁排前面看 `competes` 与比分。
    pub(crate) fn split_english_tail(&self, scope: &str) -> Option<EnglishTail> {
        if self.shuangpin.is_some()
            || scope.len() < MIN_ENGLISH_TAIL_HEAD_LETTERS + MIN_ENGLISH_TAIL_LETTERS
            || !scope.bytes().all(|b| b.is_ascii_lowercase())
        {
            return None;
        }
        let lists = self.english_lists();
        if lists.is_empty() {
            return None;
        }
        // 整段是随包表里的英文词（`agent`）：那条路本来就排第一，不切。
        // 个人表不算：打不出来时原样上屏会把整串失败的拼音（`woxiangxuexirust`）学成「英文词」，
        // 再拿它把混输这条路堵掉就是死循环——越打不出来学得越多，越学越打不出来
        if self
            .english
            .as_ref()
            .is_some_and(|words| words.get(scope).is_some())
        {
            return None;
        }
        let full = parser::segment(scope).ok();
        let unlikely = correction::unlikely_pinyin(full.as_ref().and_then(|s| s.first()), "")
            || correction::trailing_single_letter(full.as_ref().and_then(|s| s.first()));
        if full.is_none() || unlikely {
            // 拼写纠错能把整段纠成通顺的拼音（`shiide` → 是的，`yingagi` → 应该）：那是敲错，不是英文
            if self.active_correction(scope).is_some() {
                return None;
            }
            if scope.len() >= MIN_COMPLETION_LETTERS
                && lists
                    .iter()
                    .any(|words| !words.complete(scope, 1).is_empty())
            {
                return None;
            }
        }
        let personal = self.learner.user_english();
        let longest = scope.len() - MIN_ENGLISH_TAIL_HEAD_LETTERS;
        (MIN_ENGLISH_TAIL_LETTERS..=longest).rev().find_map(|len| {
            let head_len = scope.len() - len;
            let tail = &scope[head_len..];
            let words = lists.iter().find(|words| words.get(tail).is_some())?;
            let word = words.get(tail)?;
            let acronym = word.bytes().any(|b| b.is_ascii_uppercase());
            let known = personal.is_some_and(|words| words.get(tail).is_some());
            if len == MIN_ENGLISH_TAIL_LETTERS && !acronym && !known {
                return None;
            }
            if parser::is_fully_segmentable(tail) && len < MIN_PINYIN_LIKE_TAIL_LETTERS {
                return None;
            }
            if !parser::is_fully_segmentable(&scope[..head_len]) {
                return None;
            }
            Some(EnglishTail {
                head_len,
                word: word.to_owned(),
                competes: full.is_some(),
                // 词频只问随包表：个人表存的是使用次数，当 Zipf×1000 读会一律塌到兜底值
                //（`English` 用过 2 次 → Zipf 0.002 → 兜底 3.0，真实 5.19，白丢 5 nat），
                // 等于一个英文词用过一次就更难打出来。个人表里独有的词（`kubectl`）本来就走兜底
                log_prob: english_log_prob(
                    self.english
                        .as_ref()
                        .and_then(|words| words.frequency(tail)),
                ),
            })
        })
    }

    /// 整段也能读成拼音时两种读法比分：头段整句的得分加英文词的 log 概率、扣掉切到英文的代价，高过整段按拼音读的整句就按英文读。
    /// 拼音读法末尾的单字母也读（`huoz` → 或者，不是 或 + 丢掉 z），两边覆盖同样多的字母才公平。
    /// `wodedatabase`：我的 + database 赢过 我的大塔巴瑟；`womenqubeijing`：我们去北京 赢过 我们去 + Beijing；
    /// `taida`：太大 赢过 他 + Ida；`huoz`：或者 赢过 和 + Oz。
    ///
    /// 两边比的是 `static_score - penalty`（静态语言模型 + 敲错代价），**不含个人 n-gram 与用户加分**：
    /// 头段只覆盖一部分音节，整段读法多出来的那几个音节会白拿一份按长度累积的个人加成。
    /// `woxiangxuexirust` 上实测头段拿 +6.9、整段拿 +21.2，个人模型越肥差距越大——
    /// 用户打得越多混输越打不出来。这是「哪种输入方式」的判断，本来也该由静态模型定，不归个人频次管。
    pub(super) fn mixed_beats_plain(&self, scope: &str, tail: &EnglishTail) -> bool {
        let convert = |text: &str, whole: bool| {
            let segmentations = parser::segment(text).ok()?;
            self.convert_sentence_with(&segmentations.first()?.patterns(), true, whole)
        };
        let (Some(head), Some(plain)) = (
            convert(&scope[..tail.head_len], false),
            convert(scope, true),
        ) else {
            return false;
        };
        if head.has_placeholder() {
            return false;
        }
        let comparable = |c: &Conversion| c.static_score - c.penalty;
        let mixed = comparable(&head) + tail.log_prob - ENGLISH_SWITCH_PENALTY;
        let plain_score = comparable(&plain);
        tracing::debug!(
            word = %tail.word,
            head = %head.text, head = comparable(&head),
            plain = %plain.text, plain = plain_score,
            tail_log_prob = tail.log_prob, mixed,
            wins = mixed > plain_score,
            "混输比分"
        );
        mixed > plain_score
    }

    /// 头段拼音转成的汉字加上英文尾段：候选的音节是头段的全拼音节加上敲的尾段字母（上屏按它们消耗拼音）。
    /// 头段有占位音节的不出。
    pub(super) fn mixed_sentence(
        &self,
        head: &Segmentation,
        tail: &EnglishTail,
        typos: bool,
    ) -> Option<Candidate> {
        let conversion = self.convert_sentence(&head.patterns(), typos)?;
        if conversion.has_placeholder() {
            return None;
        }
        let typed = &self.composition.scope()[tail.head_len..];
        let mut syllables = conversion.syllables;
        syllables.push(typed.to_owned());
        Some(Candidate {
            rime: None,
            text: format!("{}{}", conversion.text, tail.word),
            kind: CandidateKind::Sentence,
            syllables,
            reading: None,
            translation: None,
            aux_code: None,
        })
    }
}
