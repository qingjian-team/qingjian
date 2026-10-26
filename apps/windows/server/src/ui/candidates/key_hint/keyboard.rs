//! Core 给的双拼键位、五笔字根图换成渲染器的键盘（与 macOS 壳 `host::presenting::key_hint` 一致）。

use qingjian_core::root_chart::RootChart;
use qingjian_core::shuangpin::{KeyHint, KeyState};
use qingjian_render::{Keyboard, Keycap, KeycapState, Root};

pub(super) fn shuangpin(hint: &KeyHint) -> Keyboard {
    let rows = hint
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cap| Keycap {
                    label: cap.key.to_ascii_uppercase().to_string(),
                    corner: cap.initial.clone(),
                    lines: cap.finals.clone(),
                    state: match &cap.state {
                        KeyState::Idle => KeycapState::Normal,
                        KeyState::Pressed => KeycapState::Pressed,
                        KeyState::Next(syllable) => KeycapState::Next(syllable.clone()),
                        KeyState::Unavailable => KeycapState::Dimmed,
                    },
                    roots: Vec::new(),
                })
                .collect()
        })
        .collect();
    Keyboard { rows }
}

/// 字根图：每键底下一行键名与区位号，Z 是万能键。
pub(super) fn roots(chart: &RootChart) -> Keyboard {
    let rows = chart
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|key| Keycap {
                    label: key.key.to_string(),
                    corner: key.zone.map(|zone| zone.to_string()),
                    lines: if key.roots.is_empty() {
                        vec!["万能键".to_owned()]
                    } else {
                        Vec::new()
                    },
                    roots: key
                        .roots
                        .iter()
                        .filter_map(|glyph| Root::from_svg(glyph.svg, glyph.frame))
                        .collect(),
                    ..Keycap::default()
                })
                .collect()
        })
        .collect();
    Keyboard { rows }
}
