//! 英文模式与中英混输。

use super::*;

mod tail;

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
    engine.set_input("kaifx");
    engine.backspace();
    engine.push('a');
    let kaifa = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(kaifa.text, "开发");
    engine.commit(&kaifa);
    // 拼音回车不算英文词，像英文的字母串才算
    engine.set_input("hao");
    engine.take_raw();
    engine.set_input("gist");
    engine.take_raw();
    let recorded = recorded.lock().unwrap();
    // 选的首选，敲了 6 键、中间删过一次
    let typing_ms = recorded[0].typing_ms;
    assert_eq!(
        recorded[0],
        Usage {
            hanzi: 2,
            words: 1,
            english_words: 0,
            commits: 1,
            choices: 1,
            first_choices: 1,
            typed_hanzi: 2,
            keys: 6,
            typing_ms,
            edited: 1,
        }
    );
    // 原样上屏不算选词，键数时长都不记
    assert_eq!(recorded[1].choices, 0);
    assert_eq!(recorded[1].keys, 0);
    assert_eq!(recorded[1].english_words, 0);
    assert_eq!(recorded[1].commits, 1);
    assert_eq!(recorded[2].english_words, 1);
    assert_eq!(recorded.len(), 3);
}

#[test]
fn unmetered_commits_are_not_counted_and_do_not_leak_keys() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let mut engine = engine().with_usage_meter(Box::new(MemoryMeter(recorded.clone())));
    // 壳的工具框（快捷翻译的原文）里打的不算
    engine.set_metered(false);
    engine.set_input("kaifa");
    let kaifa = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&kaifa);
    assert!(recorded.lock().unwrap().is_empty());
    // 回到应用里接着打：键数只算这一段的
    engine.set_metered(true);
    engine.set_input("kaifa");
    let again = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&again);
    let recorded = recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].keys, 5);
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
fn english_in_chinese_off_drops_words_and_completions_but_not_english_mode() {
    let words =
        WordList::parse("company\tcompany\t5600\ncompare\tcompare\t4450\nhello\thello\t1000\n")
            .unwrap();
    let mut engine = engine().with_english(words);
    engine.set_english_in_chinese(false);
    engine.set_input("compa");
    assert!(!texts_of(&engine).iter().any(|t| t.starts_with("comp")));
    engine.set_input("hello");
    assert!(!texts_of(&engine).contains(&"hello".to_owned()));
    engine.set_english_mode(true);
    engine.set_input("comp");
    assert_eq!(texts_of(&engine), ["company", "compare"]);
}

/// 关掉「打拼音时也给英文词」后英文词连句尾都不参与分词（#766）：
/// `nanine` 不再切成 `na` + `nine`，`nine` 也不再读成英文词。
#[test]
fn english_in_chinese_off_stops_the_english_tail() {
    const DICT: &str = "那\tna\t300000\n你\tni\t400000\n呢\tne\t200000\n那你呢\tna ni ne\t500000\n";
    let build = || {
        let words = WordList::parse("nine\tnine\t4500\nrust\trust\t3740\n").unwrap();
        Engine::new(Dictionary::parse(DICT).unwrap()).with_english(words)
    };

    // 开着时英文尾段照样参与比分，拼音读法按整句挑的切分（na ni ne，不是切分器贪心的 nan i ne）来比，那你呢 赢
    let mut on = build();
    on.set_input("nanine");
    assert_eq!(texts_of(&on)[0], "那你呢");

    let mut off = build();
    off.set_english_in_chinese(false);
    off.set_input("nanine");
    let all = texts_of(&off);
    assert_eq!(all[0], "那你呢");
    assert!(!all.iter().any(|text| text.contains("nine")), "{all:?}");
    // 单独打 nine 也回到拼音读法
    off.set_input("nine");
    let all = texts_of(&off);
    assert!(
        !all.iter().any(|text| text.eq_ignore_ascii_case("nine")),
        "{all:?}"
    );
}

#[test]
fn english_candidate_count_stops_at_the_configured_limit() {
    let words =
        WordList::parse("company\tcompany\t5600\ncompare\tcompare\t4450\ncompass\tcompass\t3000\n")
            .unwrap();
    let mut engine = engine().with_english(words);
    engine.set_input("compa");
    // 缺省 4：一条精确词都没有时补全仍按老样子最多 3 条
    assert_eq!(texts_of(&engine)[..3], ["company", "compare", "compass"]);
    engine.set_english_max_in_chinese(2);
    let all = texts_of(&engine);
    assert_eq!(
        all.iter()
            .filter(|text| ["company", "compare", "compass"].contains(&text.as_str()))
            .count(),
        2
    );
    // 1 只留最靠前的一条；0 越界收到 1
    engine.set_english_max_in_chinese(0);
    let all = texts_of(&engine);
    assert_eq!(
        all.iter()
            .filter(|text| ["company", "compare", "compass"].contains(&text.as_str()))
            .count(),
        1
    );
    // 整段就是精确词时，1 也只留它
    engine.set_input("company");
    engine.set_english_max_in_chinese(1);
    assert_eq!(
        texts_of(&engine)
            .iter()
            .filter(|text| ["company", "compare", "compass"].contains(&text.as_str()))
            .count(),
        1
    );
}

#[test]
fn english_min_position_keeps_chinese_candidates_above() {
    const DICT: &str = "吃\tchi\t5000\n拿\tna\t5000\n吃拿\tchi na\t6000\n";
    let words = WordList::parse("china\tchina\t5000\nGitHub\tgithub\t3000\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap()).with_english(words);
    engine.set_input("china");
    // 缺省不限制：英文词跟在第一个中文候选后面
    let all = texts_of(&engine);
    let english = all.iter().position(|text| text == "china").unwrap();
    assert!(english < 2, "{all:?}");
    // 中文候选够多时，英文最高排到第 3 位
    engine.set_english_min_position(3);
    let all = texts_of(&engine);
    assert_eq!(all[2], "china", "{all:?}");
    assert!(all[..2].iter().all(|text| text != "china"));
    // 没有中文候选时仍排第一
    engine.set_english_min_position(9);
    engine.set_input("github");
    assert_eq!(texts_of(&engine)[0], "GitHub");
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
    // emoji 与颜文字排在所有词后面，不挡上下键选词
    let table = EmojiTable::parse("help\t🆘\n").unwrap();
    let mut engine = engine.with_emoji(table);
    engine.set_english_mode(true);
    engine.set_input("helo");
    assert_eq!(texts_of(&engine), ["hello", "help", "(＾▽＾)ノ", "🆘"]);
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

#[test]
fn english_word_is_not_buried_by_reading_key_choices() {
    // `wenti` 下选的 问题（读音键记了一份）不能隔着 `went` 把英文词顶下去：
    // 英文位置的仲裁按字面键数，读音键泛化只服务中文自己的重打排序
    let dictionary = Dictionary::parse("问题\twen ti\t900000\n").unwrap();
    let words = WordList::parse("went\twent\t2000\n").unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(words)
        .with_learner(Box::new(CountingLearner(HashMap::new())));
    let commit = |engine: &mut Engine, input: &str, text: &str| {
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
    commit(&mut engine, "wenti", "问题");
    engine.set_input("went");
    assert_eq!(texts_of(&engine)[0], "went");
}
