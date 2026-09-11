//! 不经传输层，直接把协议消息喂给 Router 的闭环测试；用样例词库，跨平台可跑。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use qingjian_core::{Language, ShuangpinScheme};
use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyModifiers, KeyOutcome, PROTOCOL_VERSION, ServerMessage,
    SessionId,
};
use qingjian_platform::{AppsConfig, DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS};
use qingjian_windows_server::dispatch::{StatusEvent, StatusSink, StatusView};
use qingjian_windows_server::{AssemblySpec, Router, RouterConfig, assembly};

const SESSION: SessionId = SessionId(1);

/// Caps Lock 亮着。
const CAPS: KeyModifiers = KeyModifiers {
    ctrl: false,
    shift: false,
    alt: false,
    win: false,
    caps: true,
    english_mode: false,
};

/// 持久英文模式（Caps 灭）。
const ENGLISH: KeyModifiers = KeyModifiers {
    ctrl: false,
    shift: false,
    alt: false,
    win: false,
    caps: false,
    english_mode: true,
};

/// 样例词库装一个 Router，开好一个会话。
fn router() -> Router {
    router_with(RouterConfig::default())
}

fn router_with(config: RouterConfig) -> Router {
    router_in(config, None)
}

/// 在某个应用（宿主 exe 名）里开会话，名单用 Windows 缺省那份。
fn router_in_app(app: &str) -> Router {
    let config = RouterConfig {
        apps: AppsConfig::with_english_candidates_off(DEFAULT_ENGLISH_CANDIDATES_OFF_WINDOWS),
        ..RouterConfig::default()
    };
    router_in(config, Some(app.to_owned()))
}

fn router_in(config: RouterConfig, app: Option<String>) -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dict = root.join("assets/sample/dict.tsv");
    let glossary = root.join("assets/sample/glossary-en.tsv");
    let mut engine = assembly::assemble(&AssemblySpec {
        glossary: Some((Language::English, glossary)),
        english: Some(root.join("assets/sample/english.tsv")),
        ..AssemblySpec::new(dict)
    })
    .expect("assemble engine from sample data");
    // 与 main.rs 一样，双拼方案是启动时直接设给 Engine 的。
    engine.set_shuangpin(config.shuangpin);
    let mut router = Router::new(engine, config);
    assert_eq!(
        router.handle(ClientMessage::OpenSession {
            session: SESSION,
            app,
            protocol: PROTOCOL_VERSION,
        }),
        None
    );
    router
}

fn letter(c: char) -> KeyEvent {
    letter_with(c, Default::default())
}

/// `c` 的大小写就是 DLL 按 Shift 解析出的字符。
fn letter_with(c: char, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), modifiers)
}

fn press(router: &mut Router, event: KeyEvent) -> (KeyOutcome, Option<String>, Frame) {
    key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event,
    }))
}

/// 英文模式下敲一串字母，返回最后一次的处理结果。
fn type_english(router: &mut Router, text: &str) -> (KeyOutcome, Option<String>, Frame) {
    let mut last = None;
    for c in text.chars() {
        last = Some(press(router, letter_with(c, ENGLISH)));
    }
    last.expect("typed at least one letter")
}

fn candidate_texts(frame: &Frame) -> Vec<&str> {
    frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect()
}

fn digit(n: u32) -> KeyEvent {
    digit_with(n, Default::default())
}

/// 数字键 1–9；`character` 按 DLL 的解析：按着 Shift 是上档字符。
fn digit_with(n: u32, modifiers: KeyModifiers) -> KeyEvent {
    let c = if modifiers.shift {
        b")!@#$%^&*("[n as usize] as char
    } else {
        char::from_digit(n, 10).unwrap()
    };
    KeyEvent::new(0x30 + n, Some(c), modifiers)
}

const SHIFT: KeyModifiers = KeyModifiers {
    shift: true,
    ..ALT_OFF
};

const CTRL: KeyModifiers = KeyModifiers {
    ctrl: true,
    ..ALT_OFF
};

