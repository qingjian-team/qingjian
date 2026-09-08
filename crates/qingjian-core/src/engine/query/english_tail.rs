use super::*;

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
        if lists.iter().any(|words| words.get(scope).is_some()) {
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
                log_prob: english_log_prob(words.frequency(tail)),
            })
        })
    }

    /// 整段也能读成拼音时两种读法比分：头段整句的得分加英文词的 log 概率、扣掉切到英文的代价，高过整段按拼音读的整句就按英文读。
    /// 拼音读法末尾的单字母也读（`huoz` → 或者，不是 或 + 丢掉 z），两边覆盖同样多的字母才公平。
    /// `wodedatabase`：我的 + database 赢过 我的大塔巴瑟；`womenqubeijing`：我们去北京 赢过 我们去 + Beijing；
    /// `taida`：太大 赢过 他 + Ida；`huoz`：或者 赢过 和 + Oz。
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
        head.score + tail.log_prob - ENGLISH_SWITCH_PENALTY > plain.score
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
            text: format!("{}{}", conversion.text, tail.word),
            kind: CandidateKind::Sentence,
            syllables,
            reading: None,
            translation: None,
        })
    }
}
