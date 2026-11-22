//! 候选右键菜单：右键一格给出这格能做的几项，点中一项就做、提示随下一帧下发、下一次按键清。

use qingjian_platform::candidate_menu::CandidateAction;
use qingjian_platform::{Config, KeyCombo};

use crate::support::*;

#[test]
fn right_click_offers_actions_and_a_pick_shows_a_notice_until_next_key() {
    bamboo_slip_i18n::pin_locale("zh-Hans");
    let mut router = router();
    let sink = RecordingCandidates::default();
    router.set_candidate_sink(Box::new(sink.clone()));
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好") as usize - 1;
    let _ = router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect: rect(),
    });

    router.handle_candidate_mouse(CandidateMouse::Menu {
        offset: slot,
        point: (10, 10),
    });
    let menu = sink.3.lock().unwrap().pop().expect("右键应弹出菜单");
    assert_eq!(menu.offset, slot);
    let labels: Vec<_> = menu.items.iter().map(|(_, label)| *label).collect();
    assert!(
        labels.contains(&"降权") && labels.contains(&"删除"),
        "{labels:?}"
    );

    // 「你好」是词库词且没学习记录，删不掉，但提示照样给出，组句不变
    router.handle_candidate_mouse(CandidateMouse::MenuPick {
        offset: slot,
        id: CandidateAction::Delete.id(),
    });
    let shown = sink.0.lock().unwrap().last().cloned().expect("做完要重画");
    let notice = shown.notice.expect("做完应带屏幕提示");
    assert!(notice.contains("你好"), "{notice}");
    let (_, _, after) = press(&mut router, KeyEvent::new(0x10, None, Default::default())); // VK_SHIFT 不动组句
    assert_eq!(after.notice, None, "提示应只活到下一次按键");
    assert_eq!(preedit(&after), "ni'hao");
}

#[test]
fn picking_a_single_char_commits_it_and_ends_the_pinyin() {
    // 右键「你好」选「只上屏第一个字」：上屏「你」，这个词的整段拼音一并结束（#1026 评审第 3 条）
    bamboo_slip_i18n::pin_locale("zh-Hans");
    let mut router = router();
    let sink = RecordingCandidates::default();
    router.set_candidate_sink(Box::new(sink.clone()));
    let (_, _, frame) = type_letters(&mut router, "nihao");
    let slot = slot_of(&frame, "你好") as usize - 1;
    let _ = router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect: rect(),
    });

    router.handle_candidate_mouse(CandidateMouse::Menu {
        offset: slot,
        point: (10, 10),
    });
    let menu = sink.3.lock().unwrap().pop().expect("右键应弹出菜单");
    assert!(
        menu.items
            .iter()
            .any(|(id, _)| *id == CandidateAction::FirstChar.id()),
        "多字候选要出「只上屏第一个字」"
    );

    router.handle_candidate_mouse(CandidateMouse::MenuPick {
        offset: slot,
        id: CandidateAction::FirstChar.id(),
    });
    // 取字用的是这个词的读音：整段拼音结束，上屏的字挂到会话上等下一拍轮询带走
    let (after, commit) = match router.handle(ClientMessage::Poll { session: SESSION }) {
        Some(ServerMessage::Update { frame, commit, .. }) => (frame, commit),
        other => panic!("expected Update, got {other:?}"),
    };
    assert_eq!(commit.as_deref(), Some("你"));
    assert!(
        after.is_empty(),
        "取字后整段拼音结束：{:?}",
        preedit(&after)
    );
}

/// 以词定字的快捷键：`[shortcut] pick_char` 配成 Ctrl+E（Ctrl+D 是缺省的单字模式），组句中按下取高亮候选的第一个字，
/// 带 Shift 取最后一个字，两下都把整段拼音吃掉。
#[test]
fn pick_char_shortcut_takes_the_first_or_last_char() {
    let mut router = router_with(RouterConfig {
        pick_char: Some(KeyCombo {
            modifiers: qingjian_platform::Modifiers::CONTROL,
            key: 'e',
        }),
        ..RouterConfig::default()
    });
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    assert_eq!(frame.candidates.items[0].text, "开发");

    // Ctrl+E：取第一个字
    let (outcome, committed, frame) = press(
        &mut router,
        letter_with(
            'E',
            KeyModifiers {
                ctrl: true,
                ..KeyModifiers::default()
            },
        ),
    );
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(committed.as_deref(), Some("开"));
    assert_eq!(preedit(&frame), "", "整段拼音都吃掉了");

    // Ctrl+Shift+E：取最后一个字
    type_letters(&mut router, "kaifa");
    let (outcome, committed, frame) = press(
        &mut router,
        letter_with(
            'E',
            KeyModifiers {
                ctrl: true,
                shift: true,
                ..KeyModifiers::default()
            },
        ),
    );
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert_eq!(committed.as_deref(), Some("发"));
    assert_eq!(preedit(&frame), "");
}

/// 快捷键配成翻页键时不生效（评审要求：用户换了哪组翻页键，这个键也不能再用那一组）。
///
/// 这条挡在 `RouterConfig::from(&Config)` 那一步：手改配置文件把 `pick_char` 写成 `control+[`
/// 时，有效值直接是 `None`，于是 Ctrl+[ 照旧是翻页。
#[test]
fn pick_char_shortcut_yields_to_page_keys() {
    let mut config = Config::default();
    config.general.page_keys = "[]".to_owned();
    config.shortcut.pick_char = Some(KeyCombo {
        modifiers: qingjian_platform::Modifiers::CONTROL,
        key: '[',
    });
    assert!(
        RouterConfig::from(&config).pick_char.is_none(),
        "配成翻页键就不该生效"
    );

    // 换成不撞翻页键的字符就生效
    config.shortcut.pick_char = Some(KeyCombo {
        modifiers: qingjian_platform::Modifiers::CONTROL,
        key: 'e',
    });
    assert_eq!(
        RouterConfig::from(&config).pick_char,
        Some(KeyCombo {
            modifiers: qingjian_platform::Modifiers::CONTROL,
            key: 'e',
        })
    );
}
