//! 中文组句：出候选、选词、上屏、翻页键、直输段、失焦与学习落盘。

use crate::support::*;
use qingjian_core::{
    CandidateKind, Prediction, PredictionKind, PredictionPolicy, PredictionRequest, Predictor,
};

pub(crate) struct SentenceTranslator {
    replies: Vec<Prediction>,

    submitted: Arc<Mutex<Vec<PredictionRequest>>>,
}

impl SentenceTranslator {
    pub(crate) fn new(submitted: Arc<Mutex<Vec<PredictionRequest>>>) -> Self {
        Self {
            replies: Vec::new(),
            submitted,
        }
    }
}

impl Predictor for SentenceTranslator {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            slots: 0,
            max_items: 0,
            sentence: false,
            ..PredictionPolicy::default()
        }
    }

    fn submit(&mut self, request: PredictionRequest) {
        self.submitted.lock().unwrap().push(request.clone());
        if request.kind == PredictionKind::Translate {
            self.replies.push(Prediction {
                sequence: request.sequence,
                words: Vec::new(),
                sentence: Some("It seems not to work.".to_owned()),
            });
        }
    }

    fn poll(&mut self) -> Option<Prediction> {
        self.replies.pop()
    }
}

#[test]
fn sentence_candidates_receive_an_async_translation_without_cloud_completion() {
    let submitted = Arc::new(Mutex::new(Vec::new()));
    let mut local = router_with(RouterConfig {
        cloud_slots: 0,
        sentence_translation: false,
        ..RouterConfig::default()
    });
    let (_, _, expected) = type_letters(&mut local, "xiangkaifa");
    let expected_texts = candidate_texts(&expected);

    let mut router = router_with(RouterConfig {
        cloud_slots: 0,
        sentence_translation: true,
        ..RouterConfig::default()
    });
    router
        .engine_mut()
        .set_predictor(Box::new(SentenceTranslator::new(submitted.clone())));

    let (_, _, frame) = type_letters(&mut router, "xiangkaifa");
    let sentence = frame
        .candidates
        .items
        .iter()
        .find(|candidate| candidate.kind == CandidateKind::Sentence)
        .expect("应有本地整句候选「想开发」");
    assert_eq!(sentence.text, "想开发");
    assert_eq!(
        sentence.translation.as_ref().unwrap().senses()[0].text,
        "It seems not to work."
    );
    assert_eq!(candidate_texts(&frame), expected_texts);
    assert_eq!(frame.candidates.items[0].text, "想开发");
    let (_, commit, _) = press(&mut router, letter(' '));
    assert_eq!(commit.as_deref(), Some("想开发"));
    let requests = submitted.lock().unwrap();
    let translations: Vec<&str> = requests
        .iter()
        .filter(|request| request.kind == PredictionKind::Translate)
        .map(|request| request.text.as_str())
        .collect();
    assert_eq!(translations.last(), Some(&"想开发"));
    assert!(translations.windows(2).all(|pair| pair[0] != pair[1]));
    assert_eq!(frame.sentence, None, "关闭整句补全时不应显示补全文本");
}

