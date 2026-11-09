//! 句末英文词并入整句（`kaifarust` → 开发rust）：与拼音读法比分、个人数据不左右比分、原样上屏不学成英文词。

use super::*;

/// 句末的英文词：`kaifarust` → 开发rust 排第一，拼音行是 `kai'fa'rust`，上屏吃掉整段并把 rust 记进个人英文词表。
#[test]
fn english_word_at_the_end_of_pinyin_joins_the_sentence() {
    let words =
        WordList::parse("rust\trust\t3740\nID\tid\t4610\nto\tto\t7430\nfan\tfan\t4500\n").unwrap();
    let mut engine = engine().with_english(words);

    engine.set_input("kaifarust");
    let query = engine.query().unwrap();
    let first = query.candidates.items[0].clone();
    assert_eq!(first.text, "开发rust");
    assert_eq!(first.kind, CandidateKind::Sentence);
    assert_eq!(first.syllables, ["kai", "fa", "rust"]);
    assert_eq!(query.tail, "rust");
    assert_eq!(query.marked_text(), "kai'fa'rust");
    // 头段的词照常出（逐词上屏也行）
    assert!(query.candidates.items.iter().any(|c| c.text == "开发"));
    assert_eq!(engine.commit(&first), "开发rust");
    assert!(engine.composition().is_empty());

    // 两个字母的尾段只认缩写词：ID 行，to 不行（`kaifato` 按拼音读）
    engine.set_input("kaifaid");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发ID");
    engine.set_input("kaifato");
    let all: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert!(!all.iter().any(|t| t.ends_with("to")), "{all:?}");

    // 尾段本身是合法拼音又不到四个字母（`fan`）：就是拼音，开饭 照旧
    engine.set_input("kaifan");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开饭");
}

/// 尾段也是合法拼音时两种读法比分：`wodedatabase` 英文赢（拼音读法是四个散字），拼音读法排第二；
/// `womenqubeijing` 拼音赢（北京 是常用词），输了的英文读法不出。
#[test]
fn pinyin_like_english_tail_competes_with_the_plain_reading() {
    const DICT: &str = "我\two\t900000\n的\tde\t800000\n我的\two de\t500000\n大\tda\t50000\n塔\tta\t3000\n巴\tba\t3000\n瑟\tse\t500\n\
        我们\two men\t400000\n去\tqu\t300000\n北京\tbei jing\t200000\n北\tbei\t20000\n京\tjing\t10000\n";
    let words = WordList::parse("database\tdatabase\t4310\nBeijing\tbeijing\t4500\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap()).with_english(words);

    engine.set_input("wodedatabase");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "我的database");
    // 拼音读法第二（末尾 se 被敲错边读成 的，散字路径本来就随便）
    assert!(query.candidates.items[1].text.starts_with("我的大塔巴"));
    assert_eq!(query.candidates.items[1].kind, CandidateKind::Sentence);
    assert_eq!(query.marked_text(), "wo'de'database");

    engine.set_input("womenqubeijing");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "我们去北京");
    assert!(
        !query
            .candidates
            .items
            .iter()
            .any(|c| c.text.ends_with("Beijing"))
    );
    assert_eq!(query.marked_text(), "wo'men'qu'bei'jing");

    // 上屏英文赢了的整句：吃掉整段，英文词是最后一个词
    engine.set_input("wodedatabase");
    let mixed = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(mixed.syllables, ["wo", "de", "database"]);
    assert_eq!(engine.commit(&mixed), "我的database");
    assert!(engine.composition().is_empty());
}

