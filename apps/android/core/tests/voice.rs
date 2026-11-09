//! 语音：识别结果经会话上屏，按设置接识别方，没接时报不可用。
use std::path::Path;

use qingjian_android::{Session, VoiceSettings};
use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;

fn sample(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../assets/sample")
        .join(name)
}

fn session() -> Session {
    let engine = Engine::new(Dictionary::from_path(sample("dict.tsv")).unwrap());
    let user_dir = std::env::temp_dir().join(format!("qingjian-voice-{}", std::process::id()));
    Session::new(engine, sample("wubi86.tsv"), sample(""), user_dir)
}

#[test]
fn transcriber_is_unavailable_until_one_is_plugged_in() {
    let mut session = session();
    assert!(!session.voice_available());
    session.voice_start(16_000);
    session.voice_feed(&[0; 160]);
    session.voice_finish();
    assert_eq!(session.poll_voice(), None);
}

#[test]
fn voice_text_commits_through_the_session() {
    let mut session = session();
    let frame = session.commit_voice("你好");
    assert_eq!(frame.commit, "你好");
    assert!(frame.preedit.is_empty());
}

/// 组句中来了语音（壳没先落下）：敲的字母原样落在识别结果前面，组句清空。
#[test]
fn voice_text_settles_the_composition_first() {
    let mut session = session();
    session.push('n');
    session.push('i');
    let frame = session.commit_voice("你好");
    assert_eq!(frame.commit, "ni你好");
    assert!(frame.preedit.is_empty());
    assert!(frame.candidates.is_empty());
}

fn settings(json: &str) -> VoiceSettings {
    serde_json::from_str(json).unwrap()
}

/// 选了火山引擎且填了密钥才能用；没填密钥、别的服务、清掉配置都退回不可用。只换识别方，不连网。
#[test]
fn voice_settings_pick_the_transcriber() {
    let mut session = session();
    session.set_voice(Some(&settings(
        r#"{"provider":"volcengine","api_key":"k"}"#,
    )));
    assert!(session.voice_available());
    session.set_voice(Some(&settings(
        r#"{"provider":"volcengine","api_key":" "}"#,
    )));
    assert!(!session.voice_available());
    session.set_voice(Some(&settings(r#"{"provider":"none","api_key":"k"}"#)));
    assert!(!session.voice_available());
    session.set_voice(Some(&settings(
        r#"{"provider":"volcengine","api_key":"k"}"#,
    )));
    session.set_voice(None);
    assert!(!session.voice_available());
}
