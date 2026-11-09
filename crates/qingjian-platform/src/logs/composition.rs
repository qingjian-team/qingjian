//! 一段组句的按键账：只记数、不记内容，组句结束时在 info 级记一行。
//!
//! 缺省日志不记敲了什么，用户报「字母跟着上屏」「退格删了正文」这类应用兼容问题时，靠这一行看出
//! 是哪个应用、拼音放在哪、敲了几个键、怎么结束的、交给应用几个字。三个桌面壳共用。

use std::time::Instant;

use crate::PreeditMode;
use crate::protocol::linux::LinuxEvent;
use crate::protocol::{ClientMessage, KeyEvent, KeyOutcome, ServerMessage};

/// Windows 虚拟键码（`VK_*`），Linux Server 也用这套。
const VK_BACK: u32 = 0x08;

pub struct CompositionTrace {
    /// 这段组句收到的按键数（含开头那个键与结束那个键）。
    keys: u32,

    /// 其中的退格数。
    backspaces: u32,

    /// 交给应用的字数（中途上屏的词与结束时上屏的都算）。
    inserted: usize,

    started: Instant,
}

impl CompositionTrace {
    fn new() -> Self {
        Self {
            keys: 0,
            backspaces: 0,
            inserted: 0,
            started: Instant::now(),
        }
    }

    /// 一个事件处理完后记账：`before` / `after` 是事件前后在不在组句，`key` 是按键事件时它是不是退格
    /// （点选候选、切走这类事件为 `None`），`inserted` 是这个事件交给应用的字数。
    /// 返回组句是不是在这个事件里结束了，是的话调用方接着 [`Self::end`]。
    pub fn step(
        slot: &mut Option<Self>,
        before: bool,
        after: bool,
        key: Option<bool>,
        inserted: usize,
    ) -> bool {
        if before || after {
            let trace = slot.get_or_insert_with(Self::new);
            if let Some(backspace) = key {
                trace.keys += 1;
                trace.backspaces += u32::from(backspace);
            }
            trace.inserted += inserted;
        }
        before && !after
    }

    /// 服务端（Windows / Linux）处理完一条消息后记账。`composing` 是处理前后在不在组句，`label` 来自
    /// [`Self::message_label`] / [`Self::linux_label`]。组句在两条消息之间结束（Windows 点候选窗口上屏）时，
    /// 这条回话带走的字算它的，按 `elsewhere` 记。
    pub fn observe(
        slot: &mut Option<Self>,
        (before, after): (bool, bool),
        (key, end): (Option<bool>, &str),
        reply: Option<&ServerMessage>,
        app: Option<&str>,
        preedit: PreeditMode,
    ) {
        let (inserted, consumed) = Self::reply_effect(reply);
        if !before && slot.is_some() {
            Self::step(slot, true, false, None, inserted);
            Self::end(slot, app, preedit, "elsewhere", true);
            Self::step(slot, false, after, key, 0);
        } else if Self::step(slot, before, after, key, inserted) {
            Self::end(slot, app, preedit, end, consumed);
        }
    }

    /// 服务端（Windows / Linux）的一条消息在账里算什么：按键时它是不是退格（不是按键为 `None`）与结束标签。
    pub fn message_label(message: &ClientMessage) -> (Option<bool>, &'static str) {
        match message {
            ClientMessage::Key { event, .. } => {
                (Some(event.virtual_key == VK_BACK), Self::key_label(event))
            }
            ClientMessage::Commit { .. }
            | ClientMessage::HideCandidates { .. }
            | ClientMessage::CloseSession { .. } => (None, "focus-out"),
            // Windows 点候选窗口上屏的字随下一拍轮询带走
            ClientMessage::Poll { .. } => (None, "click"),
            _ => (None, "other"),
        }
    }

    /// Linux 插件事件的同一件事；松开键只在单击切换键时有动作（切中英、拼音原样上屏）。
    pub fn linux_label(event: &LinuxEvent) -> (Option<bool>, &'static str) {
        match event {
            LinuxEvent::Key {
                event,
                release: false,
                ..
            } => (Some(event.virtual_key == VK_BACK), Self::key_label(event)),
            LinuxEvent::Key { release: true, .. } => (None, "switch-mode"),
            LinuxEvent::Candidate { .. } => (None, "click"),
            LinuxEvent::Deactivate { .. } | LinuxEvent::Reset => (None, "focus-out"),
            _ => (None, "other"),
        }
    }

    /// 回话里交给应用的字数，与按键有没有被吞（不是按键结果的回话算吞了）。
    fn reply_effect(reply: Option<&ServerMessage>) -> (usize, bool) {
        let chars = |text: &Option<String>| text.as_deref().map_or(0, |t| t.chars().count());
        match reply {
            Some(ServerMessage::KeyResult {
                outcome, commit, ..
            }) => (chars(commit), *outcome == KeyOutcome::Consumed),
            Some(ServerMessage::Committed { text, .. }) => (chars(text), true),
            Some(ServerMessage::Update { commit, .. }) => (chars(commit), true),
            _ => (0, true),
        }
    }

    /// 按键的类别，不带具体是哪个字母或标点。
    fn key_label(event: &KeyEvent) -> &'static str {
        match (event.virtual_key, event.character) {
            (0x20, _) => "space",
            (0x0D, _) => "enter",
            (VK_BACK, _) => "backspace",
            (0x1B, _) => "esc",
            (0x09, _) => "tab",
            (_, Some(c)) if c.is_ascii_digit() => "digit",
            (_, Some(c)) if c.is_ascii_alphabetic() => "letter",
            (_, Some(c)) if c.is_ascii_punctuation() => "punct",
            _ => "other",
        }
    }

    /// 结束这段组句的账并记一行；没在记账时什么也不做。`end` 是结束它的键或事件，`consumed` 是有没有吞掉那个键
    /// （事件结束的填 `true`）。组句被按键以外的事结束、壳到下一个事件才发现时也用它补记。
    pub fn end(
        slot: &mut Option<Self>,
        app: Option<&str>,
        preedit: PreeditMode,
        end: &str,
        consumed: bool,
    ) {
        let Some(trace) = slot.take() else {
            return;
        };
        tracing::info!(
            app = app.unwrap_or("-"),
            ?preedit,
            keys = trace.keys,
            backspaces = trace.backspaces,
            inserted = trace.inserted,
            end,
            consumed,
            ms = trace.started.elapsed().as_millis() as u64,
            "composition ended"
        );
    }
}
