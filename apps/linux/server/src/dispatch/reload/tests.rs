//! 改 `config.toml` / 用户 `dicts/` 后，下一次 tick 就进入正在运行的 Router。

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use qingjian_core::Engine;
use qingjian_dictionary::{Dictionary, import};
use qingjian_platform::protocol::{ClientMessage, PROTOCOL_VERSION, SessionId};
use qingjian_platform::{Config, SwitchKeys};

use super::CONFIG_POLL_INTERVAL;
use crate::dispatch::{Router, RouterConfig};

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qingjian-reload-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 按 `config.toml` 起一个开了热加载的 Router。
fn watched(dir: &Path, source: &str) -> Router {
    let config_path = dir.join("config.toml");
    write_config(&config_path, source, 100);
    let config = Config::load(&config_path).unwrap();
    let mut router = Router::new(
        Engine::new(Dictionary::default()),
        RouterConfig::from(&config),
    );
    router.watch_config(&config, config_path, dir.to_owned(), dir.to_owned());
    router
}

/// 写文件并钉死 mtime：同一秒内连写两次时文件系统的 mtime 可能不变。
fn write_config(path: &Path, source: &str, seconds: u64) {
    std::fs::write(path, source).unwrap();
    set_mtime(path, seconds);
}

fn set_mtime(path: &Path, seconds: u64) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))
        .unwrap();
}

/// 跳过一秒节流，直接看一次。
fn poll(router: &mut Router) {
    router.reload.as_mut().unwrap().last_check = Instant::now() - CONFIG_POLL_INTERVAL;
    router.tick();
}

#[test]
fn edited_config_applies_on_next_tick() {
    let dir = temp_dir("apply");
    let mut router = watched(&dir, "[general]\npage_size = 5\n");
    assert_eq!(router.config.page_size, 5);
    assert_eq!(router.engine.shuangpin(), None);

    write_config(
        &dir.join("config.toml"),
        "[general]\npage_size = 7\nscheme = \"xiaohe\"\nchinese_first = false\n",
        200,
    );
    poll(&mut router);
    assert_eq!(router.config.page_size, 7);
    assert!(router.engine.shuangpin().is_some());
    assert!(!router.engine.chinese_first());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn turning_english_mode_off_returns_sessions_to_chinese() {
    let dir = temp_dir("english");
    let mut router = watched(&dir, "[general]\ndefault_english = true\n");
    router.handle(ClientMessage::OpenSession {
        session: SessionId(1),
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    assert!(router.sessions[&SessionId(1)].english);

    write_config(
        &dir.join("config.toml"),
        "[general]\nenglish_mode = false\ndefault_english = true\n",
        200,
    );
    poll(&mut router);
    assert!(!router.sessions[&SessionId(1)].english);
    assert_eq!(router.config.switch_keys(), SwitchKeys::NONE);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn broken_config_keeps_previous_settings() {
    let dir = temp_dir("broken");
    let mut router = watched(&dir, "[general]\npage_size = 6\n");

    write_config(&dir.join("config.toml"), "[general\npage_size = 3\n", 200);
    poll(&mut router);
    assert_eq!(router.config.page_size, 6);

    // 修好后照常生效
    write_config(&dir.join("config.toml"), "[general]\npage_size = 3\n", 300);
    poll(&mut router);
    assert_eq!(router.config.page_size, 3);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn user_dictionary_import_applies_without_config_change() {
    let dir = temp_dir("dicts");
    let mut router = watched(&dir, "");
    assert!(router.engine.extra_dictionaries().is_empty());

    let source = dir.join("law.dict.yaml");
    std::fs::write(&source, "---\nname: law\n...\n合同法\the tong fa\t120\n").unwrap();
    let imported = import::import(&source, &dir.join("dicts")).unwrap();
    set_mtime(&imported.path, 100);
    poll(&mut router);
    let dictionaries = router.engine.extra_dictionaries();
    assert_eq!(dictionaries.len(), 1);
    assert_eq!(
        dictionaries[0].lookup(&["he", "tong", "fa"], false)[0].text,
        "合同法"
    );

    std::fs::remove_file(&imported.path).unwrap();
    poll(&mut router);
    assert!(router.engine.extra_dictionaries().is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cloud_disable_clears_suspended_candidates_and_translation_review() {
    use crate::dispatch::{composed::Composed, session::SessionInfo, translate::Translation};
    use qingjian_core::{Candidate, CandidateKind, CandidateLayout};
    use qingjian_predict::PredictConfig;

    let dir = temp_dir("cloud-disable");
    let mut router = watched(&dir, "");
    let local = Candidate {
        text: "你".into(),
        kind: CandidateKind::Chinese,
        syllables: vec!["ni".into()],
        reading: None,
        translation: None,
        aux_code: None,
    };
    let composed = || {
        let mut layout = CandidateLayout::new(vec![local.clone()], 5, 1);
        let mut cloud = local.clone();
        cloud.text = "妮".into();
        cloud.kind = CandidateKind::Cloud;
        layout.set_cloud(vec![cloud]);
        Some(Composed::Candidates {
            preedit: Vec::new(),
            cursor: 0,
            layout,
            spelling: Default::default(),
            manual_sequence: None,
        })
    };
    router.composed = composed();
    let mut suspended = SessionInfo::new(None, false);
    suspended.composed = composed();
    router.sessions.insert(SessionId(2), suspended);
    router.translation = Some(Translation { result: None });
    router.pending_selection = Some((SessionId(1), 7));
    router.sentence = Some("旧服务补全".into());
    router.reload_cloud(&PredictConfig::default(), None);
    assert!(!router.engine.prediction_enabled());
    assert_eq!(router.config.cloud_slots, 0);
    assert!(router.translation.is_none());
    assert!(router.pending_selection.is_none());
    assert!(router.sentence.is_none());
    for composed in [&router.composed, &router.sessions[&SessionId(2)].composed] {
        let Some(Composed::Candidates { layout, .. }) = composed else {
            panic!("local candidates lost")
        };
        assert_eq!(layout.len(), 1);
        assert_eq!(layout.candidate(0), Some(&local));
    }
    std::fs::remove_dir_all(dir).unwrap();
}
