//! 拼写纠错 / 敲错边 / 模糊音。

use super::*;

/// 每个音节都合法的输入（`meiganxi`）：原样读得通（美感 + 洗）就按原样排第一，词图里敲错边读出的 没关系 当第二条；
/// 选了它上屏按敲的字母消耗、记个人敲错表，退格重选时退回。原样读不通时（只有 干 没有 感）敲错边照旧排第一。
#[test]
fn typo_edges_in_the_lattice_correct_legal_but_unlikely_pinyin() {
    let dictionary = Dictionary::parse(
        "没关系\tmei guan xi\t800000\n关系\tguan xi\t500000\n没\tmei\t900000\n干\tgan\t200000\n\
             洗\txi\t100000\n美感\tmei gan\t300000\n感谢\tgan xie\t300000\n谢\txie\t50000\n",
    )
    .unwrap();
    let mut engine =
        Engine::new(dictionary).with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_input("meiganxi");
    let query = engine.query().unwrap();
    assert!(query.correction.is_none());
    assert_eq!(query.candidates.items[0].text, "美感洗");
    let corrected = query.candidates.items[1].clone();
    assert_eq!(corrected.text, "没关系");
    assert_eq!(corrected.kind, CandidateKind::Chinese);
    assert_eq!(corrected.syllables, ["mei", "guan", "xi"]);
    assert!(query.candidates.items.iter().any(|c| c.text == "美感"));
    assert_eq!(engine.commit(&corrected), "没关系");
    assert!(engine.composition().is_empty());
    assert_eq!(engine.learner().typo_count("gan", "guan"), 1);
    assert_eq!(engine.learner().choice_weight("meiganxi", "没关系"), 1);
    // 整个删掉、同一段拼音改选 美感：敲错的记录退回
    for _ in 0..3 {
        engine.note_backspace();
    }
    engine.set_input("meiganxi");
    let meigan = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "美感")
        .unwrap();
    engine.commit(&meigan);
    assert_eq!(engine.learner().typo_count("gan", "guan"), 0);
    assert_eq!(engine.composition().text(), "xi");
    // 原样读得通：没 + 感谢 是正常整句，不动
    engine.set_input("meiganxie");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "没感谢");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Sentence);
    // 两个音节也纠：ganxi 原样只拼得出 干 + 洗 两个单字，不算读得通，关系 排第一
    engine.set_input("ganxi");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "关系");
    // 太短不纠（不到 4 个字母）
    engine.set_input("gan");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.text != "关系")
    );
    // 敲的拼音本身正好是一个词（按另一种切分）：不许敲错边压过它。jineng 最优切分是 jin eng，词图里读出 近藤，
    // 但 技能 的音节正好拼成整段输入
    let dictionary = Dictionary::parse(
        "技能\tji neng\t300000\n近藤\tjin teng\t900000\n近\tjin\t500000\n\
             机\tji\t400000\n能\tneng\t600000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_input("jineng");
    let query = engine.query().unwrap();
    assert_eq!(query.segmentations[0].joined("'"), "jin'eng");
    assert_eq!(query.candidates.items[0].text, "技能");
    assert!(query.candidates.items.iter().all(|c| c.text != "近藤"));
    // 退回原样的路径时原样的整句照出：shude 词图里 是的（shu → shi 相邻键）赢，但 树德 正好拼成 shude，
    // 于是整句退回 属的
    let dictionary = Dictionary::parse(
            "树德\tshu de\t500\n是的\tshi de\t5000000\n属\tshu\t100000\n的\tde\t8000000\n是\tshi\t7000000\n",
        )
        .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_input("shude");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "属的");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Sentence);
    assert!(query.candidates.items.iter().all(|c| c.text != "是的"));
}

/// 模糊音命中的词按敲的字母消耗拼音（`zi` 对 `zhi`），不算敲错。
#[test]
fn fuzzy_hits_consume_the_typed_syllables() {
    let dictionary =
        Dictionary::parse("知识\tzhi shi\t500000\n只是\tzhi shi\t600000\n资\tzi\t1000\n").unwrap();
    let mut engine =
        Engine::new(dictionary).with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_fuzzy(FuzzyRules {
        z_zh: true,
        ..FuzzyRules::default()
    });
    engine.set_input("zishi");
    let zhishi = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "知识")
        .unwrap();
    engine.commit(&zhishi);
    assert!(engine.composition().is_empty());
    assert_eq!(engine.learner().typo_count("zi", "zhi"), 0);
}