const ALT_OFF: KeyModifiers = KeyModifiers {
    ctrl: false,
    shift: false,
    alt: false,
    win: false,
    caps: false,
    english_mode: false,
};

/// 两个平台的缺省快捷键都没用 Win，拿来测「没配到的修饰键归应用」。
const WIN: KeyModifiers = KeyModifiers {
    win: true,
    ..ALT_OFF
};

/// 平台缺省的译词键：macOS 是 Alt，Windows 是 Ctrl（Alt 被系统菜单截走）。
#[cfg(not(windows))]
const TRANSLATE: KeyModifiers = KeyModifiers {
    alt: true,
    ..ALT_OFF
};
#[cfg(windows)]
const TRANSLATE: KeyModifiers = KeyModifiers {
    ctrl: true,
    ..ALT_OFF
};

const TRANSLATE_SECOND: KeyModifiers = KeyModifiers {
    shift: true,
    ..TRANSLATE
};

/// 当前页里 `text` 排第几（1 起）。
fn slot_of(frame: &Frame, text: &str) -> u32 {
    let position = candidate_texts(frame)
        .iter()
        .position(|t| *t == text)
        .unwrap_or_else(|| panic!("{text} 应在当前页：{:?}", candidate_texts(frame)));
    position as u32 + 1
}

/// 带字符的按键（标点等），虚拟键码随便给一个 OEM 键。
fn punct(c: char) -> KeyEvent {
    KeyEvent::new(0xBE, Some(c), Default::default())
}

fn key_result(message: Option<ServerMessage>) -> (KeyOutcome, Option<String>, Frame) {
    match message {
        Some(ServerMessage::KeyResult {
            outcome,
            commit,
            frame,
            ..
        }) => (outcome, commit, frame),
        other => panic!("expected KeyResult, got {other:?}"),
    }
}

/// 中文模式下敲一串字母，返回最后一次的处理结果。
fn type_letters(router: &mut Router, text: &str) -> (KeyOutcome, Option<String>, Frame) {
    let mut last = None;
    for c in text.chars() {
        last = Some(key_result(router.handle(ClientMessage::Key {
            session: SESSION,
            event: letter(c),
        })));
    }
    last.expect("typed at least one letter")
}

fn preedit(frame: &Frame) -> String {
    frame.preedit.iter().map(|s| s.text.as_str()).collect()
}

#[test]
fn typing_pinyin_shows_candidates() {
    let mut router = router();
    let (outcome, commit, frame) = type_letters(&mut router, "nihao");

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit, None);
    assert_eq!(preedit(&frame), "ni'hao");
    let texts: Vec<&str> = frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert!(
        texts.contains(&"你好"),
        "候选里应有「你好」，实际：{texts:?}"
    );
}

#[test]
fn selecting_by_digit_commits_and_clears() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let position = frame
        .candidates
        .items
        .iter()
        .position(|c| c.text == "你好")
        .expect("「你好」在候选页内");
    let (outcome, commit, after) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: digit(position as u32 + 1),
    }));

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("你好"));
    assert!(
        after.is_empty(),
        "上屏后应收起候选，实际 preedit={:?}",
        preedit(&after)
    );
}

#[test]
fn space_commits_first_candidate() {
    let mut router = router();
    type_letters(&mut router, "ni");
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    let (outcome, commit, after) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: space,
    }));

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("你"), "「ni」首选应是「你」");
    assert!(after.is_empty());
}

#[test]
fn backspace_shrinks_preedit() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    assert_eq!(preedit(&frame), "ni'hao");
    let back = KeyEvent::new(0x08, None, Default::default());
    let (outcome, _, after) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: back,
    }));

    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(preedit(&after), "ni'ha");
}

#[test]
fn non_letter_without_composing_passes_through() {
    let mut router = router();
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    let (outcome, commit, frame) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: space,
    }));

    assert_eq!(outcome, KeyOutcome::Passthrough);
    assert_eq!(commit, None);
    assert!(frame.is_empty());
}

