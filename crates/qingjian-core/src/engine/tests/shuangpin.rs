//! 双拼。

use super::*;

#[test]
fn shuangpin_decodes_keys_before_lookup_and_shows_full_pinyin() {
    let mut engine = xiaohe();
    engine.set_input("kdfa");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "开发");
    assert_eq!(query.marked_text(), "kai'fa");
    assert_eq!(query.marked_cursor(), 6);
    assert_eq!(query.text, "kdfa");
    assert!(query.decoded_keys);
    // 末尾落单的键是声母前缀：`kdf` = kai f…
    engine.set_input("kdf");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'f");
    assert_eq!(query.candidates.items[0].text, "开放");
    // 配不出音节的键原样留作尾巴
    engine.set_input("kdfl");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'fl");
    assert_eq!(query.tail, "fl");
    assert_eq!(query.candidates.items[0].text, "开");
}

#[test]
fn shuangpin_shows_raw_keys_in_preedit_and_decoded_pinyin_in_segments() {
    let mut engine = xiaohe();
    engine.set_shuangpin_raw_preedit(true);
    engine.set_input("kdfa");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "开发");
    // 输入框内 marked_text 显示原始输入按键
    assert_eq!(query.marked_text(), "kdfa");
    assert_eq!(query.marked_cursor(), 4);
    assert_eq!(query.text, "kdfa");
    // 候选窗口顶部拼音行保持显示解码全拼
    let segments_text: String = query
        .marked_segments()
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(segments_text, "kai'fa");
    assert_eq!(query.segments_cursor(), 6);

    // 移动光标到中间 kd|fa
    engine.move_cursor_left();
    engine.move_cursor_left();
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kdfa");
    assert_eq!(query.marked_cursor(), 2);
    let segments_text: String = query
        .marked_segments()
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(segments_text, "kai'fa");
    assert_eq!(query.segments_cursor(), 3);

    // k|dfa：光标后的剩余段保留原始按键，和敲的对得上（双拼光标插进音节中间，单独解码会错开）
    engine.move_cursor_left();
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kdfa");
    assert_eq!(query.marked_cursor(), 1);
    let segments_text: String = query
        .marked_segments()
        .iter()
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(segments_text, "k'dfa");
}

/// 两个开关各管一处：输入框按 `shuangpin_raw_preedit`，候选窗口的拼音行按 `shuangpin_raw_window`。
#[test]
fn shuangpin_window_and_preedit_switch_independently() {
    let window = |query: &Query| {
        let (segments, cursor) = query.window_segments(None);
        let text: String = segments.iter().map(|s| s.text.as_str()).collect();
        (text, cursor)
    };
    let mut engine = xiaohe();
    engine.set_input("kdfa");
    for (raw_preedit, raw_window, marked, line) in [
        (false, false, "kai'fa", ("kai'fa", 6)),
        (true, true, "kdfa", ("kdfa", 4)),
        (false, true, "kai'fa", ("kdfa", 4)),
        (true, false, "kdfa", ("kai'fa", 6)),
    ] {
        engine.set_shuangpin_raw_preedit(raw_preedit);
        engine.set_shuangpin_raw_window(raw_window);
        let query = engine.query().unwrap();
        assert_eq!(query.marked_text(), marked, "{raw_preedit} {raw_window}");
        assert_eq!(
            window(&query),
            (line.0.to_owned(), line.1),
            "{raw_preedit} {raw_window}"
        );
    }

    // kd|fa：光标前后的键分两段，光标后的那段按 Rest 画
    engine.set_shuangpin_raw_window(true);
    engine.move_cursor_left();
    engine.move_cursor_left();
    let query = engine.query().unwrap();
    let (segments, cursor) = query.window_segments(None);
    let kinds: Vec<(&str, MarkedKind)> =
        segments.iter().map(|s| (s.text.as_str(), s.kind)).collect();
    assert_eq!(kinds, [("kd", MarkedKind::Typed), ("fa", MarkedKind::Rest)]);
    assert_eq!(cursor, 2);

    // 全拼不受影响
    let mut engine = super::engine();
    engine.set_shuangpin_raw_window(true);
    engine.set_input("kaifa");
    assert_eq!(window(&engine.query().unwrap()).0, "kai'fa");
}

