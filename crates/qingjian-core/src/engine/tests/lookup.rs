//! 查词、切分、光标与上屏消耗。

use super::*;
use crate::ranking::FIRST_SINGLE_CHAR_BY;

#[test]
fn exact_word_first_then_longer_then_prefix_expansions_then_prefix_words() {
    assert_eq!(texts("kaifa"), ["开发", "开放", "开饭", "开发者", "开"]);
}

#[test]
fn partial_last_syllable_expands() {
    assert_eq!(texts("kaif"), ["开放", "开发", "开饭", "开发者", "开"]);
}

#[test]
fn initials_match_abbreviated_words_and_commit_consumes_letters() {
    let all = texts("kf");
    assert_eq!(all[0], "开放"); // 同为简拼命中，按词频
    assert!(all.contains(&"开发".to_owned()) && all.contains(&"咖啡".to_owned()));

    let mut engine = engine();
    engine.set_input("kfzhe");
    let kaifa = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    engine.commit(&kaifa);
    assert_eq!(engine.composition().text(), "zhe");
}

#[test]
fn unparsable_tail_is_kept_aside() {
    let mut engine = engine();
    // kaiv 唯一的纠法是删掉刚敲的 v，那不算纠错：v 留作尾巴等下一键（kaifv 会被纠成 kaifa）
    engine.set_input("kaiv");
    let query = engine.query().unwrap();
    assert!(query.correction.is_none());
    assert_eq!(query.tail, "v");
    assert_eq!(query.marked_text(), "kai'v");
    assert_eq!(query.candidates.items[0].text, "开");

    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    engine.commit(&kaifa);
    assert_eq!(engine.composition().text(), "v");
    // 剩下的 v 进表达式模式：没候选但也不报错
    assert!(engine.query().unwrap().candidates.items.is_empty());
}

#[test]
fn cursor_edits_requery_from_the_start_and_map_into_marked_text() {
    let mut engine = engine();
    engine.set_input("kaifa");
    engine.move_cursor_left();
    engine.move_cursor_left();
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'fa");
    assert_eq!(query.marked_cursor(), 3); // kai|'fa

    engine.push('n');
    let query = engine.query().unwrap();
    assert_eq!(query.text, "kainfa");
    assert_eq!(query.marked_cursor(), 5); // kai'n|'fa

    engine.set_input("xi'an");
    engine.move_cursor_left();
    engine.move_cursor_left();
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "xi'an");
    assert_eq!(query.marked_cursor(), 3); // xi'|an，紧跟用户自己敲的 '
}

#[test]
fn shifted_brackets_punctuate_to_corner_quotes() {
    // macOS 上 ⇧[ / ⇧] 产生的字符是 { / }，全角标点下应出「」而不是放行半角花括号（issue #228）
    let mut engine = engine();
    assert_eq!(engine.punctuate('{'), Some("「"));
    assert_eq!(engine.punctuate('}'), Some("」"));
}

#[test]
fn punctuation_follows_committed_text() {
    let mut engine = engine();
    assert_eq!(engine.punctuate(','), Some("，"));
    engine.note_passthrough('3');
    assert_eq!(engine.punctuate('.'), None);
    engine.set_input("kaifa");
    let kaifa = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&kaifa);
    assert_eq!(engine.punctuate('.'), Some("。"));
}

#[test]
fn marked_text_joins_best_segmentation_with_apostrophes() {
    let mut engine = engine();
    engine.set_input("kaifa");
    assert_eq!(engine.query().unwrap().marked_text(), "kai'fa");
    engine.set_input("kf");
    assert_eq!(engine.query().unwrap().marked_text(), "k'f");
}

#[test]
fn prefix_words_appear_after_full_matches() {
    // 开放 / 开饭 来自 `kai f… a zhe` 这种把 `fa` 拆开的切分，`kai fa zhe` 读得通时排到最后，单字 开 在它们前面
    let all = texts("kaifazhe");
    assert_eq!(&all[..2], ["开发者", "开发"]);
    let mut last = all[all.len() - 2..].to_vec();
    last.sort();
    assert_eq!(last, ["开放", "开饭"]);
    assert!(all.contains(&"开".to_owned()));
}

