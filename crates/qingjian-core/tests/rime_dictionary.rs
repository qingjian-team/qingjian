//! 用户导入的拼音词库经现有附加词库入口参与全拼和双拼候选。

use qingjian_core::{Engine, ShuangpinScheme};
use qingjian_dictionary::{Dictionary, import::import};

#[test]
fn imported_rime_words_are_available_in_full_pinyin_and_xiaohe() {
    let dir = std::env::temp_dir().join(format!("qingjian-rime-engine-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("main.dict.yaml");
    std::fs::write(
        &source,
        "---\nname: main\nimport_tables: [words]\n...\n开\tkai\t100\n发\tfa\t100\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("words.dict.yaml"),
        "---\nname: words\ncolumns: [text, weight]\n...\n开发\t9000\n",
    )
    .unwrap();
    let imported = import(&source, &dir.join("dicts")).unwrap();
    let dictionary = Dictionary::from_path(&imported.path).unwrap();
    let mut engine = Engine::new(Dictionary::parse("").unwrap());
    engine.set_extra_dictionaries(vec![dictionary]);
    for (scheme, input) in [(None, "kaifa"), (Some(ShuangpinScheme::Xiaohe), "kdfa")] {
        engine.set_shuangpin(scheme);
        engine.set_input(input);
        let query = engine.query().unwrap();
        assert!(
            query
                .candidates
                .items
                .iter()
                .any(|candidate| candidate.text == "开发")
        );
    }
    drop(engine);
    std::fs::remove_dir_all(dir).unwrap();
}
