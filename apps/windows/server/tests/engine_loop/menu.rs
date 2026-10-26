//! 候选右键菜单：右键一格给出这格能做的几项，点中一项就做、提示随下一帧下发、下一次按键清。

use qingjian_platform::candidate_menu::CandidateAction;

use crate::support::*;

#[test]
fn right_click_offers_actions_and_a_pick_shows_a_notice_until_next_key() {
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
