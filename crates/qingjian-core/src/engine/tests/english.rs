//! 英文模式与中英混输。

use super::*;

#[test]
fn english_word_ranks_first_when_input_is_unlikely_pinyin() {
    // 词频是 Zipf×1000，与产品 english.tsv 同一尺度
    let words = WordList::parse(
        "hello\thello\t4720\nchina\tchina\t5100\nGitHub\tgithub\t3180\nkey\tkey\t5120\n",
    )
    .unwrap();
    let mut engine = engine().with_english(words);

    engine.set_input("hello"); // he l… l… o：中间有声母缩写
    let all: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert_eq!(all[0], "hello");

    engine.set_input("github"); // gi 不是音节 → 切不动
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "GitHub");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::English);
    let word = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&word), "GitHub");
    assert!(engine.composition().is_empty());

    // china 是干净的 chi na，中文候选（词库里没有就只有英文）排前；这里词库没有 chi na，英文仍在第一位
    engine.set_input("china");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "china");
}

#[test]
fn usage_meter_counts_hanzi_words_and_english_words_per_commit() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let mut engine = engine().with_usage_meter(Box::new(MemoryMeter(recorded.clone())));
    engine.set_input("kaifa");
    let kaifa = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(kaifa.text, "开发");
    engine.commit(&kaifa);
    // 拼音回车不算英文词，像英文的字母串才算
    engine.set_input("hao");
    engine.take_raw();
    engine.set_input("gist");
    engine.take_raw();
    let recorded = recorded.lock().unwrap();
    assert_eq!(
        recorded[0],
        Usage {
            hanzi: 2,
            words: 1,
            english_words: 0,
            commits: 1
        }
    );
    assert_eq!(recorded[1].english_words, 0);
    assert_eq!(recorded[1].commits, 1);
    assert_eq!(recorded[2].english_words, 1);
    assert_eq!(recorded.len(), 3);
}

#[test]
fn english_word_yields_to_a_chinese_word_the_user_keeps_choosing() {
    let dictionary = Dictionary::parse("可以\tke yi\t9000\n客运\tke yun\t100\n").unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(WordList::parse("key\tkey\t5120\n").unwrap())
        .with_learner(Box::new(CountingLearner(HashMap::new())));
    let first_two = |engine: &Engine| {
        let all = texts_of(engine);
        (all[0].clone(), all[1].clone())
    };
    let pick = |engine: &mut Engine, text: &str| {
        engine.set_input("key");
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == text)
            .unwrap();
        engine.commit(&candidate);
    };
    // 开了中文优先：ke'y 再不像话，中文词也在前、英文第二
    engine.set_chinese_first(true);
    engine.set_input("key");
    assert_eq!(first_two(&engine), ("可以".into(), "key".into()));
    // 缺省关：末尾落单一个字母、拼音不像话，英文词在前
    engine.set_chinese_first(false);
    engine.set_input("key");
    assert_eq!(first_two(&engine), ("key".into(), "可以".into()));
    // 这段字母下选过一次 可以：中文在前，英文退到第二
    pick(&mut engine, "可以");
    engine.set_input("key");
    assert_eq!(first_two(&engine), ("可以".into(), "key".into()));
    // 之后选英文词的次数反超：英文回到第一
    pick(&mut engine, "key");
    pick(&mut engine, "key");
    engine.set_input("key");
    assert_eq!(first_two(&engine), ("key".into(), "可以".into()));
}

#[test]
fn short_all_caps_acronym_yields_to_chinese() {
    // `mp` 整段只有两个字母、英文写法又是全大写缩写（MP）：几乎总是在打 门票，让中文先，
    // 英文词仍在候选里只是退到后面。超过两个字母的正文英文（hello / cargo）不受这条影响——
    // 曾经试过按词频一刀切，冻结日志回放实测英文首选从 82.5% 掉到 50.9%，cargo / rust 全被整句挤掉。
    let dictionary = Dictionary::parse("门票\tmen piao\t5000\n买票\tmai piao\t3000\n").unwrap();
    let words = WordList::parse("MP\tmp\t4290\nhello\thello\t4720\ncargo\tcargo\t5000\n").unwrap();
    let mut engine = Engine::new(dictionary).with_english(words);

    engine.set_input("mp");
    let all = texts_of(&engine);
    assert_eq!(all[0], "门票");
    assert!(
        all.iter().any(|t| t == "MP"),
        "英文词仍在候选里，只是让到后面"
    );
    assert!(all.iter().position(|t| t == "MP").unwrap() > 0);

    // 三个字母以上的正文英文：拼音不像话时照旧排第一
    engine.set_input("hello");
    assert_eq!(texts_of(&engine)[0], "hello");
    engine.set_input("cargo");
    assert_eq!(texts_of(&engine)[0], "cargo");
}

