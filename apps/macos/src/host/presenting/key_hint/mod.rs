//! 键位提示：按 `[general] key_hint` 决定这一帧画不画键盘，把 Core 给的双拼键位或五笔字根图换成渲染器的键盘。

mod state;
mod timer;

pub use state::KeyHintState;

use objc2::MainThreadMarker;
use qingjian_core::root_chart::RootChart;
use qingjian_core::shuangpin::{KeyHint, KeyState};
use qingjian_platform::KeyHintMode;
use qingjian_render::{Keyboard, Keycap, KeycapState, Root};

use super::super::Host;

impl Host {
    /// 这一帧的键盘；不该画时为 `None`。开着五笔画字根图（混输也是），否则画双拼键位。
    /// 组句结束（缓冲区空了）时清掉「卡住时显示」的状态。
    pub(in crate::host) fn key_hint_keyboard(&mut self) -> Option<Keyboard> {
        let composing = self.key_hint_mode != KeyHintMode::Off
            && self.translation.is_none()
            && !self.engine.composition().is_empty();
        let chart = composing.then(|| self.engine.root_chart()).flatten();
        let hint = (composing && chart.is_none())
            .then(|| self.engine.key_hint())
            .flatten();
        if chart.is_none() && hint.is_none() {
            self.key_hint = KeyHintState::default();
            return None;
        }
        // 字根图上没有「拼不成」的说法，只等停顿
        let stuck = hint.as_ref().is_some_and(|hint| hint.stuck);
        let show = match self.key_hint_mode {
            KeyHintMode::Always => true,
            KeyHintMode::Auto => {
                let mtm = MainThreadMarker::new().expect("Host 只在主线程用");
                let typed = self.engine.composition().text().to_owned();
                self.key_hint.update(mtm, &typed, stuck)
            }
            KeyHintMode::Off => false,
        };
        if !show {
            return None;
        }
        match (chart, hint) {
            (Some(chart), _) => Some(
                self.root_keyboard
                    .get_or_insert_with(|| root_keyboard(chart))
                    .clone(),
            ),
            (None, Some(hint)) => Some(keyboard(&hint)),
            (None, None) => None,
        }
    }

    /// 停顿计时到点：还在组句就弹出键盘。
    pub fn key_hint_hesitated(&mut self) {
        if self.engine.composition().is_empty() {
            return;
        }
        self.key_hint.hesitated();
        self.render();
    }
}

fn keyboard(hint: &KeyHint) -> Keyboard {
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

/// 字根图换成键盘：每键底下一行键名与区位号，Z 是万能键。字形解析一次，Host 留着复用。
fn root_keyboard(chart: &RootChart) -> Keyboard {
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
