//! 问字模式：部件读音、笔画、两者混着打；候选带读音；码点与英文词照旧。

use super::*;

fn first_texts(engine: &Engine, n: usize) -> Vec<String> {
    texts_of(engine).into_iter().take(n).collect()
}

#[test]
fn part_readings_find_the_character_with_its_reading() {
    let mut engine = engine();
    engine.set_input("umumu");
    let query = engine.query().unwrap();
    let lin = &query.candidates.items[0];
    assert_eq!(lin.text, "林");
    assert_eq!(lin.reading.as_deref(), Some("lín"));
    assert_eq!(query.marked_text(), "umu'mu");
    // 讠舌 也读 yan she：同一档按字频，话 在 谢 前
    engine.set_input("uyanshe");
    assert_eq!(first_texts(&engine, 2), ["话", "谢"]);
    engine.set_input("uniuniuniu");
    assert_eq!(first_texts(&engine, 1), ["犇"]);
    // 敲到一半也出：前缀 mumu 的 森（mumumu）跟在 林 后面
    engine.set_input("umumu");
    assert!(texts_of(&engine).contains(&"森".to_owned()));
}

/// 整字笔顺：`木` 是 `hspn`（横竖撇捺），`林` 是两遍。
#[test]
fn strokes_find_the_character() {
    let mut engine = engine();
    engine.set_input("uhspn");
    assert_eq!(first_texts(&engine, 1), ["木"]);
    engine.set_input("uhspnhspn");
    assert_eq!(first_texts(&engine, 1), ["林"]);
    // 打一半也出，按「还差几笔」排
    engine.set_input("uhsp");
    assert_eq!(first_texts(&engine, 1), ["木"]);
}

/// 倒笔画：整串反过来打，`木` 是 `npsh`。
#[test]
fn reversed_strokes_find_the_character() {
    let mut engine = engine();
    engine.set_input("unpsh");
    assert_eq!(first_texts(&engine, 1), ["木"]);
    // 林 = hspnhspn，倒过来是 npshnpsh
    engine.set_input("unpshnpsh");
    assert_eq!(first_texts(&engine, 1), ["林"]);
}

/// 混输：一个部件按读音、另一个按笔画。
#[test]
fn readings_and_strokes_mix_within_one_input() {
    let mut engine = engine();
    // 木(mu) + 木的笔顺
    engine.set_input("umuhspn");
    assert_eq!(first_texts(&engine, 1), ["林"]);
    // 禾(he) + 口(szh)
    engine.set_input("uheszh");
    assert_eq!(first_texts(&engine, 1), ["和"]);
    // 第一个部件按笔顺、第二个按读音：禾(phspn) + 口(kou)
    engine.set_input("uphspnkou");
    assert_eq!(first_texts(&engine, 1), ["和"]);
}

/// 部件的俗名：囗 的字表读音是 wei，用户按 kou 打也要认。
#[test]
fn part_alias_reads_the_shape() {
    let mut engine = engine();
    engine.set_input("ukoumu");
    assert_eq!(first_texts(&engine, 1), ["困"]);
    engine.set_input("ukouhspn");
    assert_eq!(first_texts(&engine, 1), ["困"]);
    // 本来的读音照旧
    engine.set_input("uweimu");
    assert_eq!(first_texts(&engine, 1), ["困"]);
}

/// 繁体拆法并进了简体主键：打台北字形的拆法（辶 言）也能找到简体字。
#[test]
fn traditional_splits_find_the_simplified_character() {
    let mut engine = engine();
    // 这 = 辶 文（简体字形）或 辶 言（繁体字形），两种都出同一个字
    engine.set_input("uzouyan");
    assert_eq!(first_texts(&engine, 1), ["这"]);
    engine.set_input("uzouwen");
    assert_eq!(first_texts(&engine, 1), ["这"]);
}

#[test]
fn hex_looking_letters_select_instead_of_typing_a_code_point() {
    let mut engine = engine();
    engine.set_input("udada");
    assert!(!engine.unicode_entry());
    assert_eq!(first_texts(&engine, 1), ["夶"]);
    // 有数字的仍是码点
    engine.set_input("u4e00");
    assert!(engine.unicode_entry());
    assert_eq!(first_texts(&engine, 1), ["一"]);
    // 拆不出字的十六进制字母仍可能是码点：`face` 切得出部件读音，但那是拼音不是拆字
    engine.set_input("uface");
    assert!(engine.unicode_entry());
}

#[test]
fn english_words_stay_first() {
    let words = WordList::parse("under\tunder\t3000\n").unwrap();
    let mut engine = engine().with_english(words);
    engine.set_input("under");
    assert_eq!(first_texts(&engine, 1), ["under"]);
}

/// 整段都读得通的时候，按读音出的字排在混输前面。
///
/// `炭` 是 `s` 当一笔竖 + `hui` 当 灰 拼出来的，两个部件都走完了；`没` 只走了 氵。
/// 之前「走完没走完」比档位靠前，`ushui` 第一变成了 炭。
#[test]
fn readings_beat_mixed_splits_when_the_whole_input_reads() {
    let mut engine = engine();
    engine.set_input("ushui");
    assert_eq!(first_texts(&engine, 1), ["没"]);
    let all = texts_of(&engine);
    assert!(all.iter().position(|t| t == "法").is_some_and(|at| at < 8));
    engine.set_input("ujin");
    assert_eq!(first_texts(&engine, 1), ["钱"]);
    engine.set_input("ushou");
    assert_eq!(first_texts(&engine, 1), ["我"]);
}