#[test]
fn disabled_sentence_translation_never_sends_a_translation_request() {
    let submitted = Arc::new(Mutex::new(Vec::new()));
    let mut router = router_with(RouterConfig {
        cloud_enabled: true,
        cloud_slots: 0,
        sentence_translation: false,
        ..RouterConfig::default()
    });
    router
        .engine_mut()
        .set_predictor(Box::new(SentenceTranslator::new(submitted.clone())));

    let _ = type_letters(&mut router, "xiangkaifa");

    assert!(
        submitted
            .lock()
            .unwrap()
            .iter()
            .all(|request| request.kind != PredictionKind::Translate)
    );
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

/// 拼音显示位置随帧下发给 DLL：DLL 按 `inline()` 决定要不要往应用里放行内拼音，
/// 窗口顶部画不画拼音行由 Server 自己按 `in_window()` 定，所以帧始终带着拼音行。
#[test]
fn frame_carries_the_preedit_mode() {
    for mode in PreeditMode::ALL {
        let mut router = router_with(RouterConfig {
            preedit: mode,
            ..RouterConfig::default()
        });
        let (_, _, frame) = type_letters(&mut router, "nihao");

        assert_eq!(frame.preedit_mode, mode);
        assert_eq!(
            preedit(&frame),
            "ni'hao",
            "{mode:?} 下帧也要带拼音行，画不画是窗口的事"
        );
    }
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
    // 组句中标点进直输段的老行为（`punct_commits` 关），`]` 才会留在拼音行里
    let mut router = router_with(RouterConfig {
        page_size: 1,
        page_keys: (',', '.'),
        punct_commits: false,
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
fn minus_equals_page_keys_preserve_expression_input() {
    let mut router = router_with(RouterConfig {
        page_size: 1,
        page_keys: ('-', '='),
        ..RouterConfig::default()
    });
    let (_, _, frame) = type_letters(&mut router, "ni");
    assert!(frame.page_count > 1);
    let (outcome, commit, frame) = press(&mut router, punct('='));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(frame.page, 1);
    assert_eq!(preedit(&frame), "ni");
    let (_, _, frame) = press(&mut router, punct('-'));
    assert_eq!(frame.page, 0);
    assert_eq!(preedit(&frame), "ni");
    press(&mut router, KeyEvent::new(0x1B, None, Default::default()));

    type_letters(&mut router, "v");
    for c in "2-1=".chars() {
        let (outcome, commit, _) = press(&mut router, punct(c));
        assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    }
    let (outcome, commit, _) = press(&mut router, punct(' '));
    assert_eq!(
        (outcome, commit),
        (KeyOutcome::Consumed, Some("2-1=1".to_owned()))
    );
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
    assert!(
        user_dir.join(qingjian_learning::USAGE_FILE).is_file(),
        "上屏时就该追加进输入统计"
    );
    let _ = std::fs::remove_dir_all(&user_dir);
}

#[test]
fn raw_segment_takes_digits_and_keeps_the_space() {
    // `-` 进英文直输段之后数字是内容不是选词键，空格整段原样上屏并保留空格（#28 排查时发现 `gpt-6` 丢了 6）。
    let mut router = router();
    let mut frame = Frame::default();
    for c in "gpt-6".chars() {
        let (outcome, commit, next) = press(&mut router, letter(c));
        assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
        frame = next;
    }
    assert_eq!(preedit(&frame), "gpt-6");
    let (outcome, commit, after) = press(&mut router, letter(' '));
    assert_eq!(
        (outcome, commit.as_deref()),
        (KeyOutcome::Consumed, Some("gpt-6 "))
    );
    assert!(after.is_empty());
}

#[test]
fn digit_without_a_slot_joins_the_buffer() {
    // 这一页没有第 9 格：数字是内容（`gpt9`），不再被静默吞掉；成了直输段之后空格整段上屏。
    let mut router = router();
    let (_, _, frame) = type_letters(&mut router, "gpt");
    let shown = frame.candidates.items.len();
    assert!(
        (1..9).contains(&shown),
        "样例词库下 gpt 的候选应不满 9 个：{shown}"
    );
    let (outcome, commit, frame) = press(&mut router, digit(9));
    assert_eq!((outcome, commit), (KeyOutcome::Consumed, None));
    assert_eq!(preedit(&frame), "gpt9");
    let (_, commit, after) = press(&mut router, letter(' '));
    assert_eq!(commit.as_deref(), Some("gpt9 "));
    assert!(after.is_empty());
    // 有这一格照常选词。
    let (_, _, frame) = type_letters(&mut router, "ni");
    let first = frame.candidates.items[0].text.clone();
    let (_, commit, _) = press(&mut router, digit(1));
    assert_eq!(commit, Some(first));
}

/// 双拼「输入框显示原始按键」：发给 DLL 的帧是敲的键、光标按键数算，自绘窗的拼音行照旧全拼。
#[test]
fn shuangpin_raw_preedit_goes_to_the_app_and_full_pinyin_to_the_window() {
    let mut router = router_with(RouterConfig {
        scheme: Scheme::Shuangpin(ShuangpinScheme::Xiaohe),
        ..RouterConfig::default()
    });
    router.engine_mut().set_shuangpin_raw_preedit(true);
    let sink = RecordingCandidates::default();
    router.set_candidate_sink(Box::new(sink.clone()));
    type_letters(&mut router, "kdfa");
    let _ = router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect: rect(),
    });
    press(&mut router, function_key(0x25));
    let (_, _, frame) = press(&mut router, function_key(0x25));
    assert_eq!((preedit(&frame).as_str(), frame.cursor), ("kdfa", 2));
    let shown = sink.0.lock().unwrap();
    let last = shown.last().expect("自绘窗收到过帧");
    assert_eq!((preedit(last).as_str(), last.cursor), ("kai'fa", 3));
}

/// 自绘窗随帧收到输入状态（给主题显示中 / 英、简 / 繁、方案），与配置一致。
#[test]
fn self_drawn_window_gets_input_mode() {
    let mut router = router_with(RouterConfig {
        scheme: Scheme::Shuangpin(ShuangpinScheme::Xiaohe),
        traditional: true,
        full_width: true,
        ..RouterConfig::default()
    });
    let sink = RecordingCandidates::default();
    router.set_candidate_sink(Box::new(sink.clone()));
    type_letters(&mut router, "ni");
    let _ = router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect: rect(),
    });
    let modes = sink.1.lock().unwrap();
    let mode = modes.last().expect("自绘窗收到过帧");
    assert!(!mode.english && mode.traditional && mode.full_width);
    assert_eq!(mode.scheme, "小鹤双拼");
}

#[test]
fn space_on_an_email_address_commits_it_without_the_space() {
    let mut router = router();
    type_letters(&mut router, "qingjian");
    let (_, _, frame) = press(&mut router, punct('@'));
    assert_eq!(
        candidate_texts(&frame)[..2],
        ["qingjian@", "qingjian@qq.com"]
    );
    press(&mut router, function_key(0x28));
    let (_, commit, after) = press(&mut router, letter(' '));
    assert_eq!(commit.as_deref(), Some("qingjian@qq.com"));
    assert!(after.is_empty());
}