#[test]
fn shuangpin_commit_consumes_keys_per_syllable() {
    let mut engine = xiaohe();
    engine.set_input("kdfave");
    let query = engine.query().unwrap();
    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .cloned()
        .unwrap();
    engine.commit(&kaifa);
    assert_eq!(engine.composition().text(), "ve");
    assert_eq!(engine.query().unwrap().marked_text(), "开发zhe");
    // 未打完的最后一个键也被候选吃掉
    engine.set_input("kdf");
    let kaifang = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&kaifang);
    assert!(engine.composition().is_empty());
}

/// 开了「双拼兼容全拼」：双拼方案下全拼也认，还能在同一段输入里混用（#374）。
fn mixed_engine() -> Engine {
    let mut engine = xiaohe();
    engine.set_shuangpin_full_pinyin(true);
    engine
}

fn mixed_texts(engine: &mut Engine, input: &str) -> Vec<String> {
    engine.set_input(input);
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect()
}

#[test]
fn full_pinyin_option_accepts_full_pinyin_under_a_shuangpin_scheme() {
    // 关着时 `kaifa` 只有双拼一种读法（ka|chen|a…），出不来 开发
    let mut engine = xiaohe();
    engine.set_input("kaifa");
    let plain: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert!(!plain.contains(&"开发".to_owned()));
    // 开着时整段按全拼读，`kaifa` 就是 开发
    let mut engine = mixed_engine();
    assert_eq!(mixed_texts(&mut engine, "kaifa")[0], "开发");
    // 纯双拼照旧
    assert_eq!(mixed_texts(&mut engine, "kdfa")[0], "开发");
}

#[test]
fn full_pinyin_option_mixes_both_ways_in_one_input() {
    // 前半全拼 kai、后半双拼 fh（= fang）：只有混输能解
    let mut engine = xiaohe();
    engine.set_input("kaifh");
    let plain: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert!(!plain.contains(&"开放".to_owned()));
    let mut engine = mixed_engine();
    assert_eq!(mixed_texts(&mut engine, "kaifh")[0], "开放");
}

#[test]
fn full_pinyin_option_keeps_both_readings_of_xian() {
    // 小鹤下 xi'an 是双拼的定局；开着混输时全拼的 xian（先）也进来
    let mut engine = xiaohe();
    engine.set_input("xian");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.text != "先")
    );
    let mut engine = mixed_engine();
    let all = mixed_texts(&mut engine, "xian");
    assert!(all.contains(&"先".to_owned()));
    assert!(all.contains(&"西安".to_owned()));
}

#[test]
fn full_pinyin_option_works_with_a_custom_shuangpin_layout() {
    // 用户方案文件夹读出的键位表与内置方案是同一个 `Layout`：把 ai 挪到 t，混输照样能用
    let mut layout = Scheme::Xiaohe.layout().clone();
    layout.set_finals('t', vec!["ai".to_owned()]);
    let mut engine = engine();
    engine.set_shuangpin_layout(Some(std::sync::Arc::new(layout)));
    engine.set_shuangpin_full_pinyin(true);
    // 自定义键位：ktfa 读成 kai'fa
    assert_eq!(mixed_texts(&mut engine, "ktfa")[0], "开发");
    // 全拼照常出候选
    assert_eq!(mixed_texts(&mut engine, "kaifa")[0], "开发");
}

#[test]
fn full_pinyin_option_ignores_partial_readings_and_short_prefixes() {
    // 混输不能拿「只解出开头一个音节」的读法当家，也不能让别的读法的短前缀把学习记录套到整段输入上
    let dict = "版本\tban ben\t9000\n吧\tba\t20000\n多\tduo\t20000\n都\tdou\t10000\n";
    let mut engine = Engine::new(Dictionary::parse(dict).unwrap())
        .with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_shuangpin_full_pinyin(true);
    // 先选过 ba → 吧，再打 banben：版本 仍是首选
    engine.set_input("ba");
    let ba = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(ba.text, "吧");
    engine.commit(&ba);
    engine.set_input("banben");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "版本");
    // doubudui 不被只解出 duo 的那半条读法挤成 多
    engine.set_input("doubudui");
    assert_ne!(engine.query().unwrap().candidates.items[0].text, "多");
}

