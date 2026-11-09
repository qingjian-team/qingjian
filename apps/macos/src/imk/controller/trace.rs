//! 组句按键账的起止：每个按键前后看组句开始还是结束，结束时记一行（见 [`CompositionTrace`]）。

use objc2_app_kit::NSEvent;
use qingjian_platform::logs::CompositionTrace;

use super::{QingjianInputController, digit_key};
use crate::host;
use crate::imk::{TextClient, take_inserted};

impl QingjianInputController {
    /// 按下键的分发外面包一层账。组句被鼠标点选、切中英这类按键以外的事结束时，到下一个键才补记。
    pub(super) fn dispatch_traced(&self, event: &NSEvent, client: TextClient<'_>) -> bool {
        let before = composing();
        if !before {
            self.end_trace("elsewhere", true);
        }
        take_inserted();
        let consumed = self.dispatch_event(event, client);
        let inserted = take_inserted();
        let after = composing();
        let ended = host::with(|h| {
            CompositionTrace::step(
                &mut h.trace,
                before,
                after,
                Some(event.keyCode() == 51),
                inserted,
            )
        })
        .unwrap_or(false);
        if ended {
            self.end_trace(key_label(event), consumed);
        }
        consumed
    }

    /// 结束这段组句的账并记日志；没在记账时什么也不做。`end` 与 `consumed` 见 [`CompositionTrace::end`]。
    pub(super) fn end_trace(&self, end: &str, consumed: bool) {
        let inserted = take_inserted();
        host::with(|h| {
            if h.trace.is_some() {
                CompositionTrace::step(&mut h.trace, true, false, None, inserted);
            }
            let app = h.engine.application().map(str::to_owned);
            CompositionTrace::end(&mut h.trace, app.as_deref(), h.preedit_mode, end, consumed);
        });
    }
}

fn composing() -> bool {
    host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false)
}

/// 按键的类别，不带具体是哪个字母或标点；与 Windows / Linux 的标签一致。
fn key_label(event: &NSEvent) -> &'static str {
    let key = event.keyCode();
    match key {
        49 => "space",
        36 | 76 => "enter",
        51 => "backspace",
        53 => "esc",
        48 => "tab",
        _ if digit_key(key).is_some() => "digit",
        _ => match event
            .charactersIgnoringModifiers()
            .and_then(|c| c.to_string().chars().next())
        {
            Some(c) if c.is_ascii_alphabetic() => "letter",
            Some(c) if c.is_ascii_punctuation() => "punct",
            _ => "other",
        },
    }
}