fn texts_in(dictionary: &str, input: &str) -> Vec<String> {
    let mut engine = Engine::new(Dictionary::parse(dictionary).unwrap());
    engine.set_input(input);
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .filter(|c| c.kind != CandidateKind::Sentence)
        .map(|c| c.text)
        .collect()
}

fn position(all: &[String], text: &str) -> usize {
    all.iter()
        .position(|t| t == text)
        .unwrap_or_else(|| panic!("{text} 不在候选里：{all:?}"))
}

#[test]
fn single_chars_come_before_guessed_words_and_split_readings() {
    // 整段读得通（`shi shan`）但词库里没有想要的词：猜出来的长词至多三条，然后是首音节的单字，
    // 拆开 `shan` 读出来的（`shi s… h… an` 的 事实、`shi sha n…` 的 是啥呢）排在单字后面
    let dictionary = concat!(
        "是\tshi\t9000\n时\tshi\t8000\n石山\tshi shan\t100\n",
        "时尚\tshi shang\t9000\n世上\tshi shang\t8000\n史上\tshi shang\t7000\n事上\tshi shang\t6000\n",
        "时尚潮流\tshi shang chao liu\t5000\n事实\tshi shi\t9000\n是啥呢\tshi sha ne\t9000\n",
    );
    let all = texts_in(dictionary, "shishan");
    assert_eq!(all[0], "石山");
    for guess in ["时尚", "世上", "史上"] {
        assert!(position(&all, guess) < position(&all, "是"), "{all:?}");
    }
    for later in ["事上", "时尚潮流", "事实", "是啥呢"] {
        assert!(position(&all, "时") < position(&all, later), "{all:?}");
    }
}

#[test]
fn long_input_without_a_word_offers_the_first_syllables_chars_after_a_few_words() {
    // #769：整段没有词时，盖住开头几个音节的词至多几个，接着就是首音节的单字，不穿插也不用翻几页
    let dictionary = concat!(
        "实验室\tshi yan shi\t9000\n十堰市\tshi yan shi\t8000\n试验室\tshi yan shi\t7000\n",
        "实验\tshi yan\t9000\n试验\tshi yan\t8000\n是\tshi\t9000\n时\tshi\t8000\n十\tshi\t7000\n",
    );
    let all = texts_in(dictionary, "shiyanshiqing");
    assert_eq!(
        &all[..8],
        [
            "实验室",
            "十堰市",
            "试验室",
            "实验",
            "试验",
            "是",
            "时",
            "十"
        ]
    );
}

#[test]
fn sentences_and_many_words_still_leave_a_first_syllable_char_on_the_first_page() {
    // #769：整句与词合起来占满第一页时，第一个单字最晚在第 9 位，让出来的词排在单字后面
    let dictionary = concat!(
        "实验室\tshi yan shi\t9000\n十堰市\tshi yan shi\t8000\n试验室\tshi yan shi\t7000\n",
        "实验师\tshi yan shi\t6000\n实验\tshi yan\t9000\n试验\tshi yan\t8000\n饰演\tshi yan\t7000\n",
        "是\tshi\t9000\n时\tshi\t8000\n情\tqing\t9000\n轻\tqing\t8000\n清\tqing\t7000\n",
    );
    let mut engine = Engine::new(Dictionary::parse(dictionary).unwrap());
    engine.set_input("shiyanshiqing");
    let items = engine.query().unwrap().candidates.items;
    let texts: Vec<&str> = items.iter().map(|c| c.text.as_str()).collect();
    let sentences = items
        .iter()
        .take_while(|c| c.kind == CandidateKind::Sentence)
        .count();
    assert!(sentences >= 3, "{texts:?}");
    let first = position_of(&texts, "是");
    assert!(first < FIRST_SINGLE_CHAR_BY, "{texts:?}");
    assert_eq!(texts[first + 1], "时", "{texts:?}");
    assert!(position_of(&texts, "饰演") > first, "{texts:?}");
}

fn position_of(texts: &[&str], text: &str) -> usize {
    texts.iter().position(|t| *t == text).unwrap_or(usize::MAX)
}

