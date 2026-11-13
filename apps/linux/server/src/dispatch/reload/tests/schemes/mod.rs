//! 方案装配、回退与配置热更新。

use super::{poll, temp_dir, watched, write_config};
use crate::dispatch::Router;
use qingjian_core::{CandidateKind, Engine};
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    ClientMessage, KeyEvent, KeyModifiers, PROTOCOL_VERSION, SessionId,
};
use qingjian_platform::{Config, UserDirs};
use std::path::Path;

mod lifecycle;

const PURE: &str = "[general]\nscheme = \"none\"\ncode = \"wubi86\"\n";

fn scheme(dir: &Path, bundled: bool, table: &str) {
    let path = dir.join(if bundled {
        "data/generated/schemes/wubi86"
    } else {
        "custom/schemes/wubi86"
    });
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(
        path.join("scheme.toml"),
        "name = \"五笔测试\"\ncode_length = 4\n",
    )
    .unwrap();
    std::fs::write(path.join("table.tsv"), table).unwrap();
}

fn texts(router: &mut Router, input: &str) -> Vec<String> {
    router.engine.set_input(input);
    router
        .engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|candidate| candidate.text)
        .collect()
}

fn open_and_type(router: &mut Router, session: u64, input: &str) {
    let session = SessionId(session);
    router.handle(ClientMessage::OpenSession {
        session,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    for character in input.chars() {
        router.handle(ClientMessage::Key {
            session,
            event: KeyEvent::new(
                character.to_ascii_uppercase() as u32,
                Some(character),
                KeyModifiers::default(),
            ),
        });
    }
}

#[test]
fn startup_loads_bundled_code_and_user_override() {
    let dir = temp_dir("code-startup");
    scheme(&dir, true, "开\tga\t10\n");
    let mut router = watched(&dir, PURE);
    assert!(router.engine.code_scheme().is_some());
    assert_eq!(texts(&mut router, "ga"), ["开"]);
    assert!(texts(&mut router, "ni").is_empty());
    scheme(&dir, false, "改\tga\t10\n");
    poll(&mut router);
    assert_eq!(texts(&mut router, "ga"), ["改"]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn selection_changes_clear_all_sessions_but_unrelated_config_keeps_input() {
    let dir = temp_dir("code-sessions");
    scheme(&dir, true, "开\tga\t10\n");
    let mut router = watched(&dir, "");
    open_and_type(&mut router, 1, "ni");
    open_and_type(&mut router, 2, "hao");
    write_config(&dir.join("config.toml"), "[general]\npage_size = 7\n", 200);
    poll(&mut router);
    assert_eq!(router.engine.composition().text(), "hao");
    write_config(&dir.join("config.toml"), PURE, 300);
    poll(&mut router);
    assert!(router.engine.composition().is_empty());
    for session in [SessionId(1), SessionId(2)] {
        router.ensure_focus(session);
        assert!(router.engine.composition().is_empty());
        assert!(router.current_frame().candidates.items.is_empty());
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_or_broken_code_falls_back_to_pinyin() {
    let dir = temp_dir("code-fallback");
    let config_path = dir.join("config.toml");
    write_config(&config_path, PURE, 100);
    let config = Config::load(&config_path).unwrap();
    let mut router = Router::new(
        Engine::new(Dictionary::parse("你\tni\t100\n").unwrap()),
        crate::dispatch::RouterConfig::from(&config),
    );
    router.watch_config(&config, config_path, dir.clone(), UserDirs::at(dir.clone()));
    assert_eq!(texts(&mut router, "ni"), ["你"]);
    scheme(&dir, true, "开\tga\t10\n");
    poll(&mut router);
    assert_eq!(texts(&mut router, "ga"), ["开"]);
    std::fs::write(dir.join("data/generated/schemes/wubi86/table.tsv"), [0xff]).unwrap();
    poll(&mut router);
    assert!(router.engine.code_scheme().is_none());
    assert_eq!(texts(&mut router, "ni"), ["你"]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn code_rules_and_mixed_input_apply_without_restart() {
    let dir = temp_dir("code-rules");
    scheme(&dir, true, "开\tga\t10\n开发\tgant\t1\n");
    let mut router = watched(&dir, PURE);
    assert!(router.engine.code_scheme().unwrap().auto_commit);
    write_config(
        &dir.join("config.toml"),
        "[general]\ncode = \"wubi86\"\n[code.wubi86]\nauto_commit = false\ntop = false\n",
        200,
    );
    poll(&mut router);
    assert!(!router.engine.code_scheme().unwrap().auto_commit);
    assert!(!router.engine.code_scheme().unwrap().top);
    router.engine.set_input("ga");
    assert!(
        router
            .engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .any(|candidate| candidate.kind == CandidateKind::Code)
    );
    assert!(!router.engine.composing_code());
    write_config(&dir.join("config.toml"), "", 300);
    poll(&mut router);
    assert!(router.engine.code_scheme().is_none());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn imported_code_words_reload_and_respect_last_valid_config() {
    let dir = temp_dir("code-words");
    scheme(&dir, true, "开\tga\t10\n");
    let mut router = watched(&dir, PURE);
    let words = dir.join("custom/code_dicts");
    std::fs::create_dir_all(&words).unwrap();
    let path = words.join("extra.tsv");
    std::fs::write(&path, "# scheme: wubi86\n新增\tga\t20\n").unwrap();
    poll(&mut router);
    assert!(texts(&mut router, "ga").contains(&"新增".to_owned()));
    write_config(
        &dir.join("config.toml"),
        &format!("{PURE}\n[dictionaries]\ndisabled = [\"extra\"]\n"),
        200,
    );
    poll(&mut router);
    assert_eq!(texts(&mut router, "ga"), ["开"]);
    write_config(&dir.join("config.toml"), "[broken", 300);
    std::fs::write(&path, "# scheme: wubi86\n仍禁用\tga\t30\n").unwrap();
    poll(&mut router);
    assert_eq!(texts(&mut router, "ga"), ["开"]);
    std::fs::remove_file(path).unwrap();
    poll(&mut router);
    assert_eq!(texts(&mut router, "ga"), ["开"]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn custom_shuangpin_is_loaded_and_reloaded() {
    let dir = temp_dir("custom-shuangpin");
    let path = dir.join("custom/schemes/my-xiaohe");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(
        path.join("scheme.toml"),
        "type = \"shuangpin\"\nname = \"测试双拼\"\nbase = \"xiaohe\"\n",
    )
    .unwrap();
    let mut router = watched(&dir, "[general]\nscheme = \"my-xiaohe\"\n");
    assert!(router.engine.shuangpin().is_some());
    std::fs::remove_file(path.join("scheme.toml")).unwrap();
    poll(&mut router);
    assert!(router.engine.shuangpin().is_none());
    std::fs::remove_dir_all(dir).unwrap();
}