#[test]
fn focus_leave_commits_raw_pinyin() {
    let mut router = router();
    type_letters(&mut router, "nihao");
    let committed = router.handle(ClientMessage::Commit { session: SESSION });
    assert_eq!(
        committed,
        Some(ServerMessage::Committed {
            session: SESSION,
            text: Some("nihao".to_owned()),
        })
    );
    let (_, _, frame) = type_letters(&mut router, "ni");
    assert_eq!(preedit(&frame), "ni");
    // 没在组句时 Commit 不交东西。
    router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x1B, None, Default::default()),
    });
    assert_eq!(
        router.handle(ClientMessage::Commit { session: SESSION }),
        Some(ServerMessage::Committed {
            session: SESSION,
            text: None,
        })
    );
}

#[test]
fn commit_from_other_session_does_not_take_buffer() {
    let mut router = router();
    type_letters(&mut router, "ni");
    let other = SessionId(2);
    router.handle(ClientMessage::OpenSession {
        session: other,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    // 别的会话拿不到这个会话的拼音，但残留组句一并清掉。
    assert_eq!(
        router.handle(ClientMessage::Commit { session: other }),
        Some(ServerMessage::Committed {
            session: other,
            text: None,
        })
    );
    let space = KeyEvent::new(0x20, Some(' '), Default::default());
    let (outcome, _, _) = key_result(router.handle(ClientMessage::Key {
        session: SESSION,
        event: space,
    }));
    assert_eq!(outcome, KeyOutcome::Passthrough);
}

#[test]
fn page_keys_follow_config() {
    // 每页 1 条保证多页；翻页键改成 `,` `.`。
    let mut router = router_with(RouterConfig {
        page_size: 1,
        page_keys: (',', '.'),
        ..RouterConfig::default()
    });
    let (_, _, frame) = type_letters(&mut router, "ni");
    assert!(frame.page_count > 1, "样例词库里 ni 应不止一个候选");
    assert_eq!(frame.page, 0);

    let key = |router: &mut Router, c| {
        key_result(router.handle(ClientMessage::Key {
            session: SESSION,
            event: punct(c),
        }))
    };
    let (outcome, commit, frame) = key(&mut router, '.');
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(frame.page, 1, "`.` 应翻到下一页");
    let (_, _, frame) = key(&mut router, ',');
    assert_eq!(frame.page, 0, "`,` 应翻回上一页");
    // 缺省的 `]` 此时不再翻页，进直输段。
    let (_, _, frame) = key(&mut router, ']');
    assert_eq!(frame.page, 0);
    assert!(
        preedit(&frame).contains(']'),
        "`]` 应进直输段：{}",
        preedit(&frame)
    );
}

#[test]
fn english_mode_gives_candidates_and_space_commits_raw() {
    let mut router = router();
    let (outcome, commit, frame) = type_english(&mut router, "hel");
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "hel", "英文模式敲的字母原样显示");
    let texts = candidate_texts(&frame);
    assert!(
        texts.contains(&"hello") && texts.contains(&"help"),
        "候选应来自英文词表：{texts:?}"
    );
    // 没动过高亮的空格：字母原样上屏，空格一起插（放行会让应用先插空格）。
    let (outcome, commit, after) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(commit.as_deref(), Some("hel "));
    assert!(after.is_empty());
}

#[test]
fn caps_lock_types_direct_uppercase_english_regardless_of_mode() {
    let mut router = router();
    let (outcome, commit, frame) = press(&mut router, letter_with('H', CAPS));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("H"))
    );
    assert!(frame.is_empty(), "Caps 直接上屏不出候选：{frame:?}");
    // 组句中 Caps 亮着敲字母：拼音先原样上屏，再接大写字母。
    type_letters(&mut router, "ni");
    let (_, commit, after) = press(&mut router, letter_with('A', CAPS));
    assert_eq!(commit.as_deref(), Some("niA"));
    assert!(after.is_empty());
}