#[test]
fn ambiguous_segmentation_merges_results() {
    let all = texts("xian");
    assert_eq!(all[0], "先");
    assert!(all.contains(&"西安".to_owned()));
}

#[test]
fn complete_syllable_that_is_also_prefix_expands_after_exact() {
    // 下 是完整命中；先 / 想 来自 xia → xian / xiang 的前缀扩展；西安 来自 xi + a… 的次级切分
    assert_eq!(texts("xia"), ["下", "先", "想", "西安"]);
}

#[test]
fn keyboard_u_umlaut_spelling_matches_canonical_dictionary_keys() {
    let dictionary =
        Dictionary::parse("策略\tce lve\t9000\n虐待\tnve dai\t8000\n学习\txue xi\t7000\n").unwrap();
    let mut engine = Engine::new(dictionary);

    engine.set_input("celue");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "ce'lüe");
    assert_eq!(query.candidates.items[0].text, "策略");
    assert_eq!(query.candidates.items[0].syllables, ["ce", "lve"]);
    let strategy = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&strategy), "策略");
    assert!(engine.composition().is_empty());

    engine.set_input("nuedai");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "虐待");

    engine.set_input("xuexicelue");
    let sentence = &engine.query().unwrap().candidates.items[0];
    assert_eq!(sentence.text, "学习策略");
    assert_eq!(sentence.kind, CandidateKind::Sentence);
}

#[test]
fn keyboard_v_after_jqxy_matches_u_dictionary_keys() {
    let dictionary =
        Dictionary::parse("提取\tti qu\t9000\n去留\tqu liu\t8000\n学习\txue xi\t7000\n").unwrap();
    let mut engine = Engine::new(dictionary);

    engine.set_input("tiqv");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "ti'qu");
    assert_eq!(query.candidates.items[0].text, "提取");
    assert_eq!(query.candidates.items[0].syllables, ["ti", "qu"]);

    engine.set_input("qvliu");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "去留");

    engine.set_input("xuexiqvliu");
    let sentence = &engine.query().unwrap().candidates.items[0];
    assert_eq!(sentence.text, "学习去留");
    assert_eq!(sentence.kind, CandidateKind::Sentence);
}

#[test]
fn empty_input_is_an_error() {
    assert_eq!(engine().query().unwrap_err(), ParseError::Empty);
}

#[test]
fn commit_consumes_only_the_candidate_syllables() {
    let mut engine = engine();
    engine.set_input("kaifazhe");
    let kaifa = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    assert_eq!(engine.commit(&kaifa), "");
    assert_eq!(engine.composition().text(), "zhe");

    engine.set_input("kaif");
    assert_eq!(engine.commit(&kaifa), "开发");
    assert!(engine.composition().is_empty());

    engine.set_input("xi'an");
    let xian = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .find(|c| c.text == "西安")
        .unwrap()
        .clone();
    engine.commit(&xian);
    assert!(engine.composition().is_empty());
}

#[test]
fn take_raw_returns_pinyin_and_clears() {
    let mut engine = engine();
    engine.set_input("kaifa");
    assert_eq!(engine.take_raw(), "kaifa");
    assert!(engine.composition().is_empty());
}

#[test]
fn choice_key_strips_separators_and_clamps() {
    assert_eq!(choice_key("kai'fa", 6), "kaifa");
    assert_eq!(choice_key("kai'fa", 3), "kai");
    assert_eq!(choice_key("ba", 10), "ba");
}

#[test]
fn sentence_conversion_leads_when_input_spans_several_words() {
    // 想开发：SAMPLE 里没有整词，最优路径是 想 + 开发
    let all = texts("xiangkaifa");
    assert_eq!(all[0], "想开发");
    let mut engine = engine();
    engine.set_input("xiangkaifa");
    let query = engine.query().unwrap();
    let sentence = query.candidates.items[0].clone();
    assert_eq!(sentence.kind, CandidateKind::Sentence);
    assert_eq!(sentence.syllables, ["xiang", "kai", "fa"]);
    assert_eq!(engine.commit(&sentence), "想开发");
    assert!(engine.composition().is_empty());

    // 整段本身是一个词：不出整句
    let all = texts("kaifa");
    assert_eq!(all[0], "开发");
    assert_eq!(all.iter().filter(|t| *t == "开发").count(), 1);

    // 末尾落单的声母也参与整句：xiangkaif → 想开放（f 当前缀，fang / fa 里词频高的 开放 胜出）
    let all = texts("xiangkaif");
    assert_eq!(all[0], "想开放");
}