#[test]
fn full_pinyin_option_handles_separators_and_keeps_whole_coverage() {
    // 显式 `'`、末尾多敲一个 `'`、只解出开头的读法、整段覆盖量的单位，都要与关掉混输时的语义一致
    let dict = "版本\tban ben\t1000\n坂本\tban ben\t5000\n吧\tba\t2000\n您\tnin\t5000\n嗯\ten\t5000\n多\tduo\t20000\n躲\tduo\t1000\n朵\tduo\t500\n都\tdou\t10000\n产\tchan\t1000\n仓\tcang\t1000\n安\tan\t1000\n";
    let mut engine = Engine::new(Dictionary::parse(dict).unwrap())
        .with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_shuangpin_full_pinyin(true);
    // 不带 `'` 时选过 版本，再打带 `'` 的同一段输入，选择记录要照常生效（坂本词频更高）
    engine.set_input("banben");
    let version = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "版本")
        .unwrap();
    engine.commit(&version);
    engine.set_input("ban'ben");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "版本");
    // 末尾多一个分隔符，全拼读法不少算键，主切分不被双拼读法（吧您嗯）抢走
    engine.set_input("banben'");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "版本");
    // 只解出开头一个音节的读法不参与：多 / 躲 / 朵 不进前三
    engine.set_input("doubudui");
    let first3: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .take(3)
        .map(|c| c.text.clone())
        .collect();
    assert!(
        !first3
            .iter()
            .any(|text| ["多", "躲", "朵"].contains(&text.as_str())),
        "{first3:?}"
    );
    // 整段能读完整时，全拼读法的覆盖量按敲的键算，不被更长的双拼字母数（cang an）挤进前缀桶
    engine.set_input("chan");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "产");
}

#[test]
fn full_pinyin_option_keeps_english_words_behind_chinese_sentences() {
    // `women` / `beijing` 同时是英文词：双拼读法把整段读成半截（wo'me'n…、bei'ji'neng）时，
    // 不能因此判成「不像拼音」把英文词排到 我们 / 北京 前面
    let dict = "我们\two men\t9000\n北京\tbei jing\t9000\n";
    let mut engine = Engine::new(Dictionary::parse(dict).unwrap())
        .with_english(WordList::parse("women\twomen\t3000\nbeijing\tbeijing\t3000\n").unwrap());
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_shuangpin_full_pinyin(true);
    engine.set_input("women");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "我们");
    engine.set_input("beijing");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "北京");
}

#[test]
fn full_pinyin_option_commits_consuming_the_typed_keys() {
    let mut engine = mixed_engine();
    engine.set_input("kaifa");
    let kaifa = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(kaifa.text, "开发");
    engine.commit(&kaifa);
    assert!(engine.composition().is_empty());
    // 学习键是全拼，与双拼打法共用一份
    assert!(engine.recent_commits.last().unwrap().same_input("kaifa"));
    // 混输下打双拼也照旧吃掉两键一音节
    engine.set_input("kdfa");
    let kaifa = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&kaifa);
    assert!(engine.composition().is_empty());
}

#[test]
fn shuangpin_records_choices_by_full_pinyin() {
    let mut engine = xiaohe();
    engine.set_input("kdfa");
    let query = engine.query().unwrap();
    // 两键定局：`fa` 不再按前缀放宽成 fan / fang
    assert!(query.candidates.items.iter().all(|c| c.text != "开饭"));
    let kaifa = query.candidates.items[0].clone();
    assert_eq!(kaifa.text, "开发");
    engine.commit(&kaifa);
    // 学习记的是全拼 kaifa：切回全拼、同样的拼音也受益
    assert!(engine.recent_commits.last().unwrap().same_input("kaifa"));
}

#[test]
fn shuangpin_moves_mode_keys_to_shifted_letters() {
    let mut engine = xiaohe();
    engine.set_input("v");
    assert!(!engine.expression_mode());
    assert_eq!(engine.query().unwrap().marked_text(), "zh");
    engine.set_input("u1");
    assert!(!engine.question_mode());
    // Shift+V / Shift+U 进模式，之后与全拼下的 v / u 一样
    assert!(engine.takes_mode_letter('V') && engine.takes_mode_letter('U'));
    assert!(!engine.takes_mode_letter('v') && !engine.takes_mode_letter('I'));
    engine.set_input("V1+2");
    assert!(engine.expression_mode() && !engine.raw_mode());
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "3");
    engine.set_input("U4e00");
    assert!(engine.question_mode() && engine.unicode_entry());
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "一");
    engine.set_input("Unihc");
    assert!(engine.question_mode());
    assert_eq!(engine.query().unwrap().marked_text(), "Uni'hao");
    // 全拼下大写字母不是入口
    let mut full = super::engine();
    assert!(!full.takes_mode_letter('V'));
    full.set_input("V1+2");
    assert!(!full.expression_mode());
    // `?` 入口开着时照常进问字（缺省关）
    engine.set_input("?nihc");
    assert!(!engine.question_mode());
    engine.set_mode_keys(ModeKeys {
        question_mark: true,
        ..ModeKeys::default()
    });
    engine.set_input("?nihc");
    assert!(engine.question_mode());
    assert_eq!(engine.query().unwrap().marked_text(), "?ni'hao");
}

