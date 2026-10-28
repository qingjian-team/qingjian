//! 只读检查 Server 实际使用的配置与翻译装配；不启动 IPC、窗口、UIA 或剪贴板访问。

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use qingjian_core::Engine;
use qingjian_dictionary::Dictionary;
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, ScreenRect,
    ServerMessage, SessionId,
};
use qingjian_platform::{KeyCombo, resources};
use qingjian_windows_server::{Router, RouterConfig, dispatch};

const SESSION: SessionId = SessionId(1);
const RETURN: u32 = 0x0d;
const ESCAPE: u32 = 0x1b;
const CHINESE: &str = "今天下午我们一起去公园散步。";
const ENGLISH: &str = "Hello, we'll go for a walk to the park this afternoon.";
const LONG_CHINESE: &str =
    "今天下午我们一起去公园散步，如果下雨，我们就留在家里看电影，明天上午再去公园。";

pub(super) fn run(root: Option<String>) -> i32 {
    super::attach_parent_console();
    super::diagnostics::ConfigSource::inspect(super::config_path().as_deref()).print();
    if super::config_path().is_none() {
        println!("config_loaded=false config_source=path_unavailable");
        return 1;
    }
    let config = match super::load_config() {
        Ok(config) => config,
        Err(_) => {
            println!("config_loaded=false");
            return 1;
        }
    };
    let root = root.map(PathBuf::from).or_else(resources::bundled_root);
    let Some(root) = root else {
        println!("bundle_found=false");
        return 1;
    };
    let mut engine = Engine::new(Dictionary::default());
    let user = super::user_dir();
    dispatch::attach_cloud(&mut engine, &config.predict, None);
    dispatch::attach_offline_translation(
        &mut engine,
        config.offline_translation.enabled,
        user.as_deref(),
        &root,
    );
    let ready = engine.translation_enabled();
    println!(
        "config_loaded=true offline_configured={} cloud_configured={} translation_ready={ready}",
        config.offline_translation.enabled, config.predict.enabled
    );
    i32::from(!ready)
}

/// 从最终 Server 程序验证离线推理与消息状态机；固定样例不经过真实宿主或系统输入。
pub(super) fn verify(root: Option<String>) -> i32 {
    super::attach_parent_console();
    match verify_inner(root) {
        Ok(()) => {
            println!("offline_router_verification=passed host_tsf_verified=false");
            0
        }
        Err(reason) => {
            println!("offline_router_verification=failed reason={reason}");
            1
        }
    }
}

fn verify_inner(root: Option<String>) -> Result<(), &'static str> {
    let config = super::load_config().map_err(|_| "config_load_failed")?;
    if !config.offline_translation.enabled {
        return Err("offline_translation_disabled");
    }
    let root = root
        .map(PathBuf::from)
        .or_else(resources::bundled_root)
        .ok_or("bundle_missing")?;
    // 不装配真实词库、学习器、日志或配置热加载；不会写用户数据或启动更新检查。
    let dictionary = Dictionary::parse("你好\tni hao\t100\n你\tni\t90\n好\thao\t80\n")
        .map_err(|_| "verification_dictionary_invalid")?;
    let mut engine = Engine::new(dictionary);
    dispatch::attach_offline_translation(&mut engine, true, super::user_dir().as_deref(), &root);
    if !engine.translation_enabled() || engine.prediction_enabled() {
        return Err("offline_provider_unavailable");
    }
    let mut router_config = RouterConfig::from(&config);
    router_config.cloud_enabled = false;
    router_config.sentence_translation = false;
    let shortcut = router_config.translate_selection;
    let mut router = Router::new(engine, router_config);
    if !matches!(
        router.handle(ClientMessage::OpenSession {
            session: SESSION,
            app: Some("qingjian-offline-verification.exe".to_owned()),
            protocol: PROTOCOL_VERSION,
        }),
        Some(ServerMessage::SessionOpened {
            session: SESSION,
            ..
        })
    ) {
        return Err("session_open_failed");
    }
    println!("config_loaded=true offline_configured=true network_provider_attached=false");

    start_selection(&mut router, shortcut, CHINESE)?;
    // 先确认，再轮询；此时 Router 尚未接收异步结果，必须保持等待，不能提前取消。
    if !matches!(
        key(&mut router, RETURN, None, KeyModifiers::default()),
        Some(ServerMessage::SelectionReview {
            session: SESSION,
            active: true
        })
    ) {
        return Err("early_confirmation_lost_review");
    }
    let (translated, short_elapsed) = wait_translation(&mut router)?;
    validate_english(&translated, &["park", "afternoon"])?;
    println!("sample=zh_en output={translated}");
    accept_translation(&mut router, &translated)?;
    println!("check=early_confirmation_and_explicit_commit passed=true");

    start_selection(&mut router, shortcut, ENGLISH)?;
    let (translated, _) = wait_translation(&mut router)?;
    if !translated.chars().any(is_han)
        || !translated.contains("公园")
        || !translated.contains("下午")
    {
        return Err("english_to_chinese_content_invalid");
    }
    println!("sample=en_zh output={translated}");
    accept_translation(&mut router, &translated)?;

    start_selection(&mut router, shortcut, LONG_CHINESE)?;
    let (translated, long_elapsed) = wait_translation(&mut router)?;
    validate_english(&translated, &["park", "rain", "tomorrow"])?;
    println!("sample=long_zh_en output={translated}");
    cancel_translation(&mut router)?;
    if !poll_without_commit(&mut router)?.is_empty() {
        return Err("cancelled_translation_still_visible");
    }
    println!("check=ready_translation_escape passed=true");

    start_selection(&mut router, shortcut, LONG_CHINESE)?;
    thread::sleep(Duration::from_millis(100));
    cancel_translation(&mut router)?;
    verify_typing_after_cancel(
        &mut router,
        short_elapsed.max(long_elapsed).saturating_mul(2),
    )?;
    Ok(())
}

