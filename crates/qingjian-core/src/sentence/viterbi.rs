//! 词图上的最优路径：bigram Viterbi + 束搜索。
//!
//! 状态只按前一个词分（束宽内），个人三元要的前二词取前驱节点的回指（它那条最优路径上的前一个词）：
//! 不扩状态，代价是三元上下文是近似的，个人数据量下够用。

use qingjian_dictionary::{Dictionary, Match, SyllablePattern};

use super::{
    ABBREVIATED_SPAN_CANDIDATES, BEAM_WIDTH, Context, Conversion, LanguageModel,
    MAX_WORD_SYLLABLES, MIN_PARTIAL_LETTERS, SPAN_CANDIDATES, SentenceWord, SpanCache, SpanWord,
    UserNgram, fallback_log_prob, transition_log_prob,
};
use crate::ranking::weight_bonus;

/// 词库里没有的孤立音节（罕见音节没有单字）按这个 log 概率兜底，让路径总能走通。
const UNKNOWN_LOG_PROB: f64 = -30.0;

/// 一条部分路径的末尾节点。
struct Node {
    /// 这个词从第几个音节开始。
    start: usize,

    /// 词。
    text: String,

    /// 词的音节。
    syllables: Vec<String>,

    /// 到此为止的累计得分。
    score: f64,

    /// 前驱在 `nodes[start]` 里的下标；`start == 0` 时无意义。
    back: usize,

    /// 是占位音节。
    placeholder: bool,

    /// 到此为止路径上模糊音 / 敲错变体的代价之和（已从 `score` 里扣掉，另记一份给调用方判断路径是不是原样）。
    penalty: f64,
}