#[test]
fn english_candidates_are_off_in_listed_apps_by_exe_name() {
    // VS Code 在缺省名单里（exe 名不区分大小写）：英文模式字母直插、不出候选。
    let mut router = router_in_app("code.exe");
    let (outcome, commit, frame) = press(&mut router, letter_with('h', ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("h"))
    );
    assert!(frame.is_empty(), "名单里的应用不该有候选：{frame:?}");
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
    // 中文模式不受名单影响。
    let (_, _, frame) = type_letters(&mut router, "ni");
    assert!(!frame.candidates.items.is_empty(), "拼音照常出候选");
}

#[test]
fn english_candidates_stay_on_in_other_apps() {
    let mut router = router_in_app("notepad.exe");
    let (outcome, commit, frame) = type_english(&mut router, "hel");
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(
        candidate_texts(&frame).contains(&"hello"),
        "不在名单里的应用照常给英文候选：{frame:?}"
    );
}

#[test]
fn app_list_is_looked_up_per_session() {
    // 两个应用同时在线：切会话时按各自的 exe 名判断。
    let mut router = router_in_app("Code.exe");
    let notepad = SessionId(2);
    router.handle(ClientMessage::OpenSession {
        session: notepad,
        app: Some("notepad.exe".to_owned()),
        protocol: PROTOCOL_VERSION,
    });
    let (_, _, frame) = key_result(router.handle(ClientMessage::Key {
        session: notepad,
        event: letter_with('h', ENGLISH),
    }));
    assert_eq!(preedit(&frame), "h", "记事本会话组词");
    // 切回编辑器会话：残留组句清掉，字母直插。
    let (outcome, commit, after) = press(&mut router, letter_with('e', ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("e"))
    );
    assert!(after.is_empty());
}

#[test]
fn english_tab_and_navigated_space_pick_candidates() {
    let mut router = router();
    let (_, _, frame) = type_english(&mut router, "hel");
    let first = frame.candidates.items[0].text.clone();
    // Tab 选高亮的词。
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0x09, None, ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some(first.as_str()))
    );

    // 方向键动过高亮之后，空格也选那个词，再接上空格。
    let (_, _, frame) = type_english(&mut router, "hel");
    let second = frame.candidates.items[1].text.clone();
    let (outcome, _, _) = press(&mut router, KeyEvent::new(0x28, None, ENGLISH));
    assert_eq!(outcome, KeyOutcome::Consumed);
    let (_, commit, after) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(commit, Some(format!("{second} ")));
    assert!(after.is_empty());
}

#[test]
fn english_without_candidates_is_passthrough_with_shift_case() {
    let mut router = router_with(RouterConfig {
        english_candidates: false,
        ..RouterConfig::default()
    });
    // 字母由我们插入，大小写按 Shift；不组句。
    let (outcome, commit, frame) = press(&mut router, letter_with('h', ENGLISH));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("h"))
    );
    assert!(frame.is_empty());
    let shifted = KeyModifiers {
        shift: true,
        ..ENGLISH
    };
    let (_, commit, _) = press(&mut router, letter_with('H', shifted));
    assert_eq!(commit.as_deref(), Some("H"));
    // 其他键交给应用。
    let (outcome, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn switching_to_chinese_mid_word_flushes_english_letters() {
    let mut router = router();
    type_english(&mut router, "hel");
    // 切回中文模式再敲字母：之前的英文字母原样上屏，新字母从头当拼音。
    let (outcome, commit, frame) = press(&mut router, letter('l'));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hel"))
    );
    assert_eq!(preedit(&frame), "l");
}

#[test]
fn shift_uppercase_while_composing_commits_raw_first() {
    let mut router = router();
    type_letters(&mut router, "ni");
    // 中文模式按住 Shift 打大写字母：拼音原样上屏，字母跟在后面一起插。
    let shifted = KeyModifiers {
        shift: true,
        ..KeyModifiers::default()
    };
    let (outcome, commit, frame) = press(&mut router, letter_with('A', shifted));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("niA"))
    );
    assert!(frame.is_empty());
    // 没在组句时大写字母交给应用。
    let (outcome, commit, _) = press(&mut router, letter_with('A', shifted));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn alt_digit_commits_first_translation() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    // 缺省译词键（mac ⌥ / Windows Ctrl）+ 数字：上屏那个候选的第一个译词，组句结束。
    let (outcome, commit, after) = press(&mut router, digit_with(slot, TRANSLATE));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("hello"))
    );
    assert!(after.is_empty());
}

