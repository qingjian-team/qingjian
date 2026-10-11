//! 整句词图搜索的回归测试。

use super::*;
use crate::sentence::{NoLanguageModel, UserNgram};

const SAMPLE: &str = "我\two\t900000\n想\txiang\t500000\n去\tqu\t400000\n吃\tchi\t300000\n饭\tfan\t200000\n\
    吃饭\tchi fan\t100000\n我想\two xiang\t600000\n翔\txiang\t3000\n区\tqu\t100000\n卧\two\t2000\n\
    开发\tkai fa\t9000\n开\tkai\t20000\n发\tfa\t30000\n开放\tkai fang\t20000\n";

#[test]
fn k_best_paths_preserve_alternative_prefixes_after_a_shared_suffix() {
    let dictionary = Dictionary::parse(SAMPLE).unwrap();
    let paths = convert_paths(
        &[&dictionary],
        &complete(&["wo", "xiang", "qu", "chi", "fan"]),
        false,
        8,
        &NoLanguageModel,
        Personal::NONE,
        |_| 0,
        |_, _| 0.0,
        &mut SpanCache::default(),
    );
    assert_eq!(paths[0].text, "我想去吃饭");
    assert!(paths.iter().any(|path| path.text == "我翔去吃饭"));
    assert!(
        paths
            .iter()
            .all(|path| path.syllables.concat() == "woxiangquchifan")
    );
    let texts: HashSet<_> = paths.iter().map(|path| path.text.as_str()).collect();
    assert_eq!(texts.len(), paths.len());
}

fn complete<'a>(syllables: &[&'a str]) -> Vec<Vec<SyllablePattern<'a>>> {
    syllables
        .iter()
        .map(|s| vec![SyllablePattern::complete(s)])
        .collect()
}

fn unigram(dictionary: &Dictionary, patterns: &[Vec<SyllablePattern<'_>>]) -> Option<Conversion> {
    convert(
        &[dictionary],
        patterns,
        &NoLanguageModel,
        Personal::NONE,
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
            Personal::NONE,
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
            Personal::NONE,
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
        Personal::NONE,
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
        Personal::NONE,
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
            Personal::new(Some(personal)),
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
            Personal::new(Some(personal)),
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
            Personal::NONE,
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
            Personal::NONE,
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
