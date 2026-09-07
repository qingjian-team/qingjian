//! 拼写纠错 / 敲错边 / 模糊音。

use super::*;

/// 每个音节都合法、整句却不通的输入（`meiganxi`）：词图里的敲错边把 没关系 读出来，作为词候选插到最前；
/// 上屏按敲的字母消耗、记个人敲错表，退格重选时退回；原样说得通（`meiganxie` 没感谢）时不改。
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
    let first = query.candidates.items[0].clone();
    assert_eq!(first.text, "没关系");
    assert_eq!(first.kind, CandidateKind::Chinese);
    assert_eq!(first.syllables, ["mei", "guan", "xi"]);
    // 词级候选不带敲错变体：原样的 美感 还在后面
    assert!(query.candidates.items.iter().any(|c| c.text == "美感"));
    assert_eq!(engine.commit(&first), "没关系");
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
    // 两个音节也纠：ganxi → 关系
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
    // ha 是前缀，换成 fa 前缀后 开放（词频更高）与 开发 都出，覆盖更多字母排在 开 前面
    assert_eq!(after[0], "开放");
    assert!(after.contains(&"开发".to_owned()));
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