#[test]
fn microsoft_semicolon_is_a_final_only_after_a_lone_initial() {
    let mut engine = engine();
    engine.set_shuangpin(Some(Scheme::Microsoft));
    engine.set_input("x");
    assert!(engine.takes_semicolon());
    engine.push(';');
    assert!(!engine.takes_semicolon());
    assert!(!engine.raw_mode());
    assert_eq!(engine.query().unwrap().marked_text(), "xing");
    // 问字模式里也认：`?x;` 问的是 xing
    engine.set_mode_keys(ModeKeys {
        question_mark: true,
        ..ModeKeys::default()
    });
    engine.set_input("?x");
    assert!(engine.takes_semicolon());
    engine.push(';');
    assert!(engine.question_mode());
    assert_eq!(engine.query().unwrap().marked_text(), "?xing");
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_input("x");
    assert!(!engine.takes_semicolon());
}

#[test]
fn shuangpin_raw_commit_does_not_learn_decodable_keys_as_english() {
    let mut engine = xiaohe();
    engine.set_input("nihc");
    assert!(!looks_like_english_word_in(&engine));
    engine.set_input("gist");
    assert!(looks_like_english_word_in(&engine));
}

#[test]
fn unmatched_keys_get_abbreviated_candidates() {
    // 拼音加加：`im` / `zm` 配不出任何音节，按每键一个声母读 → 什么 / 怎么
    let dictionary =
        Dictionary::parse("什么\tshen me\t9000\n怎么\tzen me\t8000\n说明\tshuo ming\t500\n")
            .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_shuangpin(Some(Scheme::Jiajia));
    engine.set_input("im");
    assert_eq!(texts_of(&engine)[0], "什么");
    engine.set_input("zm");
    assert_eq!(texts_of(&engine)[0], "怎么");
}

#[test]
fn pending_key_that_spells_a_whole_word_keeps_english_out_of_first() {
    // 拼音加加：`veg` = zhe g…（末尾落单的一键），词级已有「这个」逐音节对上全部输入
    let word_of = |dict: &str| {
        let mut engine = Engine::new(Dictionary::parse(dict).unwrap())
            .with_english(WordList::parse("veg\tveg\t500\n").unwrap());
        engine.set_shuangpin(Some(Scheme::Jiajia));
        engine.set_input("veg");
        texts_of(&engine)
    };
    let whole = word_of("这个\tzhe ge\t9000\n");
    assert_eq!(whole[0], "这个", "{whole:?}");
    // 英文词还在，只是让到后面：空格不会再把它整串上屏
    assert!(whole.iter().any(|text| text == "veg"), "{whole:?}");
    // 只有单字、没有整词覆盖全部音节时不改：那串更像在打英文
    let single = word_of("遮\tzhe\t9000\n歌\tge\t8000\n");
    assert_eq!(single[0], "veg", "{single:?}");
}

/// 模糊音命中时末音节要吃满它的键：`cuqk` 解成 `cuqing`，选 出勤（chu qin）不能因为
/// `qin` 原样能对上 `qing` 的前三个字母就只吃三个，否则 `keys_for` 向下取整，`qk` 留在缓冲区。
#[test]
fn fuzzy_final_syllable_consumes_all_its_keys() {
    let dictionary = Dictionary::parse("出勤\tchu qin\t1000\n").unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_fuzzy(FuzzyRules {
        c_ch: true,
        in_ing: true,
        ..FuzzyRules::default()
    });
    engine.set_input("cuqk");
    let chuqin = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "出勤")
        .unwrap();
    engine.commit(&chuqin);
    assert!(
        engine.composition().is_empty(),
        "残留：{}",
        engine.composition().text()
    );
}
