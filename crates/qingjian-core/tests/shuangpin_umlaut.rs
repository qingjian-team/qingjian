//! 双拼 ü 键的拼音规范化，以及查询、上屏消耗的回归。

use qingjian_core::{Engine, ShuangpinScheme as Scheme};
use qingjian_dictionary::Dictionary;

#[test]
fn umlaut_keys_decode_to_standard_pinyin() {
    for (scheme, key) in [
        (Scheme::Sogou, 'y'),
        (Scheme::Microsoft, 'y'),
        (Scheme::Xiaohe, 'v'),
        (Scheme::Ziranma, 'v'),
        (Scheme::Abc, 'v'),
        (Scheme::Shoudao, 'v'),
        (Scheme::Xiaolang, 'x'),
    ] {
        for (initial, expected) in [('j', "ju"), ('q', "qu"), ('x', "xu"), ('y', "yu")] {
            let keys = format!("{initial}{key}");
            let decoded = scheme.decode(&keys);
            assert_eq!(decoded.pinyin(), expected, "{scheme}: {keys}");
            assert!(decoded.is_complete(), "{scheme}: {keys}");
        }
        for (initial, expected) in [('n', "nv"), ('l', "lv")] {
            let keys = format!("{initial}{key}");
            assert_eq!(scheme.decode(&keys).pinyin(), expected, "{scheme}: {keys}");
        }
    }
    // 同键的合法读法保持优先，不能把微软的 jue 或小浪的 jing 改成 ju。
    assert_eq!(Scheme::Microsoft.decode("jv").pinyin(), "jue");
    assert_eq!(Scheme::Xiaolang.decode("jv").pinyin(), "jing");
    assert_eq!(Scheme::Sogou.decode("gy").pinyin(), "guai");
    assert_eq!(Scheme::Sogou.decode("gu").pinyin(), "gu");
    assert_eq!(Scheme::Sogou.decode("ju").pinyin(), "ju");
    assert_eq!(Scheme::Sogou.decode("lv").tail(), "lv");
}

#[test]
fn sogou_umlaut_keys_query_and_commit_by_syllable() {
    let dictionary = Dictionary::parse(
        "句\tju\t9000\n去\tqu\t8000\n需\txu\t7000\n鱼\tyu\t6000\n女\tnv\t5000\n绿\tlv\t4000\n星\txing\t3000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_shuangpin(Some(Scheme::Sogou));
    for (keys, pinyin, word) in [
        ("jy", "ju", "句"),
        ("qy", "qu", "去"),
        ("xy", "xu", "需"),
        ("yy", "yu", "鱼"),
        ("ny", "nv", "女"),
        ("ly", "lv", "绿"),
    ] {
        engine.set_input(&format!("{keys}x;"));
        let query = engine.query().unwrap();
        assert_eq!(query.marked_text(), format!("{pinyin}'xing"));
        let candidate = query
            .candidates
            .items
            .iter()
            .find(|candidate| candidate.text == word)
            .unwrap();
        engine.commit(candidate);
        assert_eq!(engine.composition().text(), "x;");
        assert_eq!(engine.query().unwrap().marked_text(), "xing");
    }
}