#[test]
fn second_translation_key_without_second_sense_is_swallowed() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    // 样例释义表里「你好」只有一条译文：第二个译词键（Shift+译词键）+ 数字吞掉不动，组句还在。
    let (outcome, commit, after) = press(&mut router, digit_with(slot, TRANSLATE_SECOND));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&after), "ni'hao");
}

#[test]
fn shift_digit_forgets_candidate_and_requeries() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");
    // 缺省 Shift + 数字：删候选（词库词只清学习记录），重新查一遍，组句不变。
    let (outcome, commit, after) = press(&mut router, digit_with(slot, SHIFT));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&after), "ni'hao");
    assert!(!after.candidates.items.is_empty());
}

#[test]
fn unconfigured_modifier_digit_is_not_a_selection() {
    // 删候选改成 Ctrl+Shift：Shift+4 就是普通的 `$`；Win+1 没配到快捷键，归应用。
    let mut router = router_with(RouterConfig {
        delete_keys: KeyModifiers {
            shift: true,
            ..CTRL
        },
        ..RouterConfig::default()
    });
    type_letters(&mut router, "nihao");
    let (outcome, commit, frame) = press(&mut router, digit_with(4, SHIFT));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(preedit(&frame).contains('$'), "{}", preedit(&frame));
    let (outcome, commit, _) = press(&mut router, digit_with(1, WIN));
    assert_eq!((outcome, commit), (KeyOutcome::Passthrough, None));
}

#[test]
fn learning_data_persists_to_user_dir() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let user_dir =
        std::env::temp_dir().join(format!("qingjian-windows-learning-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&user_dir);
    std::fs::create_dir_all(&user_dir).unwrap();
    let engine = assembly::assemble(&AssemblySpec {
        glossary: Some((
            Language::English,
            root.join("assets/sample/glossary-en.tsv"),
        )),
        user_dir: Some(user_dir.clone()),
        ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
    })
    .unwrap();
    let mut router = Router::new(engine, RouterConfig::default());
    router.handle(ClientMessage::OpenSession {
        session: SESSION,
        app: None,
        protocol: PROTOCOL_VERSION,
    });
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let position = frame
        .candidates
        .items
        .iter()
        .position(|c| c.text == "你好")
        .unwrap();
    router.handle(ClientMessage::Key {
        session: SESSION,
        event: digit(position as u32 + 1),
    });
    // 关会话时落盘。
    router.handle(ClientMessage::CloseSession { session: SESSION });

    let user = std::fs::read_to_string(user_dir.join("user.tsv")).expect("user.tsv 应已写出");
    assert!(user.contains("你好"), "user.tsv 里应记了「你好」：{user}");
    assert!(user_dir.join("usage.tsv").is_file(), "usage.tsv 应已写出");
    let _ = std::fs::remove_dir_all(&user_dir);
}

/// 记录状态条调用：`Some(模式格文字)` 是显示、`None` 是收起。
#[derive(Clone, Default)]
struct RecordingStatus(Arc<Mutex<Vec<Option<String>>>>);

impl RecordingStatus {
    fn calls(&self) -> Vec<Option<String>> {
        self.0.lock().unwrap().clone()
    }
}

impl StatusSink for RecordingStatus {
    fn show_status(&self, view: StatusView) {
        let label = match (view.english, view.scheme) {
            (true, _) => "英".to_owned(),
            (false, Some(scheme)) => format!("中 · {scheme}"),
            (false, None) => "中".to_owned(),
        };
        self.0.lock().unwrap().push(Some(label));
    }

    fn hide_status(&self) {
        self.0.lock().unwrap().push(None);
    }
}

#[test]
fn status_bar_mode_click_is_handed_to_dll_via_sync_mode() {
    let config = RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });

    // 点「中」：状态条先翻成「英」，DLL 来取时拿到目标模式，取一次就清。
    router.handle_status_event(StatusEvent::ToggleMode);
    assert_eq!(recorder.calls().last(), Some(&Some("英".to_owned())));
    assert_eq!(
        router.handle(ClientMessage::SyncMode { session: SESSION }),
        Some(ServerMessage::ModeSync {
            session: SESSION,
            english: Some(true),
        })
    );
    assert_eq!(
        router.handle(ClientMessage::SyncMode { session: SESSION }),
        Some(ServerMessage::ModeSync {
            session: SESSION,
            english: None,
        })
    );
}