#[test]
fn trailing_initial_sentence_yields_to_a_word_covering_the_whole_input() {
    // 单字组句「后悬空」词频远高于低频整词「候选框」，落单 k 进词图后整句本来会赢
    let dict = "后\thou\t900000\n悬\txuan\t900000\n空\tkong\t900000\n\
        候\thou\t100\n选\txuan\t100\n框\tkuang\t100\n候选框\thou xuan kuang\t500\n";
    let mut engine = Engine::new(Dictionary::parse(dict).unwrap());
    engine.set_input("houxuank");
    let items = engine.query().unwrap().candidates.items;
    // 词库整词逐音节对上全部输入（末位 kuang 按前缀 k 命中），整句不插队
    assert_eq!(items[0].text, "候选框");
    assert!(items.iter().all(|c| c.kind != CandidateKind::Sentence));

    // 两音节同样：zaiw → 在外（zai wai），不让整句「在我」（zai wo）
    let dict = "在\tzai\t900000\n我\two\t900000\n外\twai\t100\n在外\tzai wai\t500\n";
    let mut engine = Engine::new(Dictionary::parse(dict).unwrap());
    engine.set_input("zaiw");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "在外");
    assert!(items.iter().all(|c| c.kind != CandidateKind::Sentence));

    // 对照：没有整词覆盖全部输入时，落单声母的整句照常给出
    let dict = "后\thou\t900000\n悬\txuan\t900000\n空\tkong\t900000\n\
        候\thou\t100\n选\txuan\t100\n框\tkuang\t100\n";
    let mut engine = Engine::new(Dictionary::parse(dict).unwrap());
    engine.set_input("houxuank");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "后悬空");
    assert_eq!(items[0].kind, CandidateKind::Sentence);
}

#[test]
fn shortcuts_follow_the_first_local_candidate() {
    let all = texts("rq");
    let shortcut = all.iter().position(|t| t.ends_with('日')).unwrap();
    assert!(shortcut <= 1);
    assert!(all.iter().any(|t| t.contains('-')));

    let mut engine = engine();
    engine.set_input("xq");
    let query = engine.query().unwrap();
    let weekday = query
        .candidates
        .items
        .iter()
        .find(|c| c.kind == CandidateKind::Shortcut)
        .unwrap()
        .clone();
    assert!(weekday.text.starts_with("星期"));
    engine.commit(&weekday);
    assert!(engine.composition().is_empty());
}

#[test]
fn full_pinyin_keyword_brings_its_shortcuts() {
    let mut engine = Engine::new(Dictionary::parse(KEYWORDS).unwrap());
    engine.set_input("shijian");
    let all: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert_eq!(all[0], "时间");
    assert!(all[1].contains(':'), "时间之后紧跟现在的时间：{all:?}");
    // 一组写法只出一遍
    assert_eq!(all.iter().filter(|t| t.contains(':')).count(), 2);
}

#[test]
fn ng_alone_gives_en_interjection_first_in_full_pinyin() {
    let mut engine = engine();
    engine.set_input("ng");
    let query = engine.query().unwrap();
    let first = query.candidates.items[0].clone();
    assert_eq!(first.text, "嗯");
    engine.commit(&first);
    assert!(engine.composition().is_empty());

    engine.set_input("ngh");
    assert!(texts_of(&engine).iter().all(|t| t != "嗯"));

    // hng 只有 哼；hm 还是 好吗 一类的简拼，噷 排第二
    engine.set_input("hng");
    assert_eq!(texts_of(&engine)[0], "哼");
    engine.set_input("hm");
    assert!(
        texts_of(&engine)
            .iter()
            .position(|t| t == "噷")
            .is_some_and(|at| at <= 1)
    );
}