/// 把音节序列转成最可能的词序列。`positions` 每个位置是若干写法（第一种是敲的，其余是模糊音 / 敲错变体），
/// `cost(位置, 命中的音节)` 是那个位置命中这种写法要扣的分（敲的原样 0），
/// `weight` 是用户选择次数，`personal` 是个人 n-gram（没有就传 `None`），`cache` 是格子候选的缓存
/// （调用方保证它与词库、`weight`、`personal`、`cost` 一致，这些一变就清）。
///
/// 简拼位置（`w x q`）按前缀取词：每个格子的候选会多得多，由语言模型在路径上分辨。
/// 全拼句子末尾的前缀太短时不算它（多半是没打完的音节）；前面已有简拼的句子里末尾单字母就是一个音节。
pub fn convert(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    model: &dyn LanguageModel,
    personal: Option<&UserNgram>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Option<Conversion> {
    convert_with(
        dictionaries,
        positions,
        false,
        model,
        personal,
        weight,
        cost,
        cache,
    )
}

/// 同 [`convert`]，但全拼句子末尾的单字母也当一个音节读（`huo z…` → 或者）：
/// 给「整段拼音读法」与别的读法比分用，比分要两边覆盖同样多的字母。
pub fn convert_whole(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    model: &dyn LanguageModel,
    personal: Option<&UserNgram>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Option<Conversion> {
    convert_with(
        dictionaries,
        positions,
        true,
        model,
        personal,
        weight,
        cost,
        cache,
    )
}

/// [`convert`] 与 [`convert_whole`] 的共同实现，`keep_partial` 选哪种。
#[allow(clippy::too_many_arguments)]
pub fn convert_with(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    keep_partial: bool,
    model: &dyn LanguageModel,
    personal: Option<&UserNgram>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Option<Conversion> {
    let (last, head) = positions.split_last()?;
    let last = *last.first()?;
    let abbreviated_head = head.iter().any(|p| p.first().is_none_or(|t| !t.complete));
    let positions = if keep_partial
        || last.complete
        || abbreviated_head
        || last.text.len() >= MIN_PARTIAL_LETTERS
    {
        positions
    } else {
        head
    };
    let n = positions.len();
    if n == 0 {
        return None;
    }
    let total: f64 = dictionaries
        .iter()
        .map(|d| d.total_frequency() as f64)
        .sum::<f64>()
        .max(1.0);
    let log_total = total.ln();

    // nodes[i]：覆盖前 i 个音节、以某个词结尾的部分路径；nodes[0] 是虚拟起点
    let mut nodes: Vec<Vec<Node>> = (0..=n).map(|_| Vec::new()).collect();
    nodes[0].push(Node {
        start: 0,
        text: String::new(),
        syllables: Vec::new(),
        score: 0.0,
        back: 0,
        placeholder: false,
        penalty: 0.0,
    });
    for start in 0..n {
        prune(&mut nodes[start]);
        if nodes[start].is_empty() {
            continue;
        }
        let mut any = false;
        for end in start + 1..=n.min(start + MAX_WORD_SYLLABLES) {
            let span = &positions[start..end];
            let hits = cache.get_or_insert_with(SpanCache::key(span), || {
                span_candidates(dictionaries, span, start, personal, &weight, &cost)
            });
            if hits.is_empty() {
                continue;
            }
            any = true;
            for hit in hits.iter() {
                let bonus = weight_bonus(weight(&hit.text));
                let fallback = fallback_log_prob(hit.frequency, log_total);
                let (score, back) =
                    best_predecessor(&nodes, start, &hit.text, model, personal, fallback);
                let penalty = nodes[start][back].penalty + hit.penalty;
                nodes[end].push(Node {
                    start,
                    text: hit.text.clone(),
                    syllables: hit.syllables.clone(),
                    score: score + bonus - hit.penalty,
                    back,
                    placeholder: false,
                    penalty,
                });
            }
        }
        // 这个音节连单字都查不到：用音节本身占位，别让整句断掉
        if !any {
            let text = positions[start][0].text;
            let (score, back) =
                best_predecessor(&nodes, start, text, &NoModel, None, UNKNOWN_LOG_PROB);
            let penalty = nodes[start][back].penalty;
            nodes[start + 1].push(Node {
                start,
                text: text.to_owned(),
                syllables: vec![text.to_owned()],
                score,
                back,
                placeholder: true,
                penalty,
            });
        }
    }
    prune(&mut nodes[n]);
    let mut index = 0;
    let mut position = n;
    if nodes[n].is_empty() {
        return None;
    }
    let score = nodes[n][0].score;
    let penalty = nodes[n][0].penalty;
    // 回溯
    let mut words: Vec<SentenceWord> = Vec::new();
    while position > 0 {
        let node = &nodes[position][index];
        words.push(SentenceWord {
            text: node.text.clone(),
            syllables: node.syllables.clone(),
            placeholder: node.placeholder,
        });
        position = node.start;
        index = node.back;
    }
    words.reverse();
    let mut text = String::new();
    let mut syllables = Vec::new();
    for word in &words {
        text.push_str(&word.text);
        syllables.extend(word.syllables.iter().cloned());
    }
    Some(Conversion {
        text,
        syllables,
        words,
        score,
        penalty,
    })
}

/// 占位音节不问语言模型。
struct NoModel;

impl LanguageModel for NoModel {
    fn log_prob(&self, _previous: Option<&str>, _word: &str) -> Option<f64> {
        None
    }
}

/// 一个格子里的候选词：所有词库的精确命中，按词频（加用户选择次数与个人出现次数，替代写法命中的按代价打折）取前几个。
/// 个人次数只在这里保证用户常用的同音词进得了格子，不进路径打分（那是 n-gram 的事）；
/// 打折让敲错变体命中的词只在原样命中不够多时才进格子，而常用词（关系）即使打折也留得住。
/// 格子里有简拼位置时命中的是一大片不同读音的词，多留一些让语言模型去挑。
fn span_candidates(
    dictionaries: &[&Dictionary],
    span: &[Vec<SyllablePattern<'_>>],
    start: usize,
    personal: Option<&UserNgram>,
    weight: &impl Fn(&str) -> u32,
    cost: &impl Fn(usize, &str) -> f64,
) -> Vec<SpanWord> {
    let alternatives = span.iter().any(|p| p.len() > 1);
    let penalty_of = |m: &Match<'_>| {
        if !alternatives {
            return 0.0;
        }
        m.syllables()
            .enumerate()
            .map(|(index, syllable)| cost(start + index, syllable))
            .sum::<f64>()
    };
    // 得分先算好再排：单字母简拼的格子能命中几千条，比较器里每次查两张表会让排序占掉十几毫秒
    let mut scored: Vec<(f64, f64, Match<'_>)> = dictionaries
        .iter()
        .flat_map(|d| d.lookup_exact_alt(span))
        .map(|m| {
            let seen = weight(m.text) + personal.map_or(0, |p| p.count(m.text));
            let penalty = penalty_of(&m);
            let score = f64::from(m.frequency) * (1.0 + f64::from(seen)) * (-penalty).exp();
            (score, penalty, m)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.dedup_by(|a, b| a.2.text == b.2.text);
    let abbreviated = span.iter().any(|p| p.iter().any(|t| !t.complete));
    scored.truncate(if abbreviated {
        ABBREVIATED_SPAN_CANDIDATES
    } else {
        SPAN_CANDIDATES
    });
    scored
        .into_iter()
        .map(|(_, penalty, hit)| SpanWord {
            text: hit.text.to_owned(),
            syllables: hit.syllables().map(str::to_owned).collect(),
            frequency: hit.frequency,
            penalty,
        })
        .collect()
}

/// 在 `nodes[start]` 的前驱里挑让 `word` 得分最高的那条，返回 (累计得分, 前驱下标)。
/// 转移概率先问静态模型（不认识就用词库兜底值），再与个人 n-gram 插值；前二词是前驱自己的前驱（回指）。
fn best_predecessor(
    nodes: &[Vec<Node>],
    start: usize,
    word: &str,
    model: &dyn LanguageModel,
    personal: Option<&UserNgram>,
    fallback: f64,
) -> (f64, usize) {
    let mut best = (f64::NEG_INFINITY, 0);
    for (index, previous) in nodes[start].iter().enumerate() {
        let context = if start == 0 {
            Context::START
        } else {
            Context {
                previous: Some(previous.text.as_str()),
                earlier: (previous.start > 0)
                    .then(|| nodes[previous.start][previous.back].text.as_str()),
            }
        };
        let score = previous.score + transition_log_prob(model, personal, context, word, fallback);
        if score > best.0 {
            best = (score, index);
        }
    }
    best
}

/// 按得分降序只留束宽条。
fn prune(nodes: &mut Vec<Node>) {
    nodes.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    nodes.truncate(BEAM_WIDTH);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sentence::NoLanguageModel;

    const SAMPLE: &str = "我\two\t900000\n想\txiang\t500000\n去\tqu\t400000\n吃\tchi\t300000\n饭\tfan\t200000\n\
        吃饭\tchi fan\t100000\n我想\two xiang\t600000\n翔\txiang\t3000\n区\tqu\t100000\n卧\two\t2000\n\
        开发\tkai fa\t9000\n开\tkai\t20000\n发\tfa\t30000\n开放\tkai fang\t20000\n";

    fn complete<'a>(syllables: &[&'a str]) -> Vec<Vec<SyllablePattern<'a>>> {
        syllables
            .iter()
            .map(|s| vec![SyllablePattern::complete(s)])
            .collect()
    }

    fn unigram(
        dictionary: &Dictionary,
        patterns: &[Vec<SyllablePattern<'_>>],
    ) -> Option<Conversion> {
        convert(
            &[dictionary],
            patterns,
            &NoLanguageModel,
            None,
            |_| 0,
            |_, _| 0.0,
            &mut SpanCache::default(),
        )
    }

    #[test]
    fn picks_common_words_over_characters() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = complete(&["wo", "xiang", "qu", "chi", "fan"]);
        let conversion = unigram(&dictionary, &patterns).unwrap();
        assert_eq!(conversion.text, "我想去吃饭");
        assert_eq!(conversion.word_count(), 3); // 我想 / 去 / 吃饭
        assert_eq!(conversion.words[0].text, "我想");
        assert_eq!(conversion.words[0].syllables, ["wo", "xiang"]);
        assert_eq!(conversion.syllables.len(), 5);
    }

    #[test]
    fn user_weight_lifts_near_ties_but_is_capped() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        // wo qu 没有整词：卧 / 我 词频差 450 倍（log 差 6.1），加分封顶后选多少次都翻不过来
        let patterns = complete(&["wo", "qu"]);
        let lifted = |count: u32| {
            convert(
                &[&dictionary],
                &patterns,
                &NoLanguageModel,
                None,
                |t| if t == "卧" { count } else { 0 },
                |_, _| 0.0,
                &mut SpanCache::default(),
            )
            .unwrap()
            .text
        };
        assert_eq!(lifted(20), "我去");
        assert_eq!(lifted(500), "我去");
        // 去 / 区 差 4 倍（log 差 1.4）：选过十几次就翻过来
        let patterns = complete(&["wo", "qu"]);
        let lifted = |count: u32| {
            convert(
                &[&dictionary],
                &patterns,
                &NoLanguageModel,
                None,
                |t| if t == "区" { count } else { 0 },
                |_, _| 0.0,
                &mut SpanCache::default(),
            )
            .unwrap()
            .text
        };
        assert_eq!(lifted(2), "我去");
        assert_eq!(lifted(20), "我区");
    }

    #[test]
    fn partial_last_syllable_of_a_full_pinyin_sentence() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let mut patterns = complete(&["wo", "xiang"]);
        patterns.push(vec![SyllablePattern::prefix("ka")]);
        assert_eq!(unigram(&dictionary, &patterns).unwrap().text, "我想开");
        // 全拼句子末尾的单字母多半是没打完的音节，不参与
        let mut patterns = complete(&["wo", "xiang"]);
        patterns.push(vec![SyllablePattern::prefix("k")]);
        assert_eq!(unigram(&dictionary, &patterns).unwrap().text, "我想");
    }

    fn abbreviated<'a>(letters: &[&'a str]) -> Vec<Vec<SyllablePattern<'a>>> {
        letters
            .iter()
            .map(|s| vec![SyllablePattern::prefix(s)])
            .collect()
    }

    #[test]
    fn abbreviated_sentences_convert_by_prefix() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        // 全简拼：每个格子按前缀取词，末尾单字母也是一个音节
        let conversion = unigram(&dictionary, &abbreviated(&["w", "x", "q"])).unwrap();
        assert_eq!(conversion.text, "我想去");
        assert_eq!(conversion.syllables, ["wo", "xiang", "qu"]);
        assert_eq!(conversion.words[0].text, "我想");
        // 简拼与全拼混用
        let patterns = [
            vec![SyllablePattern::prefix("w")],
            vec![SyllablePattern::complete("xiang")],
            vec![SyllablePattern::prefix("q")],
            vec![SyllablePattern::prefix("c")],
            vec![SyllablePattern::prefix("f")],
        ];
        assert_eq!(unigram(&dictionary, &patterns).unwrap().text, "我想去吃饭");
        // 简拼位置按前缀取词：`f` 既是 fa 也是 fang，词频高的 开放 胜出
        let conversion = unigram(&dictionary, &abbreviated(&["k", "f"])).unwrap();
        assert_eq!(conversion.text, "开放");
        assert!(!conversion.has_placeholder());
    }

    /// 只认 `我 → 翔` 的假模型下，简拼 `w x` 也该听语言模型的。
    #[test]
    fn language_model_decides_abbreviated_homophones() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = abbreviated(&["w", "x"]);
        let conversion = convert(
            &[&dictionary],
            &patterns,
            &XiangModel,
            None,
            |_| 0,
            |_, _| 0.0,
            &mut SpanCache::default(),
        )
        .unwrap();
        assert_eq!(conversion.text, "我翔");
    }

    #[test]
    fn unknown_syllables_are_kept_as_pinyin() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = complete(&["wo", "zhuang", "qu"]);
        let conversion = unigram(&dictionary, &patterns).unwrap();
        assert_eq!(conversion.text, "我zhuang去");
        assert!(conversion.has_placeholder());
        assert!(conversion.words.iter().filter(|w| w.placeholder).count() == 1);
    }

    /// 只认 `我 → 翔` 的假模型：bigram 应该压过一元词频。
    struct XiangModel;

    impl LanguageModel for XiangModel {
        fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
            match (previous, word) {
                (Some("我"), "翔") => Some(-2.0),
                (Some("我"), "想") => Some(-8.0),
                (None, "我") => Some(-1.0),
                _ => None,
            }
        }
    }

    #[test]
    fn language_model_decides_between_homophones() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = complete(&["wo", "xiang"]);
        let conversion = convert(
            &[&dictionary],
            &patterns,
            &XiangModel,
            None,
            |_| 0,
            |_, _| 0.0,
            &mut SpanCache::default(),
        )
        .unwrap();
        assert_eq!(conversion.text, "我翔");
        assert_eq!(conversion.word_count(), 2);
    }

    #[test]
    fn personal_ngram_overrides_static_model_after_two_selections() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = complete(&["wo", "xiang"]);
        let mut personal = UserNgram::default();
        let text = |personal: &UserNgram| {
            convert(
                &[&dictionary],
                &patterns,
                &XiangModel,
                Some(personal),
                |_| 0,
                |_, _| 0.0,
                &mut SpanCache::default(),
            )
            .unwrap()
            .text
        };
        assert_eq!(text(&personal), "我翔");
        // 静态模型给 翔 的是很强的 bigram（P ≈ 0.14）：用户选过一次 我 → 想 翻不过（防误选），两次就翻。
        // 静态证据越弱（P 越小），个人偏好翻过来得越早。
        personal.record(Context::START, "我");
        personal.record(Context::after("我"), "想");
        assert_eq!(text(&personal), "我翔");
        personal.record(Context::START, "我");
        personal.record(Context::after("我"), "想");
        assert_eq!(text(&personal), "我想");
    }

    /// 三元上下文来自前驱的回指：「我想」后面的 去 / 区 由用户在「我想」后选过什么决定，
    /// 而「卧想」后面（回指不同）拿不到这条三元。
    #[test]
    fn trigram_context_comes_from_the_predecessor_chain() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let patterns = complete(&["wo", "xiang", "qu"]);
        let convert_with = |personal: &UserNgram| {
            convert(
                &[&dictionary],
                &patterns,
                &NoLanguageModel,
                Some(personal),
                |_| 0,
                |_, _| 0.0,
                &mut SpanCache::default(),
            )
            .unwrap()
        };
        let mut personal = UserNgram::default();
        // 一元下 我想去（我想 是一个词）
        assert_eq!(convert_with(&personal).text, "我想去");
        // 用户在 我 → 想 之后选过 区：三元 (我, 想) → 区 把 区 抬过 去，路径改走 我 / 想 / 区
        for _ in 0..4 {
            personal.record(Context::START, "我");
            personal.record(Context::after("我"), "想");
            personal.record(Context::after_two("我", "想"), "区");
        }
        let conversion = convert_with(&personal);
        assert_eq!(conversion.text, "我想区");
        assert_eq!(conversion.word_count(), 3);
    }

    #[test]
    fn cached_spans_give_the_same_sentence_and_only_new_spans_are_computed() {
        let dictionary = Dictionary::parse(SAMPLE).unwrap();
        let mut cache = SpanCache::default();
        let run = |cache: &mut SpanCache, syllables: &[&str]| {
            let patterns = complete(syllables);
            convert(
                &[&dictionary],
                &patterns,
                &NoLanguageModel,
                None,
                |_| 0,
                |_, _| 0.0,
                cache,
            )
            .unwrap()
            .text
        };
        let fresh = run(&mut cache, &["wo", "xiang", "qu", "chi"]);
        let before = cache.len();
        // 多敲一个音节：只新增以它结尾的格子
        let extended = run(&mut cache, &["wo", "xiang", "qu", "chi", "fan"]);
        assert_eq!(extended, "我想去吃饭");
        assert!(cache.len() > before);
        assert!(cache.len() - before <= MAX_WORD_SYLLABLES);
        // 再算一遍全部命中缓存，结果一致
        let again = cache.len();
        assert_eq!(run(&mut cache, &["wo", "xiang", "qu", "chi"]), fresh);
        assert_eq!(cache.len(), again);
    }

    /// 敲错变体是带代价的边：`gan xi` 在 `gan` 位多一种写法 `guan`（代价 4.5），原样凑不出像样的句子时 关系 胜出，
    /// 原样本身说得通（`gan xie` 感谢）时代价让它输。
    #[test]
    fn typo_alternatives_are_penalized_edges() {
        let dictionary = Dictionary::parse(
            "关系\tguan xi\t500000\n干\tgan\t20000\n洗\txi\t10000\n感谢\tgan xie\t300000\n关\tguan\t30000\n谢\txie\t5000\n",
        )
        .unwrap();
        let cost = |index: usize, syllable: &str| {
            if index == 0 && syllable == "guan" {
                4.5
            } else {
                0.0
            }
        };
        let run = |positions: Vec<Vec<SyllablePattern<'_>>>| {
            convert(
                &[&dictionary],
                &positions,
                &NoLanguageModel,
                None,
                |_| 0,
                cost,
                &mut SpanCache::default(),
            )
            .unwrap()
        };
        let with_typo = |second: &'static str| {
            vec![
                vec![
                    SyllablePattern::complete("gan"),
                    SyllablePattern::complete("guan"),
                ],
                vec![SyllablePattern::complete(second)],
            ]
        };
        let conversion = run(with_typo("xi"));
        assert_eq!(conversion.text, "关系");
        assert_eq!(conversion.words[0].syllables, ["guan", "xi"]);
        // 原样 干洗 两个单字的得分远低于 关系 − 4.5：噪声信道选纠正，路径带着代价
        assert_eq!(conversion.penalty, 4.5);
        let conversion = run(with_typo("xie"));
        assert_eq!(conversion.text, "感谢");
        assert_eq!(conversion.penalty, 0.0);
    }
}