/// 个人 n-gram 再肥也不该左右「混输还是纯拼音」：头段只覆盖一部分音节，整段读法多出来的那几个音节
/// 会白拿一份按长度累积的个人加成，用户打得越多混输越打不出来。比分只看静态模型加敲错代价。
#[test]
fn personal_ngram_does_not_decide_between_mixed_and_plain_readings() {
    const DICT: &str = "我\two\t900000\n的\tde\t800000\n我的\two de\t500000\n大\tda\t50000\n塔\tta\t3000\n巴\tba\t3000\n瑟\tse\t500\n";

    struct NgramLearner(crate::sentence::UserNgram);

    impl Learner for NgramLearner {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn user_ngram(&self) -> Option<&crate::sentence::UserNgram> {
            Some(&self.0)
        }
    }

    let mut ngram = crate::sentence::UserNgram::default();
    // 把纯拼音那条读法的每一步都记成用户的老习惯
    for (previous, word) in [
        (None, "我的"),
        (Some("我的"), "大"),
        (Some("大"), "塔"),
        (Some("塔"), "巴"),
        (Some("巴"), "瑟"),
    ] {
        ngram.record_times(
            crate::sentence::Context {
                previous,
                earlier: None,
            },
            word,
            5000,
        );
    }

    let words = WordList::parse("database\tdatabase\t4310\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap())
        .with_english(words)
        .with_learner(Box::new(NgramLearner(ngram)));

    engine.set_input("wodedatabase");
    let first = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(first.text, "我的database", "个人 n-gram 不该把混输压下去");
}

/// 英文词频只问随包表：个人英文表存的是使用次数，当 Zipf×1000 读会一律塌到兜底值
/// （`English` 用过两次 → Zipf 0.002 → 兜底 3.0，真实 5.19），等于用过一次反而更难打出来。
#[test]
fn personal_english_usage_count_does_not_replace_the_shipped_frequency() {
    const DICT: &str = "我\two\t900000\n的\tde\t800000\n我的\two de\t500000\n大\tda\t50000\n塔\tta\t3000\n巴\tba\t3000\n瑟\tse\t500\n";

    struct PersonalEnglish(WordList);

    impl Learner for PersonalEnglish {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn user_english(&self) -> Option<&WordList> {
            Some(&self.0)
        }
    }

    // 个人表里是「用过 1 次」，随包表里是 Zipf 5.19。个人表排在前面，按 Zipf×1000 读就是 0.001，
    // 会被夹到兜底的 Zipf 3，白丢 5 nat
    let personal = WordList::parse("database\tdatabase\t1\n").unwrap();
    let shipped = WordList::parse("database\tdatabase\t5190\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap())
        .with_english(shipped)
        .with_learner(Box::new(PersonalEnglish(personal)));

    let tail = engine
        .split_english_tail("wodedatabase")
        .expect("末尾是英文词");
    let expected = (5.19 - 9.0) * std::f64::consts::LN_10;
    assert!(
        (tail.log_prob - expected).abs() < 1e-9,
        "log_prob {} 不是随包表的 Zipf 5.19（{expected}）",
        tail.log_prob,
    );

    engine.set_input("wodedatabase");
    let first = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(first.text, "我的database");
}

/// 打不出来时原样上屏的整串（`kaifarust`）不学成英文词，学进去了也不许堵掉混输：
/// 它能读成「拼音头 + 英文尾」，把整串当英文词会让 `split_english_tail` 一开头就退出，
/// 于是越打不出来学得越多、越学越打不出来。
#[test]
fn a_raw_committed_mixed_input_does_not_block_the_english_tail() {
    #[derive(Default)]
    struct EnglishLearner {
        words: Vec<String>,
        list: Option<WordList>,
    }

    impl Learner for EnglishLearner {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn learn_english(&mut self, word: &str) {
            self.words.push(word.to_owned());
            let tsv: String = self
                .words
                .iter()
                .map(|w| format!("{w}\t{w}\t1\n"))
                .collect();
            self.list = WordList::parse(&tsv).ok();
        }

        fn user_english(&self) -> Option<&WordList> {
            self.list.as_ref()
        }
    }

    let words = WordList::parse("rust\trust\t3740\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_english(words)
        .with_learner(Box::new(EnglishLearner::default()));

    // 原样上屏整串：不该被记成英文词
    engine.set_input("kaifarust");
    assert_eq!(engine.take_raw(), "kaifarust");
    engine.set_input("kaifarust");
    let query = engine.query().unwrap();
    assert!(
        !query
            .candidates
            .items
            .iter()
            .any(|c| c.kind == CandidateKind::English && c.text == "kaifarust"),
        "失败的混输不该进个人英文词表：{:?}",
        query.candidates.items.first(),
    );
    assert_eq!(query.candidates.items[0].text, "开发rust");

    // 就算个人表里已经有这条（老用户攒下的），混输那条路也要照走
    let polluted = WordList::parse("kaifarust\tkaifarust\t1\n").unwrap();
    struct Polluted(WordList);
    impl Learner for Polluted {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn user_english(&self) -> Option<&WordList> {
            Some(&self.0)
        }
    }
    let words = WordList::parse("rust\trust\t3740\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_english(words)
        .with_learner(Box::new(Polluted(polluted)));
    engine.set_input("kaifarust");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发rust");
}

/// 英文夹在中间的整串（`kaifarustdaima`）原样上屏也不学成英文词。
///
/// 上一个测试那条守卫靠 `split_english_tail`，只看得见句尾的英文；英文在中间时它返回 `None`，
/// 于是整串照学不误——用户表里 `yongdockerbushuhenfangbian` 就是这么来的，学完还会顶到候选第一位。
/// 这里靠的是长度上限（[`MAX_ENGLISH_WORD_LETTERS`]）。
#[test]
fn a_long_raw_commit_with_english_in_the_middle_is_not_learned() {
    struct Recorder(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

    impl Learner for Recorder {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn learn_english(&mut self, word: &str) {
            self.0.lock().unwrap().push(word.to_owned());
        }
    }

    let learned = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let words = WordList::parse("rust\trust\t3740\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_english(words)
        .with_learner(Box::new(Recorder(learned.clone())));

    // 英文夹在中间：`split_english_tail` 看不见 rust，只有长度上限拦得住
    engine.set_input("yongdockerbushuhenfangbian");
    assert_eq!(engine.take_raw(), "yongdockerbushuhenfangbian");
    // 短的英文词照学不误
    engine.set_input("kubectl");
    assert_eq!(engine.take_raw(), "kubectl");

    assert_eq!(
        *learned.lock().unwrap(),
        vec!["kubectl".to_owned()],
        "只有真正像一个词的才该进个人英文词表"
    );
}

/// 末位落单声母正好是英文词的最后一个字母时（`henhaobook`），英文词内部换位凑出的残尾拼音
/// （`book` → `…o/bo/k…`）不得被当成拼写纠正：落单声母进词图后原样多读一个位置，残尾对残尾
/// 就能靠字频赢过原样，把「拼音头 + 英文词」整条挤掉（很好book 退化成 很好哦伯克）。见 #619。
#[test]
fn an_unfinished_correction_does_not_push_the_english_tail_away() {
    let dictionary = Dictionary::parse(
        "很好\then hao\t900000\n很\then\t800000\n好\thao\t800000\n\
         波\tbo\t5000\n哦\to\t100\n口\tkou\t5000\n空\tkong\t5000\n克\tke\t5000\n",
    )
    .unwrap();
    let words = WordList::parse("book\tbook\t5000\n").unwrap();
    let mut engine = Engine::new(dictionary).with_english(words);
    engine.set_input("henhaobook");
    let query = engine.query().unwrap();
    assert!(query.correction.is_none(), "残尾换位不该触发拼写纠正");
    assert_eq!(query.candidates.items[0].text, "很好book");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Sentence);
}

/// 英文尾段与整段拼音比分时，拼音读法按整句挑的切分来算：切分器打平时贪心取的 `zheng cen an ti` 读出 政策安提，
/// 拿它去比会让 解决政策anti 赢、正确的 政策难题 连候选都没有。
#[test]
fn the_pinyin_reading_competes_with_its_best_segmentation() {
    const DICT: &str = concat!(
        "政策\tzheng ce\t50000\n难题\tnan ti\t20000\n岑\tcen\t50\n安\tan\t30000\n",
        "提\tti\t30000\n政\tzheng\t3000\n"
    );
    let words = WordList::parse("anti\tanti\t6000\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap()).with_english(words);
    engine.set_input("zhengcenanti");
    let all = texts_of(&engine);
    assert_eq!(all[0], "政策难题", "{all:?}");
}