#[test]
fn shuangpin_ng_offers_en_second_after_neng() {
    // #880：小鹤里 ng 是 neng，能 留在第一，嗯 跟在后面；双拼的 hm / hng 是别的音，不给叹词
    let dictionary = Dictionary::parse("能\tneng\t90000\n").unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_input("ng");
    let texts = texts_of(&engine);
    assert_eq!(texts[0], "能");
    assert_eq!(texts[1], "嗯");
    engine.set_input("hm");
    assert!(texts_of(&engine).iter().all(|t| t != "噷"));
}

#[test]
fn shift_letters_join_the_buffer_only_when_configured() {
    let dictionary = Dictionary::parse("C盘\tc pan\t8000\n磁盘\tci pan\t249\n").unwrap();
    let mut engine = Engine::new(dictionary);
    let type_cpan = |engine: &mut Engine| {
        // 中文模式下按住 Shift 敲 C，再打 pan
        engine.push('C');
        for c in "pan".chars() {
            engine.push(c);
        }
    };

    // 缺省 `shift_letter = "passthrough"`：壳直接把大写字母交给应用，这一路本来就不会走到；
    // 万一走到也不该被当成拼音去匹配（所以「C盘」出不来）
    type_cpan(&mut engine);
    assert_ne!(engine.query().unwrap().candidates.items[0].text, "C盘");
    engine.clear();

    // 开了 compose：按小写参与匹配，拼音行按敲的样子显示，回车原样上屏时保留大写
    engine.set_shift_letter_compose(true);
    type_cpan(&mut engine);
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "C盘");
    assert_eq!(query.marked_text(), "C'pan");
    assert_eq!(engine.take_raw(), "Cpan");
    assert!(engine.composition().is_empty());
}

#[test]
fn expression_mode_skips_pinyin_and_evaluates() {
    let mut engine = self::engine();
    assert!(!engine.expression_mode());
    engine.set_input("v1+2");
    assert!(engine.expression_mode());
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "v1+2");
    assert_eq!(query.marked_cursor(), 4);
    assert_eq!(query.candidates.items[0].text, "3");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Shortcut);
    assert_eq!(query.candidates.items[1].text, "1+2=3");
    let result = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&result), "3");
    assert!(engine.composition().is_empty());

    // 只有 v：候选为空但不报错，preedit 照显示
    engine.set_input("v");
    let query = engine.query().unwrap();
    assert!(query.candidates.items.is_empty());
    assert_eq!(query.marked_text(), "v");

    // v 开头的英文词仍能混输
    let words = WordList::parse("very\tvery\t4800\n").unwrap();
    let mut engine = self::engine().with_english(words);
    engine.set_input("very");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "very");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::English);
}

/// 只认句首的 开发 与 开发 → 先 的假模型：让两词路径压过整段的词。
struct XianModel;

impl LanguageModel for XianModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        match (previous, word) {
            (None, "开发") => Some(-1.0),
            (Some("开发"), "先") => Some(-0.5),
            _ => None,
        }
    }
}

#[test]
fn a_word_spelling_the_sentence_keeps_its_rank_unless_its_reading_differs() {
    // 词级排序里 开发线 在 开发先 前面；整句转换读出的是 开发 + 先，与词 开发先 同文本同读音：不重复插，词留在原位
    let sample = format!("{SAMPLE}开发线\tkai fa xian\t5000\n开发先\tkai fa xian\t1\n");
    let mut engine =
        Engine::new(Dictionary::parse(&sample).unwrap()).with_language_model(Box::new(XianModel));
    engine.set_input("kaifaxian");
    let all = texts_of(&engine);
    assert_eq!(&all[..2], ["开发线", "开发先"]);
    assert_eq!(all.iter().filter(|t| *t == "开发先").count(), 1);

    // 同文本的词是按别的读音（xiang，靠模糊音 an-ang 对上）收的：那条错读音的词让位，整句以正确读音排最前
    let sample = format!("{SAMPLE}开发线\tkai fa xian\t5000\n开发先\tkai fa xiang\t1\n");
    let mut engine =
        Engine::new(Dictionary::parse(&sample).unwrap()).with_language_model(Box::new(XianModel));
    engine.set_fuzzy(FuzzyRules {
        an_ang: true,
        ..FuzzyRules::default()
    });
    engine.set_input("kaifaxian");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "开发先");
    assert_eq!(items[0].kind, CandidateKind::Sentence);
    assert_eq!(items[0].syllables, ["kai", "fa", "xian"]);
    assert_eq!(items.iter().filter(|c| c.text == "开发先").count(), 1);
}