/// 只打第一个部件读音时，繁体字形拆法带进来的字不该压过简体拆法的常用字。
///
/// `了` 的 `目尞` 是繁体 瞭 的拆法（繁体表并进简体主键），`umu` 只走到 目 就停了；
/// `本` 是 木一、`林` 是 木木，都是简体拆法。改之前 `umu` 第一是 了。
#[test]
fn partial_rereads_from_traditional_splits_rank_after_simplified_ones() {
    let mut engine = engine();
    engine.set_input("umu");
    let all = texts_of(&engine);
    let at = |text: &str| all.iter().position(|t| t == text);
    assert!(at("本").is_some_and(|i| i < 4), "本 应当靠前：{all:?}");
    assert!(at("林").is_some_and(|i| i < 8), "林 应当靠前：{all:?}");
    // 了 是由 目尞 匹配到的，靠后
    assert!(at("了").is_none_or(|i| i > at("本").unwrap()));
}

/// 读音打一半：最后一个部件（`ushangxia` 的 叔 = 上小又，`xia` 是 `xiao` 的前半截），
/// 以及中间部件（`ushangxiyou` 的 小 打成 `xi`）都要能接上。
#[test]
fn half_typed_readings_find_the_character() {
    let mut engine = engine();
    engine.set_input("ushangxia");
    assert_eq!(first_texts(&engine, 1), ["卡"]);
    let all = texts_of(&engine);
    // 叔（上小又）只看 `xia` 对 `xiao` 的前缀也能出
    assert!(all.contains(&"叔".to_owned()), "{all:?}");
    assert!(all.contains(&"尗".to_owned()), "{all:?}");

    // 中间那个部件打一半，后面的部件照样接得上
    engine.set_input("ushangxiyou");
    assert_eq!(first_texts(&engine, 1), ["叔"]);
}

/// ü 的两种敲法都认：`yve` / `yue`、`jvan` / `juan`、`xve` / `xue`。
#[test]
fn umlaut_accepts_both_spellings() {
    let mut engine = engine();
    for input in ["uriyue", "uriyve"] {
        engine.set_input(input);
        assert_eq!(first_texts(&engine, 1), ["明"], "{input}");
    }
    for input in ["ujuan", "ujvan"] {
        engine.set_input(input);
        assert_eq!(first_texts(&engine, 1), ["卷"], "{input}");
    }
    for input in ["uxue", "uxve"] {
        engine.set_input(input);
        assert_eq!(first_texts(&engine, 1), ["穿"], "{input}");
    }
    // l、n 后面表里写 `ve`，用户敲 `ue` 也要认
    engine.set_input("ulue");
    let with_ue = texts_of(&engine);
    engine.set_input("ulve");
    assert!(!with_ue.is_empty(), "lue 不该一条都没有");
}

/// 简体表里漏着的繁体字形要并到简体主键：`內` → `内`、`円` → `圆`。
#[test]
fn traditional_glyphs_inside_the_simplified_table_are_merged() {
    let mut engine = engine();
    for input in ["usz", "ush"] {
        engine.set_input(input);
        let all = texts_of(&engine);
        assert!(
            !all.contains(&"內".to_owned()),
            "{input} 出了繁体字形：{all:?}"
        );
        assert!(
            !all.contains(&"円".to_owned()),
            "{input} 出了繁体字形：{all:?}"
        );
        // 同一个字不该出现两次
        let mut seen = std::collections::BTreeSet::new();
        for text in &all {
            assert!(seen.insert(text.clone()), "{input} 重复候选：{text}");
        }
    }
}

/// 弱匹配不出候选：`uface`、`usangemu` 是要问云端的，本地不该冒出几个不相干的字。
#[test]
fn weak_matches_produce_no_candidates() {
    let mut engine = engine();
    for input in ["uface", "usangemu"] {
        engine.set_input(input);
        assert!(
            texts_of(&engine).is_empty(),
            "{input} 不该有本地候选：{:?}",
            texts_of(&engine)
        );
    }
}

/// 最基础的一笔：`uh`/`us`/`up`/`un` 正好也是英文词（uh/us/up/UN），
/// 但它们是笔画输入最常用的入口，不能因为撞上英文词就一条字都不给。
#[test]
fn a_single_stroke_still_gives_the_character() {
    let mut engine = engine();
    for (input, expected) in [("uh", "一"), ("us", "丨"), ("up", "丿"), ("un", "丶")] {
        engine.set_input(input);
        let all = texts_of(&engine);
        assert!(
            all.contains(&expected.to_owned()),
            "{input} 该出 {expected}：{all:?}"
        );
    }
    // 英文词排在前面这一条由 english_words_stay_first 管（这里没有英文词表）
}
/// 长英文串不该带出一串按部件凑出来的字：弱匹配由 `lookup` 的门槛挡着。
#[test]
fn a_long_english_string_stays_clean() {
    let mut engine = engine();
    for input in ["unchanged", "uabcd", "udeadbeef"] {
        engine.set_input(input);
        assert!(
            texts_of(&engine).is_empty(),
            "{input} 不该有拆字候选：{:?}",
            texts_of(&engine)
        );
    }
}