/// 接受整段一处编辑的纠正也记个人敲错表：敲的那段字母对纠正后的音节。
#[test]
fn accepted_whole_string_correction_feeds_the_typo_table() {
    let dictionary = Dictionary::parse(
            "你好吗\tni hao ma\t5000\n你好\tni hao\t9000\n你\tni\t90000\n好\thao\t80000\n吗\tma\t70000\n",
        )
        .unwrap();
    let mut engine =
        Engine::new(dictionary).with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_input("nihooma");
    let query = engine.query().unwrap();
    assert_eq!(query.correction.as_ref().unwrap().corrected, "nihaoma");
    let first = query.candidates.items[0].clone();
    engine.commit(&first);
    assert_eq!(engine.learner().typo_count("hoo", "hao"), 1);
    // 只选到 你 就上屏：没吃到编辑处，不算接受
    engine.set_input("nihooma");
    let ni = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "你")
        .unwrap();
    engine.commit(&ni);
    assert_eq!(engine.learner().typo_count("hoo", "hao"), 1);
}

#[test]
fn spelling_correction_fixes_one_edit_and_learns_from_enter() {
    let dictionary = Dictionary::parse(
            "你好吗\tni hao ma\t5000\n你好\tni hao\t9000\n你\tni\t90000\n好\thao\t80000\n吗\tma\t70000\n\
             和\the\t50000\n何\the\t3000\n咯\tlo\t100\n你猴\tni hou\t10\n",
        )
        .unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(WordList::parse("hello\n").unwrap())
        .with_learner(Box::new(CountingLearner(HashMap::new())));
    // 换错一个字母：nihooma → nihaoma，候选来自纠正后的拼音，拼音行画出被改掉的 o
    engine.set_input("nihooma");
    let query = engine.query().unwrap();
    let correction = query.correction.clone().expect("corrected");
    assert_eq!(correction.corrected, "nihaoma");
    assert_eq!(query.candidates.items[0].text, "你好吗");
    let kinds: Vec<(String, MarkedKind)> = query
        .marked_segments()
        .iter()
        .map(|s| (s.text.clone(), s.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("ni'h".to_owned(), MarkedKind::Typed),
            ("o".to_owned(), MarkedKind::Corrected),
            ("ao'ma".to_owned(), MarkedKind::Typed),
        ]
    );
    assert_eq!(query.marked_cursor(), 10);
    // 选纠正后的词吃掉整段原串，并按原输入串记选择
    let first = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&first), "你好吗");
    assert!(engine.composition().is_empty());
    engine.set_input("nihooma");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "你好吗");
    // 相邻换位
    engine.set_input("nihoama");
    let query = engine.query().unwrap();
    assert_eq!(query.correction.as_ref().unwrap().corrected, "nihaoma");
    // 选前缀词 你好 只吃到 nihoa 之后：换位不改长度，剩 ma
    let nihao = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "你好")
        .cloned()
        .unwrap();
    engine.commit(&nihao);
    assert_eq!(engine.composition().text(), "ma");
    // 整段是英文词的不纠（hello 不会变成 和咯）
    engine.set_input("hello");
    assert!(engine.query().unwrap().correction.is_none());
    // 原样已经说得通的合法简拼不纠：nhao 是 你好 的简拼，纠成 nihao 得分一样、扣掉编辑代价就输了
    engine.set_input("nhao");
    assert!(engine.query().unwrap().correction.is_none());
    // 末尾单字母只试相邻换位：nihoa → nihao
    engine.set_input("nihoa");
    assert_eq!(
        engine.query().unwrap().correction.unwrap().corrected,
        "nihao"
    );
    // 短串不纠
    engine.set_input("nih");
    assert!(engine.query().unwrap().correction.is_none());
    // 回车原样上屏过的串以后不纠
    engine.set_input("nihooma");
    assert!(engine.query().unwrap().correction.is_some());
    assert_eq!(engine.take_raw(), "nihooma");
    engine.set_input("nihooma");
    assert!(engine.query().unwrap().correction.is_none());
}

#[test]
fn transposition_is_corrected_while_the_last_syllable_is_still_unfinished() {
    let dictionary = Dictionary::parse(
        "明天\tming tian\t8000\n明\tming\t12000\n天\ttian\t9000\n米\tmi\t3000\n给\tgei\t5000\n\
         你\tni\t90000\n太\ttai\t4000\n啊\ta\t6000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary);
    // 敲反了 gn 之后还在往下敲：mignt → ming't，候选按前缀出 明天，拼音行划掉 gn
    engine.set_input("mignt");
    let query = engine.query().unwrap();
    let correction = query.correction.clone().expect("corrected");
    assert_eq!(correction.corrected, "mingt");
    assert!(correction.segmentation.last_is_partial());
    let kinds: Vec<(String, MarkedKind)> = query
        .marked_segments()
        .iter()
        .map(|s| (s.text.clone(), s.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("mi".to_owned(), MarkedKind::Typed),
            ("gn".to_owned(), MarkedKind::Corrected),
            ("ng't".to_owned(), MarkedKind::Typed),
        ]
    );
    assert_eq!(query.candidates.items[0].text, "明天");
    // 再敲两键：migntia → ming'tia…，仍是 明天；选它吃掉整段原串
    engine.set_input("migntia");
    let query = engine.query().unwrap();
    assert_eq!(query.correction.as_ref().unwrap().corrected, "mingtia");
    let first = query.candidates.items[0].clone();
    assert_eq!(first.text, "明天");
    assert_eq!(engine.commit(&first), "明天");
    assert!(engine.composition().is_empty());
    // 敲完整个词：migntian → ming'tian
    engine.set_input("migntian");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "明天");
}