#[test]
fn learned_english_word_keeps_first_place_over_the_two_letter_acronym_rule() {
    // `ok` / `pc` / `ll` 同样满足「两个字母的全大写缩写」，一刀切会把它们一起翻成中文；
    // 而用户**选过**的英文词要照旧排第一 —— 选过 OK，下次敲 `ok` 就该还是 OK 在前。
    let dictionary = Dictionary::parse("哦\to\t5000\n门票\tmen piao\t5000\n").unwrap();
    let words = WordList::parse("OK\tok\t5140\nMP\tmp\t4290\n").unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(words)
        .with_learner(Box::new(CountingLearner(HashMap::new())));
    let pick = |engine: &mut Engine, input: &str, text: &str| {
        engine.set_input(input);
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == text)
            .unwrap();
        engine.commit(&candidate);
    };

    // 没选过时规则照旧生效：两个字母的全大写缩写让中文先。
    engine.set_input("mp");
    assert_eq!(texts_of(&engine)[0], "门票");
    engine.set_input("ok");
    assert_eq!(texts_of(&engine)[0], "哦");

    // 选过一次 OK：规则不再压过学习记录，`ok` 回到英文第一。
    pick(&mut engine, "ok", "OK");
    engine.set_input("ok");
    assert_eq!(texts_of(&engine)[0], "OK");

    // `mp` 同理：选过 MP 之后它也从中文切回英文第一。
    pick(&mut engine, "mp", "MP");
    engine.set_input("mp");
    assert_eq!(texts_of(&engine)[0], "MP");
}

#[test]
fn hyphen_turns_the_buffer_into_a_raw_english_segment() {
    let mut engine = engine();
    engine.set_input("no");
    assert!(!engine.raw_mode());
    engine.push('-');
    assert!(engine.raw_mode());
    engine.push('w');
    engine.push('a');
    engine.push('y');
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items.len(), 1);
    assert_eq!(query.candidates.items[0].text, "no-way");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::English);
    assert_eq!(query.tail, "no-way");
    let raw = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&raw), "no-way");
    assert!(engine.composition().is_empty());
    // 直输段不发云联想
    engine.set_input("a-b");
    assert_eq!(engine.request_prediction(None, &[]), None);
    // 表达式与问字模式优先
    engine.set_input("v1-2");
    assert!(engine.expression_mode() && !engine.raw_mode());
}

#[test]
fn english_completions_appear_when_pinyin_is_unlikely() {
    // Zipf×1000：company 5.6 / compare 4.45 / compass 3.74 都过补全门槛 3.5
    let words = WordList::parse(
            "company\tcompany\t5600\ncompare\tcompare\t4450\ncompass\tcompass\t3740\ncomma\tcomma\t2900\nxian\txian\t4500\nxiangkai\txiangkai\t10\n",
        )
        .unwrap();
    let mut engine = engine().with_english(words);
    // compa 切成 co'm'pa，不像拼音：补全排最前，最多三条、按词频
    engine.set_input("compa");
    let all = texts_of(&engine);
    assert_eq!(&all[..3], ["company", "compare", "compass"]);
    // xian 是干净的拼音：只有精确词 xian，不补全
    engine.set_input("xian");
    let all = texts_of(&engine);
    assert_eq!(all.iter().filter(|t| t.starts_with("xian")).count(), 1);
    // 太短的前缀不补全
    engine.set_input("co");
    assert!(!texts_of(&engine).contains(&"company".to_owned()));
    // 第一个字母就切不动的（i 不是任何音节的开头）也要出补全
    engine.set_input("impo");
    assert!(engine.query().is_err());
    let words = WordList::parse("important\timportant\t4800\nimport\timport\t4600\n").unwrap();
    let mut fresh = Engine::new(Dictionary::parse(SAMPLE).unwrap()).with_english(words);
    fresh.set_input("impo");
    assert_eq!(texts_of(&fresh), ["important", "import"]);
}

#[test]
fn english_mode_suggests_from_the_word_list_and_keeps_the_typed_text() {
    let words = WordList::parse(
        "company\tcompany\t900\ncompare\tcompare\t500\nhello\thello\t1000\nhelp\thelp\t700\n",
    )
    .unwrap();
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut engine = engine()
        .with_english(words)
        .with_learner(Box::new(CountingLearner(HashMap::new())))
        .with_predictor(Box::new(EchoPredictor {
            submitted: submitted.clone(),
            replies: Vec::new(),
            sentence: false,
        }));
    engine.set_english_mode(true);
    assert!(engine.english_mode());
    // 大小写跟着敲的走，marked text 就是敲的字母，没有拼音切分
    engine.set_input("Comp");
    let query = engine.query().unwrap();
    assert_eq!(texts_of(&engine), ["Company", "Compare"]);
    assert_eq!(query.marked_text(), "Comp");
    assert!(query.segmentations.is_empty());
    // 数字进缓冲区也只是没候选
    engine.set_input("foo1");
    assert!(texts_of(&engine).is_empty());
    assert_eq!(engine.query().unwrap().marked_text(), "foo1");
    // 英文模式不发云联想
    engine.set_input("comp");
    assert_eq!(engine.request_prediction(None, &[]), None);
    assert!(submitted.borrow().is_empty());
    // 选中的词记次数，下次同样的前缀它靠前
    let compare = engine.query().unwrap().candidates.items[1].clone();
    assert_eq!(engine.commit(&compare), "compare");
    assert!(engine.composition().is_empty());
    engine.set_input("comp");
    assert_eq!(texts_of(&engine), ["compare", "company"]);
    // 拼错一个字母也有候选；回车原样上屏敲的字母
    engine.set_input("helo");
    assert_eq!(texts_of(&engine), ["hello", "help"]);
    assert_eq!(engine.take_raw(), "helo");
    // emoji 排在所有词后面，不挡上下键选词
    let table = EmojiTable::parse("help\t🆘\n").unwrap();
    let mut engine = engine.with_emoji(table);
    engine.set_english_mode(true);
    engine.set_input("helo");
    assert_eq!(texts_of(&engine), ["hello", "help", "🆘"]);
    // 离开英文模式后同一串又按拼音算
    engine.set_english_mode(false);
    engine.set_input("comp");
    assert!(!engine.query().unwrap().segmentations.is_empty());
}