#[test]
fn chinese_punctuation_is_full_width_only_when_not_composing() {
    let mut router = router();
    // 没在组句：逗号转全角；数字后的点保持半角。
    let comma = KeyEvent::new(0xBC, Some(','), Default::default());
    assert_eq!(
        press(&mut router, comma),
        (
            KeyOutcome::Consumed,
            Some("，".to_owned()),
            Frame::default()
        )
    );
    press(&mut router, digit(3));
    let period = KeyEvent::new(0xBE, Some('.'), Default::default());
    assert_eq!(press(&mut router, period).0, KeyOutcome::Passthrough);
    assert_eq!(press(&mut router, period).1, Some("。".to_owned()));

    // 组句中：标点进英文直输段，不转。
    type_letters(&mut router, "ni");
    let (outcome, commit, frame) = press(&mut router, comma);
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(!frame.is_empty(), "组句应还在");

    // 状态条上关掉全角：原样交给应用。
    router.handle(ClientMessage::Commit { session: SESSION });
    router.handle_status_event(StatusEvent::TogglePunctuation);
    assert_eq!(press(&mut router, comma).0, KeyOutcome::Passthrough);
}

#[test]
fn status_bar_follows_mode_when_enabled() {
    let config = RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));

    // 中文 → 英文：各刷一次；会话关掉（应用退出）不收；切成别的输入法才收起。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
    });
    router.handle(ClientMessage::CloseSession { session: SESSION });
    assert_eq!(
        recorder.calls(),
        vec![Some("中".to_owned()), Some("英".to_owned())]
    );

    router.handle(ClientMessage::ImeSwitched { session: SESSION });
    assert_eq!(recorder.calls().last(), Some(&None));
}

#[test]
fn status_bar_shows_shuangpin_scheme_in_chinese() {
    let config = RouterConfig {
        status_enabled: true,
        shuangpin: Some(ShuangpinScheme::Xiaohe),
        ..RouterConfig::default()
    };
    let mut router = router_with(config);
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));

    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });

    assert_eq!(recorder.calls(), vec![Some("中 · 小鹤双拼".to_owned())]);
}

#[test]
fn status_bar_stays_hidden_when_disabled() {
    let mut router = router();
    let recorder = RecordingStatus::default();
    router.set_status_sink(Box::new(recorder.clone()));

    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });

    assert_eq!(recorder.calls(), vec![None]);
}

#[test]
fn deleting_a_candidate_shows_a_notice_until_next_key() {
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好");

    // 「你好」是词库词且没学习记录，删不掉，但提示照样给出。
    let (outcome, _, after) = press(&mut router, digit_with(slot, SHIFT));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(!after.is_empty(), "删候选后仍在组句");
    let notice = after.notice.as_deref().expect("删候选后应带屏幕提示");
    assert!(notice.contains("你好"), "提示应提到候选词，实际：{notice}");

    let (_, _, next) = press(&mut router, KeyEvent::new(0x28, None, Default::default())); // VK_DOWN
    assert_eq!(next.notice, None, "提示应只活到下一次按键");
}