#[test]
fn commit_alignment_backtracks_over_typo_variants() {
    let engine = engine();
    let syllables =
        |list: &[&str]| -> Vec<String> { list.iter().map(|s| (*s).to_owned()).collect() };
    // pingyin 上屏 拼音：pin 原样只吃三个字母会剩 gyin，退回来按敲错变体 ping → pin 吃四个，整段吃光并记敲错
    let alignment = engine.align("pingyin", &syllables(&["pin", "yin"]));
    assert_eq!(alignment.consumed, 7);
    assert_eq!(alignment.typos, [("ping".to_owned(), "pin".to_owned())]);
    // 原样、没打完的前缀、前缀词、带 ' 的输入照旧
    assert_eq!(
        engine.align("nihao", &syllables(&["ni", "hao"])).consumed,
        5
    );
    assert_eq!(
        engine
            .align("mingt", &syllables(&["ming", "tian"]))
            .consumed,
        5
    );
    assert_eq!(
        engine.align("nihaoma", &syllables(&["ni", "hao"])).consumed,
        5
    );
    assert_eq!(
        engine.align("ni'hao", &syllables(&["ni", "hao"])).consumed,
        6
    );
    // 对不上的候选还是贪心对到哪算哪
    assert_eq!(engine.align("nihao", &syllables(&["ni", "ma"])).consumed, 2);
}

#[test]
fn unfinished_last_syllable_is_not_recorded_as_a_typo() {
    let engine = engine();
    let syllables =
        |list: &[&str]| -> Vec<String> { list.iter().map(|s| (*s).to_owned()).collect() };
    // shijia 选 时间 是 jian 没敲完：jia 虽是 jian 的敲错变体，也不进个人敲错表
    for (input, list) in [
        ("shijia", ["shi", "jian"]),
        ("zhegua", ["zhe", "guan"]),
        ("woxia", ["wo", "xian"]),
    ] {
        let alignment = engine.align(input, &syllables(&list));
        assert_eq!(alignment.consumed, input.len());
        assert!(alignment.typos.is_empty(), "{input}: {:?}", alignment.typos);
    }
}

#[test]
fn fuzzy_rules_add_homophones_behind_exact_hits() {
    // 词库里只有 kai fa 系列加一个 哈；敲 kaiha 没开 f/h 时只有前缀词 开（开哈 原样读得通，词图的敲错边翻不过它），
    // 开了就出 开发（模糊命中）且覆盖更多字母排第一
    let mut engine = Engine::new(Dictionary::parse(&format!("{SAMPLE}哈\tha\t50000\n")).unwrap());
    engine.set_input("kaiha");
    let before = texts_of(&engine);
    assert!(!before.contains(&"开发".to_owned()));
    assert!(before.contains(&"开".to_owned()));
    engine.set_fuzzy(FuzzyRules {
        f_h: true,
        ..FuzzyRules::default()
    });
    let after = texts_of(&engine);
    // ha 读成 fa 的 开发 末音节对得上，排在按前缀扩展出的 开放（词频更高）前面；都覆盖更多字母排在 开 前面
    assert_eq!(after[0], "开发");
    assert!(after.contains(&"开放".to_owned()));
    assert!(after.contains(&"开".to_owned()));
    // 整句转换走同一套写法：xiangkaiha → 想开发
    engine.set_input("xiangkaiha");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "想开发");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Sentence);
    // 敲对的仍然优先：kaifa 第一位还是 开发，且不重复
    engine.set_input("kaifa");
    let all = texts_of(&engine);
    assert_eq!(all[0], "开发");
    assert_eq!(all.iter().filter(|t| *t == "开发").count(), 1);
}