#[test]
fn raw_committed_english_words_are_learned_and_come_back_as_candidates() {
    use std::cell::RefCell;
    use std::rc::Rc;

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
    let _ = Rc::new(RefCell::new(()));
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_learner(Box::new(EnglishLearner::default()));
    // 随包词表里没有 gist：第一次只有拼音候选，回车原样上屏
    engine.set_input("gist");
    let first = engine.query().unwrap();
    assert!(
        first
            .candidates
            .items
            .iter()
            .all(|c| c.kind != CandidateKind::English)
    );
    assert_eq!(engine.take_raw(), "gist");
    // 第二次 gist 就是英文候选，而且排第一（拼音不像话）
    engine.set_input("gist");
    let second = engine.query().unwrap();
    assert_eq!(second.candidates.items[0].text, "gist");
    assert_eq!(second.candidates.items[0].kind, CandidateKind::English);
    // 能切成完整拼音的串回车不算英文词
    engine.set_input("hao");
    assert_eq!(engine.take_raw(), "hao");
    engine.set_input("hao");
    let hao = engine.query().unwrap();
    assert!(
        hao.candidates
            .items
            .iter()
            .all(|c| c.kind != CandidateKind::English)
    );
    // 英文模式下直通的词也学
    engine.set_english_mode(true);
    engine.set_input("wo");
    assert_eq!(engine.take_raw(), "wo");
    engine.set_english_mode(false);
    engine.set_input("wo");
    let wo = engine.query().unwrap();
    assert!(
        wo.candidates
            .items
            .iter()
            .any(|c| c.kind == CandidateKind::English && c.text == "wo")
    );
}

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

#[test]
fn passthrough_has_no_candidates_modes_or_full_width_punctuation() {
    let mut engine = engine().with_mode_keys(crate::ModeKeys {
        question_mark: true,
        ..crate::ModeKeys::default()
    });
    assert!(engine.takes_question_mark());
    engine.set_english_input_policy(true, false);
    for c in "hello?v123".chars() {
        engine.push(c);
    }
    assert!(engine.composition().is_empty());
    assert!(engine.query().unwrap().candidates.items.is_empty());
    assert!(!engine.takes_question_mark());
    assert!(!engine.takes_mode_letter('U'));
    assert_eq!(engine.punctuate(','), None);
    engine.set_english_mode(false);
    assert!(!engine.english_passthrough());
    assert_eq!(engine.punctuate(','), Some("，"));
}

#[test]
fn raw_switch_and_old_english_api_preserve_existing_behavior() {
    let words = WordList::parse("hello\thello\t100\n").unwrap();
    let mut engine = engine().with_english(words);
    engine.set_input("nihao");
    assert_eq!(engine.take_raw(), "nihao");
    engine.set_english_input_policy(true, false);
    engine.set_english_mode(true);
    engine.set_input("hello");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "hello");
}

#[test]
fn passthrough_policy_is_saved_and_discarded_with_session() {
    let mut engine = engine();
    let mut session = crate::EngineSession::default();
    engine.set_english_input_policy(true, false);
    engine.swap_session(&mut session);
    assert!(!engine.english_passthrough());
    engine.swap_session(&mut session);
    assert!(engine.english_passthrough());
    engine.discard_input();
    assert!(!engine.english_passthrough());
    engine.set_english_input_policy(true, false);
    engine.swap_session(&mut session);
    session.discard_input();
    engine.swap_session(&mut session);
    assert!(!engine.english_passthrough());
}

#[test]
fn caps_letter_normalization_preserves_real_shift_and_punctuation() {
    let mut engine = engine();
    for english in [false, true] {
        engine.set_english_input_policy(english, false);
        assert_eq!(engine.normalize_input_letter('A', true, false), 'a');
        assert_eq!(engine.normalize_input_letter('a', true, true), 'A');
        assert_eq!(engine.normalize_input_letter('A', false, false), 'A');
        for c in [',', '?', '1', '中', 'é'] {
            assert_eq!(engine.normalize_input_letter(c, true, true), c);
        }
    }
    engine.set_english_mode(false);
    for c in "NIHAO".chars() {
        engine.push(engine.normalize_input_letter(c, true, false));
    }
    assert_eq!(engine.composition().text(), "nihao");
    assert_eq!(engine.take_raw(), "nihao");
}