#[test]
fn option_backspace_deletes_a_syllable_and_command_backspace_deletes_to_the_start() {
    let mut engine = engine();
    // 全拼：删最优切分的最后一个音节；`'` 连同前面的音节一起删；切不动的尾巴整个删
    engine.set_input("kaifaxian");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kaifa");
    engine.set_input("kai'fa'");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kai'");
    engine.set_input("kaifv");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kaif");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kai");
    // 光标停在中间：只动光标前的
    engine.set_input("kaifaxian");
    for _ in 0..4 {
        engine.move_cursor_left();
    }
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kaixian");
    assert_eq!(engine.composition().cursor(), 3);
    assert!(engine.delete_to_start());
    assert_eq!(engine.composition().text(), "xian");
    assert_eq!(engine.composition().cursor(), 0);
    assert!(!engine.delete_syllable_backward());
    assert!(!engine.delete_to_start());
    // 英文直输段：字母一段一段删，标点一次一个
    engine.set_input("hello,world");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "hello,");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "hello");
    // 双拼：两键一音节，落单的一键单删
    let mut engine = xiaohe();
    engine.set_input("kdfah");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kdfa");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kd");
}

#[test]
fn option_arrows_move_the_cursor_by_syllable() {
    let mut engine = engine();
    // 光标在末尾发现前面错了：⌥← 两下跳到第二个音节后面，⌥⌫ 删掉它，重敲，⌘→ 回末尾
    engine.set_input("kaifa'xian'xia");
    assert!(engine.move_cursor_syllable_left());
    assert_eq!(engine.composition().cursor(), "kaifa'xian'".len());
    assert!(engine.move_cursor_syllable_left());
    assert_eq!(engine.composition().cursor(), "kaifa'".len());
    assert!(engine.move_cursor_syllable_left());
    assert_eq!(engine.composition().cursor(), "kai".len());
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "fa'xian'xia");
    engine.push('x');
    engine.push('i');
    assert_eq!(engine.composition().text(), "xifa'xian'xia");
    // ⌥→：从 xi| 起跳过一个音节到 xifa|，再跳先越过 `'` 再过一个音节
    assert!(engine.move_cursor_syllable_right());
    assert_eq!(engine.composition().cursor(), "xifa".len());
    assert!(engine.move_cursor_syllable_right());
    assert_eq!(engine.composition().cursor(), "xifa'xian".len());
    assert!(engine.move_cursor_syllable_right());
    assert_eq!(engine.composition().cursor(), "xifa'xian'xia".len());
    assert!(!engine.move_cursor_syllable_right());
    engine.move_cursor_home();
    assert!(!engine.move_cursor_syllable_left());
    // 直输段按字母段跳
    engine.set_input("hello,world");
    assert!(engine.move_cursor_syllable_left());
    assert_eq!(engine.composition().cursor(), "hello,".len());
    engine.move_cursor_home();
    assert!(engine.move_cursor_syllable_right());
    assert_eq!(engine.composition().cursor(), "hello".len());
}

#[test]
fn committing_the_rest_after_a_separator_split_consumes_it() {
    // `shan'gou` 先选 闪，剩下 `'gou`；再选 购 要把它吃完，不能一直留着让空格反复上屏 购（#669）
    let dictionary = "闪\tshan\t9000\n山沟\tshan gou\t8000\n购\tgou\t9000\n";
    let mut engine = Engine::new(Dictionary::parse(dictionary).unwrap());
    engine.set_input("shan'gou");
    for (text, committed) in [("闪", ""), ("购", "闪购")] {
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == text)
            .unwrap();
        assert_eq!(engine.commit(&candidate), committed);
    }
    assert!(engine.composition().is_empty());
}