#[test]
fn fuzzy_single_chars_follow_the_first_page_of_typed_chars_and_precede_rare_ones() {
    // `zi` 开 z-zh：只、知 比 zi 的字常用得多，也排在原样读音的头一页之后；但在 孖、秭 这类生僻字之前
    let typed = ["子", "字", "自", "兹", "仔", "紫", "姊", "资", "滋"];
    let mut source: String = typed
        .iter()
        .enumerate()
        .map(|(index, text)| format!("{text}\tzi\t{}\n", 9000 - index * 500))
        .collect();
    source.push_str("孖\tzi\t5\n秭\tzi\t3\n只\tzhi\t99999\n知\tzhi\t50000\n");
    // `fan` 开 an-ang：放 按前缀也查得到，同样当模糊音命中排
    source.push_str("饭\tfan\t9000\n翻\tfan\t8000\n放\tfang\t99999\n");
    let mut engine = Engine::new(Dictionary::parse(&source).unwrap());
    engine.set_fuzzy(FuzzyRules {
        z_zh: true,
        an_ang: true,
        ..FuzzyRules::default()
    });
    engine.set_input("zi");
    let all = texts_of(&engine);
    assert_eq!(&all[..9], typed);
    assert_eq!(&all[9..13], ["只", "知", "孖", "秭"]);
    engine.set_input("fan");
    assert_eq!(texts_of(&engine), ["饭", "翻", "放"]);
}

/// 原样读不通（`gan` 一个字都没有，只能拿拼音占位）时敲错边照旧排第一：`meiganxi` → 没关系。
#[test]
fn typo_edges_still_win_when_the_typed_pinyin_does_not_read() {
    let dictionary = Dictionary::parse(
        "没关系	mei guan xi	800000
关系	guan xi	500000
没	mei	900000
洗	xi	100000
",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_input("meiganxi");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "没关系");
    // 关掉拼写纠错：只按原样读，没关系 不出
    engine.set_spelling_correction(false);
    let items = engine.query().unwrap().candidates.items;
    assert!(items.iter().all(|c| c.text != "没关系"), "{items:?}");
}

/// 只认几个词的语言模型：傻逼 在补充词库，主词库没有，但模型认识它（知意的三元模型就是这样）。
struct ShabiModel;

impl LanguageModel for ShabiModel {
    fn log_prob(&self, _previous: Option<&str>, word: &str) -> Option<f64> {
        match word {
            "不" => Some(-3.0),
            "杀" => Some(-6.0),
            "比" => Some(-10.0),
            "沙" => Some(-8.0),
            "傻逼" => Some(-14.0),
            _ => None,
        }
    }
}

/// 学来的敲错也只当备选（#808）：个人敲错表里 bi → bu 记了很多次、敲错边便宜到能读出 杀不，
/// 原样读得通（补充词库里模型认识的 傻逼）时首选仍是原样，杀不 排在后面。
#[test]
fn learned_typos_do_not_override_what_was_typed() {
    let base = "杀	sha	900000
不	bu	9000000
沙	sha	300000
比	bi	800000
";
    let mut general = Dictionary::parse(
        "傻逼	sha3 bi1	5000
",
    )
    .unwrap();
    general.set_secondary(true);
    let learned = HashMap::from([("typo	bi	bu".to_owned(), 20)]);
    let mut engine = Engine::new(Dictionary::parse(base).unwrap())
        .with_language_model(Box::new(ShabiModel))
        .with_learner(Box::new(CountingLearner(learned)));
    engine.set_extra_dictionaries(vec![general]);
    engine.set_input("shabi");
    let items = engine.query().unwrap().candidates.items;
    assert_ne!(items[0].text, "杀不", "{items:?}");
    assert!(items.iter().any(|c| c.text == "杀不"), "{items:?}");
    // 关掉拼写纠错：敲错表还在，但不再起作用
    engine.set_spelling_correction(false);
    let items = engine.query().unwrap().candidates.items;
    assert!(items.iter().all(|c| c.text != "杀不"), "{items:?}");
}

/// 认识 美感 / 没关系，不认识专名 菩提伽耶 的语言模型。
struct PutiModel;

impl LanguageModel for PutiModel {
    fn log_prob(&self, _previous: Option<&str>, word: &str) -> Option<f64> {
        match word {
            "没关系" => Some(-6.0),
            "美感" => Some(-11.0),
            "洗" => Some(-9.0),
            _ => None,
        }
    }
}

/// 两种读法都带着同一个模型不认识的专名（菩提伽耶）时，它不算原样读不通的理由：只比两条不同的那段，
/// 原样的 美感洗 仍排第一，纠正的 没关系 第二（长句里一个生僻专名就让整句被「纠正」，`xintaan…` → 新法案）。
#[test]
fn words_shared_with_the_correction_do_not_make_the_typed_reading_incredible() {
    let dictionary = Dictionary::parse(concat!(
        "没关系\tmei guan xi\t800000\n美感\tmei gan\t30000\n洗\txi\t100000\n",
        "菩提伽耶\tpu ti jia ye\t200\n"
    ))
    .unwrap();
    let mut engine = Engine::new(dictionary).with_language_model(Box::new(PutiModel));
    engine.set_input("meiganxiputijiaye");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "美感洗菩提伽耶", "{items:?}");
    assert_eq!(items[1].text, "没关系菩提伽耶", "{items:?}");
}
