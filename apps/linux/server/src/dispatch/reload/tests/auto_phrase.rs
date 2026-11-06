//! 自动组词设置热加载后，两条学习规则使用相同配置。

use super::{poll, temp_dir, watched, write_config};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_learning::FrequencyLearner;

fn commit(engine: &mut Engine, code: &str, text: &str) {
    engine.set_input(code);
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|candidate| candidate.text == text)
        .unwrap();
    engine.commit(&candidate);
}

#[test]
fn auto_phrase_switch_and_length_reload_without_reloading_scheme() {
    let directory = temp_dir("auto-phrase");
    let scheme_dir = directory.join("assets/schemes/wubi86");
    std::fs::create_dir_all(&scheme_dir).unwrap();
    std::fs::write(
        scheme_dir.join("scheme.toml"),
        "phrase_rule = \"wubi86\"\nauto_commit = false\n",
    )
    .unwrap();
    std::fs::write(
        scheme_dir.join("table.tsv"),
        "甲\tabcd\t10\n乙\tefgh\t10\n丙\tijkl\t10\n",
    )
    .unwrap();
    let pure = "[general]\nscheme = \"none\"\ncode = \"wubi86\"\n";
    let mut router = watched(&directory, pure);
    let engine = std::mem::replace(&mut router.engine, Engine::new(Dictionary::default()));
    router.engine = engine.with_learner(Box::<FrequencyLearner>::default());
    assert!(router.engine.code_scheme().unwrap().wubi_phrases);
    write_config(
        &directory.join("config.toml"),
        &format!("{pure}[auto_phrase]\nenabled = false\nmax_chars = 2\n"),
        200,
    );
    poll(&mut router);
    commit(&mut router.engine, "abcd", "甲");
    commit(&mut router.engine, "efgh", "乙");
    router.engine.set_input("abef");
    assert!(router.engine.query().unwrap().candidates.items.is_empty());

    write_config(
        &directory.join("config.toml"),
        &format!("{pure}[auto_phrase]\nenabled = true\nmax_chars = 2\n"),
        300,
    );
    poll(&mut router);
    router.engine.clear();
    commit(&mut router.engine, "abcd", "甲");
    commit(&mut router.engine, "efgh", "乙");
    commit(&mut router.engine, "ijkl", "丙");
    router.engine.set_input("abef");
    assert!(
        router
            .engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|candidate| candidate.text == "甲乙")
    );
    router.engine.set_input("aeij");
    assert!(
        !router
            .engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|candidate| candidate.text == "甲乙丙")
    );
    std::fs::remove_dir_all(directory).unwrap();
}