#[test]
fn completions_stay_in_front_while_the_last_syllable_is_still_being_typed() {
    // `jixub` 的 b 还没打完：补全正是用户要往下打的，不限条数，也不把单字提前
    let dictionary = concat!(
        "继续\tji xu\t9000\n继续保留\tji xu bao liu\t900\n继续秉承\tji xu bing cheng\t800\n",
        "继续北上\tji xu bei shang\t700\n继续奔跑\tji xu ben pao\t600\n及\tji\t9000\n",
    );
    let all = texts_in(dictionary, "jixub");
    for completion in ["继续保留", "继续秉承", "继续北上", "继续奔跑"] {
        assert!(
            position(&all, completion) < position(&all, "继续"),
            "{all:?}"
        );
    }
    assert!(position(&all, "继续") < position(&all, "及"), "{all:?}");
}

#[test]
fn split_readings_keep_their_place_when_no_word_reads_as_typed() {
    // `wodei` 想打 我的（口语 dei）：`wo dei` 读不出词，`wo d… ei` 的前缀 我的 正是要的，不往后放
    let dictionary = "我的\two de\t9000\n我带\two dai\t800\n我\two\t9000\n沃\two\t100\n";
    let all = texts_in(dictionary, "wodei");
    assert_eq!(all[0], "我的");
    assert!(position(&all, "我") < position(&all, "沃"), "{all:?}");
}

#[test]
fn split_readings_far_from_the_typed_pinyin_come_after_every_single_char() {
    // #854：`nihao` 读成 `ni h… ao` 捞出的 你会、霓虹 读音对不上，排在首音节全部单字之后；
    // 只差一个字母的（你喝 nihe）留在单字中间当敲错的兜底
    let mut dictionary = String::from(concat!(
        "你好\tni hao\t9000\n你会\tni hui\t9000\n",
        "霓虹\tni hong\t8000\n你喝\tni he\t100\n",
    ));
    for (i, c) in "你尼妮拟泥倪逆腻呢霓匿溺昵".chars().enumerate() {
        dictionary.push_str(&format!("{c}\tni\t{}\n", 9000 - i));
    }
    let all = texts_in(&dictionary, "nihao");
    assert_eq!(all[0], "你好");
    assert!(position(&all, "你喝") < position(&all, "霓"), "{all:?}");
    for far in ["你会", "霓虹"] {
        assert!(position(&all, "昵") < position(&all, far), "{all:?}");
    }
}

#[test]
fn an_abbreviation_with_thousands_of_words_still_offers_the_first_initials_chars() {
    // `ss` 读得出的两字词多到预选装不下时，首音节的单字仍在几个词之后出现，不被一起砍掉
    let mut dictionary = String::from("是\tshi\t9000\n说\tshuo\t8000\n");
    for n in 0..3000 {
        dictionary.push_str(&format!("森森{n}\tsen sen\t{}\n", 9000 - n % 100));
    }
    let all = texts_in(&dictionary, "ss");
    assert!(all.iter().any(|t| t == "是"), "{all:?}");
}

#[test]
fn many_whole_words_still_leave_room_for_single_chars() {
    // `zhiyi` 读得出五十多个整词：前 6 个之后先给首音节的单字，再接其余整词
    let mut dictionary = String::from("知\tzhi\t100\n");
    for (i, c) in "一二三四五六七八九十百千万亿".chars().enumerate() {
        dictionary.push_str(&format!("之{c}\tzhi yi\t{}\n", 9000 - i));
    }
    let all = texts_in(&dictionary, "zhiyi");
    let first_char = position(&all, "知");
    assert_eq!(first_char, 6, "{all:?}");
    assert!(
        all[first_char + 1..].iter().any(|t| t.starts_with('之')),
        "{all:?}"
    );
}

#[test]
fn abbreviation_reading_c_h_as_two_initials_commits_every_key() {
    // #870：`ljchlsg` 的 `ch` 是 成、灰 两个声母，不能当 成 一个音节吃掉而剩下 `lsg`
    let dictionary =
        Dictionary::parse("蜡炬成灰泪始干\tla ju cheng hui lei shi gan\t100\n").unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_input("ljchlsg");
    let idiom = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "蜡炬成灰泪始干")
        .unwrap();
    assert_eq!(engine.commit(&idiom), "蜡炬成灰泪始干");
    assert!(engine.composition().is_empty());
}