fn key(
    router: &mut Router,
    virtual_key: u32,
    character: Option<char>,
    modifiers: KeyModifiers,
) -> Option<ServerMessage> {
    router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(virtual_key, character, modifiers),
    })
}

fn start_selection(
    router: &mut Router,
    shortcut: KeyCombo,
    text: &str,
) -> Result<(), &'static str> {
    let request = match key(
        router,
        shortcut.key.to_ascii_uppercase() as u32,
        Some(shortcut.key),
        shortcut.modifiers.into(),
    ) {
        Some(ServerMessage::RequestSelection {
            session: SESSION,
            request,
        }) => request,
        _ => return Err("shortcut_did_not_request_selection"),
    };
    match router.handle(ClientMessage::Selection {
        session: SESSION,
        request,
        text: text.to_owned(),
        rect: ScreenRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        },
    }) {
        Some(ServerMessage::KeyResult {
            session: SESSION,
            outcome: KeyOutcome::Consumed,
            commit: None,
            ..
        }) => Ok(()),
        _ => Err("selection_response_invalid_or_committed_early"),
    }
}

fn poll_without_commit(router: &mut Router) -> Result<Frame, &'static str> {
    match router.handle(ClientMessage::Poll { session: SESSION }) {
        Some(ServerMessage::Update {
            session: SESSION,
            commit: None,
            frame,
        }) => Ok(frame),
        _ => Err("poll_response_invalid_or_unexpected_commit"),
    }
}

fn wait_translation(router: &mut Router) -> Result<(String, Duration), &'static str> {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(90) {
        let frame = poll_without_commit(router)?;
        if !frame.preedit.is_empty() || !router.engine_mut().composition().is_empty() {
            return Err("translation_modified_composition");
        }
        let candidate = frame
            .candidates
            .items
            .first()
            .ok_or("translation_review_disappeared")?;
        if candidate.text != "翻译中…" {
            if candidate.text.trim().is_empty() {
                return Err("empty_translation");
            }
            return Ok((candidate.text.clone(), started.elapsed()));
        }
        thread::sleep(Duration::from_millis(30));
    }
    Err("translation_timeout")
}

fn validate_english(text: &str, keywords: &[&str]) -> Result<(), &'static str> {
    let lower = text.to_ascii_lowercase();
    if text.chars().any(is_han)
        || !text
            .chars()
            .any(|character| character.is_ascii_alphabetic())
        || keywords.iter().any(|keyword| !lower.contains(keyword))
    {
        return Err("chinese_to_english_content_invalid");
    }
    Ok(())
}

fn is_han(character: char) -> bool {
    matches!(character as u32, 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff)
}

fn accept_translation(router: &mut Router, expected: &str) -> Result<(), &'static str> {
    match key(router, RETURN, None, KeyModifiers::default()) {
        Some(ServerMessage::KeyResult {
            session: SESSION,
            outcome: KeyOutcome::Consumed,
            commit: Some(text),
            frame,
            english: None,
        }) if text == expected && frame.is_empty() => {}
        _ => return Err("explicit_confirmation_did_not_commit_result"),
    }
    if !poll_without_commit(router)?.is_empty() {
        return Err("confirmed_translation_still_visible");
    }
    Ok(())
}

fn cancel_translation(router: &mut Router) -> Result<(), &'static str> {
    match key(router, ESCAPE, None, KeyModifiers::default()) {
        Some(ServerMessage::KeyResult {
            session: SESSION,
            outcome: KeyOutcome::Consumed,
            commit: None,
            frame,
            english: None,
        }) if frame.is_empty() => Ok(()),
        _ => Err("escape_did_not_cancel_without_commit"),
    }
}

fn verify_typing_after_cancel(
    router: &mut Router,
    observed_inference: Duration,
) -> Result<(), &'static str> {
    for character in "nihao".chars() {
        if !matches!(
            key(
                router,
                character.to_ascii_uppercase() as u32,
                Some(character),
                KeyModifiers::default(),
            ),
            Some(ServerMessage::KeyResult {
                commit: None,
                outcome: KeyOutcome::Consumed,
                ..
            })
        ) {
            return Err("ordinary_typing_committed_or_passthrough");
        }
    }
    // 覆盖先前实测推理时长；真正乱序交付由状态机单元测试确定性覆盖。
    let duration = observed_inference.clamp(Duration::from_secs(5), Duration::from_secs(30));
    let started = Instant::now();
    while started.elapsed() < duration {
        let frame = poll_without_commit(router)?;
        if router.engine_mut().composition().text() != "nihao"
            || frame.preedit.is_empty()
            || frame.candidates.items.iter().any(|candidate| {
                candidate.text == "翻译中…" || candidate.text.to_ascii_lowercase().contains("park")
            })
        {
            return Err("late_translation_changed_ordinary_typing");
        }
        thread::sleep(Duration::from_millis(30));
    }
    println!(
        "check=cancelled_result_does_not_commit_typing passed=true observed_ms={}",
        duration.as_millis()
    );
    Ok(())
}