#[test]
fn expression_mode_takes_digits_and_operators() {
    let mut router = router();
    // v 开头进表达式模式：数字不选词、运算符进算式，Shift + 6 是 `^` 而不是删候选键。
    type_letters(&mut router, "v");
    press(&mut router, digit(1));
    press(&mut router, punct('+'));
    let (outcome, commit, frame) = press(&mut router, digit(2));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "v1+2");
    assert_eq!(candidate_texts(&frame), ["3", "1+2=3"]);
    press(&mut router, digit_with(6, SHIFT));
    let (_, _, frame) = press(&mut router, digit(2));
    assert_eq!(preedit(&frame), "v1+2^2");
    assert_eq!(candidate_texts(&frame), ["5", "1+2^2=5"]);
    // 空格上屏首选并清空。
    let (outcome, commit, frame) = press(&mut router, punct(' '));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("5".to_owned()))
    );
    assert!(preedit(&frame).is_empty());
}

#[test]
fn expression_mode_spells_chinese_numerals() {
    let mut router = router();
    type_letters(&mut router, "v");
    for n in [1, 2, 3] {
        press(&mut router, digit(n));
    }
    let (_, _, frame) = press(&mut router, punct('.'));
    assert_eq!(preedit(&frame), "v123.");
    let (_, _, frame) = press(&mut router, digit(5));
    assert_eq!(candidate_texts(&frame), ["123.5", "123.5=123.5"]);
    router.handle(ClientMessage::Key {
        session: SESSION,
        event: KeyEvent::new(0x1B, None, Default::default()),
    });
    type_letters(&mut router, "v");
    for n in [1, 2, 3] {
        press(&mut router, digit(n));
    }
    let (_, _, frame) = press(&mut router, punct('+'));
    // `v123+` 算不出来就没有候选，回车上屏原文。
    assert!(candidate_texts(&frame).is_empty());
    let (_, _, frame) = press(&mut router, KeyEvent::new(0x08, None, Default::default()));
    assert_eq!(candidate_texts(&frame), ["一百二十三", "壹佰贰拾叁"]);
    press(&mut router, KeyEvent::new(0x28, None, Default::default()));
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("壹佰贰拾叁"));
}

#[test]
fn expression_mode_other_punctuation_commits_then_applies() {
    let mut router = router();
    type_letters(&mut router, "v");
    press(&mut router, digit(1));
    press(&mut router, punct('+'));
    press(&mut router, digit(2));
    // 逗号不是算式的一部分：先把首选上屏，逗号按没在组句处理（中文模式转全角）。
    let (outcome, commit, frame) = press(&mut router, punct(','));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("3，".to_owned()))
    );
    assert!(preedit(&frame).is_empty());
}

#[test]
fn question_key_unicode_entry_takes_digits() {
    let mut router = router();
    type_letters(&mut router, "u");
    press(&mut router, digit(4));
    type_letters(&mut router, "e");
    press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    let (_, _, frame) = press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    assert_eq!(preedit(&frame), "u4e00");
    assert_eq!(candidate_texts(&frame), ["一"]);
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("一"));
    // `u+1f600`：`+` 也进缓冲区。
    type_letters(&mut router, "u");
    press(&mut router, punct('+'));
    press(&mut router, digit(1));
    type_letters(&mut router, "f");
    press(&mut router, digit(6));
    press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    let (_, _, frame) = press(
        &mut router,
        KeyEvent::new(0x30, Some('0'), Default::default()),
    );
    assert_eq!(candidate_texts(&frame), ["😀"]);
}

fn function_key(virtual_key: u32) -> KeyEvent {
    KeyEvent::new(virtual_key, None, Default::default())
}

#[test]
fn bare_question_mark_enters_question_mode_in_both_modes() {
    let mut router = router();
    // 中文模式：`?` 进问字模式不上屏，后面的字母是问题。
    let (outcome, commit, frame) = press(&mut router, punct('?'));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "?");
    let (_, commit, frame) = type_letters(&mut router, "sangemu");
    assert_eq!(commit, None);
    assert!(preedit(&frame).starts_with('?'), "{}", preedit(&frame));
    press(&mut router, function_key(0x1B));
    // 英文模式也一样，Caps 送来的大写字母按小写收。
    let (outcome, commit, frame) = press(&mut router, KeyEvent::new(0xBF, Some('?'), CAPS));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "?");
    let (_, _, frame) = press(&mut router, letter_with('S', CAPS));
    assert_eq!(preedit(&frame), "?s");
}

