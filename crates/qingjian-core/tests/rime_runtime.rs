//! 真实 librime/Lua 和用户提供的雾凇目录集成测试，不使用模拟候选。
use qingjian_core::{Engine, EngineSession, RimeOptions};
use qingjian_dictionary::Dictionary;

fn options(schema: &str) -> RimeOptions {
    RimeOptions {
        library: std::env::var_os("QINGJIAN_TEST_RIME_LIBRARY")
            .expect("Rime library path")
            .into(),
        shared_data: std::env::var_os("QINGJIAN_TEST_RIME_SHARED")
            .expect("Rime Ice directory")
            .into(),
        user_data: std::env::var_os("QINGJIAN_TEST_RIME_USER")
            .expect("isolated writable directory")
            .into(),
        schema: schema.to_owned(),
        modules: Vec::new(),
    }
}

fn candidate_texts(engine: &Engine) -> Vec<String> {
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .map(|c| c.text.clone())
        .collect()
}

fn type_keys(engine: &mut Engine, input: &str) {
    engine.clear();
    for key in input.chars() {
        engine.process_rime_key(key as i32, 0).unwrap();
    }
}

#[test]
#[ignore = "requires librime with Lua and the original Rime Ice data; set QINGJIAN_TEST_RIME_* paths"]
fn original_rime_ice_runs_through_engine() {
    let mut engine = Engine::new(Dictionary::parse("").unwrap());
    eprintln!("opening Rime runtime");
    engine.enable_rime(options("rime_ice")).unwrap();
    eprintln!("Rime runtime ready");
    for (input, expected) in [
        ("nihao", "你好"),
        ("cC1+2", "3"),
        ("U4e2d", "中"),
        ("hello", "hello"),
        ("Hello", "Hello"),
        ("R123.45", "壹佰贰拾叁元肆角伍分"),
        ("v1", "1️⃣"),
        ("hm", "后面"),
    ] {
        eprintln!("testing {input}");
        type_keys(&mut engine, input);
        let query = engine.query().unwrap();
        assert!(
            query.candidates.items.iter().any(|c| c.text == expected),
            "{input}: {:?}",
            query.candidates.items
        );
        assert!(query.rime_menu.is_some());
    }
    type_keys(&mut engine, "nl");
    assert!(candidate_texts(&engine)[0].contains('年'));
    type_keys(&mut engine, "uuid");
    let uuid = &candidate_texts(&engine)[0];
    assert_eq!(uuid.len(), 36);
    assert_eq!(uuid.matches('-').count(), 4);
    type_keys(&mut engine, "uUmu");
    assert!(
        !candidate_texts(&engine).is_empty(),
        "radical reverse lookup"
    );
    engine.set_rime_option("traditionalization", true);
    type_keys(&mut engine, "shurufa");
    assert!(
        candidate_texts(&engine).iter().any(|c| c == "輸入法"),
        "matching OpenCC data required"
    );
    engine.set_rime_option("traditionalization", false);
    type_keys(&mut engine, "nihao");
    assert!(candidate_texts(&engine).iter().any(|c| c == "👋"));
    let (_, commit) = engine.process_rime_key('[' as i32, 0).unwrap();
    assert_eq!(commit.as_deref(), Some("你"), "Lua select_character");
    type_keys(&mut engine, "rq");
    assert!(
        engine.query().unwrap().candidates.items[0]
            .text
            .contains('-')
    );
    type_keys(&mut engine, "nihao");
    engine.process_rime_key(0xffe2, 1);
    engine.process_rime_key(0xffe2, 1 << 30);
    assert_eq!(engine.query().unwrap().text, "nihao");
    assert_eq!(engine.rime_option("ascii_mode"), Some(false));
    let candidate = engine.query().unwrap().candidates.items[0].clone();
    let mut other = EngineSession::default();
    engine.swap_session(&mut other);
    assert!(engine.composition().is_empty());
    assert!(
        engine.commit(&candidate).is_empty(),
        "foreign-session candidate must be rejected"
    );
    type_keys(&mut engine, "zhongguo");
    engine.swap_session(&mut other);
    assert_eq!(engine.query().unwrap().text, "nihao");
    let candidate = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(engine.commit(&candidate), "你好");
    assert!(engine.composition().is_empty());
    assert!(
        engine.commit(&candidate).is_empty(),
        "stale candidate must be rejected"
    );
    type_keys(&mut engine, "ni");
    engine.rime_page(true);
    assert_eq!(engine.query().unwrap().rime_menu.unwrap().page, 1);
    engine.rime_page(false);
    assert_eq!(engine.query().unwrap().rime_menu.unwrap().page, 0);
    engine.process_rime_key(0xffc1, 0); // F4 原生方案菜单
    assert!(!engine.query().unwrap().candidates.items.is_empty());
    engine.clear();
    assert!(
        candidate_texts(&engine).is_empty(),
        "clear must dismiss the native switcher too"
    );
    engine.set_private(true);
    assert_eq!(engine.process_rime_key('n' as i32, 0), Some((false, None)));
    assert!(engine.composition().is_empty());
    engine.set_private(false);
    engine.set_english_mode(true);
    assert_eq!(engine.process_rime_key('x' as i32, 0), Some((false, None)));
    engine.set_english_mode(false);
    for (schema, keys) in [
        ("double_pinyin", "nihk"),
        ("double_pinyin_abc", "nihk"),
        ("double_pinyin_mspy", "nihk"),
        ("double_pinyin_sogou", "nihk"),
        ("double_pinyin_flypy", "nihc"),
        ("double_pinyin_ziguang", "nihq"),
        ("double_pinyin_jiajia", "nihd"),
    ] {
        engine.enable_rime(options(schema)).unwrap();
        type_keys(&mut engine, keys);
        assert!(
            engine
                .query()
                .unwrap()
                .candidates
                .items
                .iter()
                .any(|c| c.text == "你好"),
            "{schema}: {keys}"
        );
    }
    engine.enable_rime(options("t9")).unwrap();
    type_keys(&mut engine, "64426");
    assert!(
        candidate_texts(&engine).iter().any(|c| c == "你好"),
        "T9 shares the Chinese dictionaries"
    );
    drop(other);
    drop(engine);

    // 测试只写到调用方显式指定的隔离目录下，个人 Rime profile 不作为测试目录。
    let mut custom = options("rime_ice");
    custom.user_data = custom.user_data.join("qingjian-custom-integration");
    std::fs::create_dir_all(custom.user_data.join("lua")).unwrap();
    std::fs::write(custom.user_data.join("rime_ice.custom.yaml"),
        "patch:\n  menu/page_size: 12\n  date_translator/date: qjdate\n  engine/translators/+:\n    - lua_translator@*qingjian_probe\n").unwrap();
    std::fs::write(custom.user_data.join("lua/qingjian_probe.lua"),
        "return function(input, seg)\n if input == 'qjlua' then\n yield(Candidate('qingjian', seg.start, seg._end, '青简 Lua', '用户脚本'))\n end\nend\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse("").unwrap());
    engine.enable_rime(custom).unwrap();
    type_keys(&mut engine, "qjlua");
    let query = engine.query().unwrap();
    let probe = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "青简 Lua")
        .unwrap();
    assert_eq!(probe.rime.as_ref().unwrap().comment, "用户脚本");
    type_keys(&mut engine, "qjdate");
    assert!(
        candidate_texts(&engine)[0].contains('-'),
        "custom YAML changes Lua triggers"
    );
    type_keys(&mut engine, "ni");
    let query = engine.query().unwrap();
    assert_eq!(query.rime_menu.unwrap().page_size, 12);
    assert_eq!(
        query.candidates.items.len(),
        12,
        "native page must not be clamped to nine items"
    );
}