#[test]
fn bare_question_mark_restores_when_followed_by_other_keys() {
    let mut router = router();
    // 空格只是把这个 ? 上屏（中文模式全角），不多打空格。
    press(&mut router, punct('?'));
    let (outcome, commit, frame) = press(&mut router, punct(' '));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("？".to_owned()))
    );
    assert!(preedit(&frame).is_empty());
    // 回车同样只上屏问号，吞掉回车。
    press(&mut router, punct('?'));
    let (outcome, commit, _) = press(&mut router, function_key(0x0D));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("？".to_owned()))
    );
    // 其他字符：问号上屏后按没在组句处理（逗号转全角）。
    press(&mut router, punct('?'));
    let (_, commit, _) = press(&mut router, punct(','));
    assert_eq!(commit.as_deref(), Some("？，"));
    // 退格删掉它，什么都不上屏。
    press(&mut router, punct('?'));
    let (outcome, commit, frame) = press(&mut router, function_key(0x08));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(preedit(&frame).is_empty());
    // 英文模式还原成半角。
    press(&mut router, KeyEvent::new(0xBF, Some('?'), ENGLISH));
    let (_, commit, _) = press(&mut router, KeyEvent::new(0x20, Some(' '), ENGLISH));
    assert_eq!(commit.as_deref(), Some("?"));
}

#[test]
fn bare_question_mark_is_half_width_when_full_width_is_off() {
    let mut router = router_with(RouterConfig {
        full_width: false,
        ..RouterConfig::default()
    });
    press(&mut router, punct('?'));
    let (_, commit, _) = press(&mut router, punct(' '));
    assert_eq!(commit.as_deref(), Some("?"));
}

#[test]
fn shuangpin_semicolon_stays_in_buffer_in_question_mode() {
    let mut router = router_with(RouterConfig {
        shuangpin: Some(ShuangpinScheme::Microsoft),
        ..RouterConfig::default()
    });
    // 微软双拼的 `;` 是 ing 键：问字模式下末尾有落单声母时进缓冲区，而不是把候选上屏。
    press(&mut router, punct('?'));
    type_letters(&mut router, "x");
    let (outcome, commit, frame) = press(&mut router, punct(';'));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert!(!preedit(&frame).is_empty());
    assert_ne!(preedit(&frame), "?x");
}

#[test]
fn punctuation_toggle_is_remembered_per_mode() {
    let mut router = router_with(RouterConfig {
        status_enabled: true,
        ..RouterConfig::default()
    });
    let comma = KeyEvent::new(0xBC, Some(','), Default::default());
    let english_comma = KeyEvent::new(0xBC, Some(','), ENGLISH);
    // 中文模式下切成半角。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });
    router.handle_status_event(StatusEvent::TogglePunctuation);
    assert_eq!(press(&mut router, comma).0, KeyOutcome::Passthrough);
    // 英文模式缺省半角；点那一格切成全角，英文模式下真转。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
    });
    assert_eq!(press(&mut router, english_comma).0, KeyOutcome::Passthrough);
    router.handle_status_event(StatusEvent::TogglePunctuation);
    assert_eq!(press(&mut router, english_comma).1, Some("，".to_owned()));
    // 切回中文：还是中文自己记住的半角；再切回英文：还是英文记住的全角。
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: false,
    });
    assert_eq!(press(&mut router, comma).0, KeyOutcome::Passthrough);
    router.handle(ClientMessage::ModeChanged {
        session: SESSION,
        english: true,
    });
    assert_eq!(press(&mut router, english_comma).1, Some("，".to_owned()));
    // 英文候选组词中敲标点：先把字母原样上屏，标点也按英文那份转。
    type_english(&mut router, "hello");
    let (_, commit, _) = press(&mut router, english_comma);
    assert_eq!(commit.as_deref(), Some("hello，"));
}
