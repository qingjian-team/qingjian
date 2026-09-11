//! `ITfKeyEventSink`：所有键先经 `OnTestKeyDown` 判吃不吃（[`TextService_Impl::would_eat`]，与 Router 的分派对齐），
//! 吃的键在 `OnKeyDown` 里转发给 Server 并按结果更新文档；单击 Shift 的判定与保留键命中也在这里。

use windows::Win32::Foundation::{FALSE, LPARAM, WPARAM};
use windows::Win32::UI::TextServices::{ITfContext, ITfKeyEventSink_Impl};
use windows::core::{BOOL, GUID, Ref, Result};

use qingjian_platform::protocol::{KeyEvent, KeyOutcome};

use super::TextService_Impl;
use super::next::Next;
use crate::client::KeyReply;
use crate::com::composition::preedit_string;
use crate::com::key::event::{digit_key, is_edit, is_letter, is_nav, to_key_event};
use crate::com::key::preserved;
use crate::com::log::log;

impl ITfKeyEventSink_Impl for TextService_Impl {
    /// 获焦：补一次连接（Server 起晚了 / 重启过），并刷指示器（系统会在切换焦点时重置它）。
    /// 失焦：把敲了一半的拼音原样落定（对应 macOS 的 `commitComposition`）。
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        self.shared.set_foreground(fforeground.as_bool());
        if fforeground.as_bool() {
            self.ensure_connected();
            self.refresh_mode_indicator();
        } else {
            self.commit_pending();
        }
        Ok(())
    }

    /// 所有键（含之后被吃掉的）都先经过这里，Shift 单击的判定放在这一层。
    fn OnTestKeyDown(&self, _pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let vk = wparam.0 as u32;
        self.note_key_down(vk, lparam);
        Ok(self.would_eat(&self.key_event(vk)).into())
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let vk = wparam.0 as u32;
        self.note_key_down(vk, lparam);
        let event = self.key_event(vk);
        Ok(self.handle_key(pic, event).into())
    }

    fn OnTestKeyUp(&self, _pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        self.note_key_up(wparam.0 as u32);
        Ok(FALSE)
    }

    fn OnKeyUp(&self, _pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        self.note_key_up(wparam.0 as u32);
        Ok(FALSE)
    }

    /// 翻译选中文字的保留键命中：当作按下了那个组合键转发给 Server（绕过 `would_eat`）。
    fn OnPreservedKey(&self, pic: Ref<ITfContext>, rguid: *const GUID) -> Result<BOOL> {
        if unsafe { *rguid } != preserved::GUID_TRANSLATE {
            return Ok(FALSE);
        }
        let Some(combo) = self.translate_combo.get() else {
            return Ok(FALSE);
        };
        let event = preserved::key_event(combo, self.mode_state.english());
        Ok(self.forward_key(pic, event).into())
    }
}

impl TextService_Impl {
    fn key_event(&self, vk: u32) -> KeyEvent {
        to_key_event(vk, self.mode_state.english())
    }

    fn note_key_down(&self, vk: u32, lparam: LPARAM) {
        self.shift_tap.key_down(vk, lparam);
    }

    fn note_key_up(&self, vk: u32) {
        if self.shift_tap.key_up(vk) {
            self.set_english_mode(!self.mode_state.english());
        }
    }

    /// 这个键吃不吃，与 Router 的分派对齐；`OnTestKeyDown` 用，无副作用。
    /// 带 Ctrl/Alt/Win 只有组句中的「修饰键 + 数字」送 Server（译词 / 删候选），其余归应用（翻译选中文字走保留键）；
    /// 字母只有「中文模式、没在组句、按住 Shift 的大写」归应用；组句中功能键 / 方向键 / 可打印字符都吃；
    /// 没在组句时数字 / 标点也先「测吃」送去转全角（中英各有一份开关），Server 不转的回 Passthrough 再放行；`?` 是问字前缀。
    fn would_eat(&self, event: &KeyEvent) -> bool {
        // 翻译评审中所有键先吃进来交给 Server 定接受 / 取消。
        if self.shared.translating() {
            return true;
        }
        let modifiers = event.modifiers;
        if modifiers.has_command_key() {
            return self.shared.composing() && digit_key(event.virtual_key);
        }
        let vk = event.virtual_key;
        if is_letter(vk) {
            return modifiers.caps
                || modifiers.english_mode
                || !modifiers.shift
                || self.shared.composing();
        }
        if self.shared.composing() {
            return is_edit(vk) || is_nav(vk) || event.character.is_some_and(|c| !c.is_control());
        }
        event
            .character
            .is_some_and(|c| c.is_ascii_punctuation() || c.is_ascii_digit())
    }

    /// 不吃的键绝不碰组句（否则光标一移，组句会把拼音重插到别处）。
    fn handle_key(&self, pic: Ref<ITfContext>, event: KeyEvent) -> bool {
        if !self.would_eat(&event) {
            return false;
        }
        self.forward_key(pic, event)
    }

    /// 把按键送给 Server 并按结果更新文档；返回吃不吃。
    fn forward_key(&self, pic: Ref<ITfContext>, event: KeyEvent) -> bool {
        // 没连上 Server：快捷键组合归应用（别吞了 Ctrl+C），其余吃掉别让拼音漏进应用。
        if !self.ensure_connected() {
            return !event.modifiers.has_command_key();
        }
        if let Ok(context) = pic.ok() {
            self.shared.set_last_context(Some(context.clone()));
        }
        // Server 交互在这段借用里做完，放掉借用再走编辑会话。
        let next = {
            let mut guard = self.engine.borrow_mut();
            let Some(client) = guard.as_mut() else {
                return true;
            };
            // 组句被应用终止过：先让 Server 清掉残留的拼音（文本已在文档里，交出的丢弃）。
            let response = if self.shared.take_server_stale() {
                client.commit().and_then(|_| client.key(event))
            } else {
                client.key(event)
            };
            match response {
                Ok(KeyReply::Result(response)) => {
                    let preedit = preedit_string(&response.frame);
                    self.shared.set_composing(!response.frame.is_empty());
                    // 翻译评审的任何键都结束评审（Server 侧已同步结束）。
                    self.shared.set_translating(false);
                    let consumed = matches!(response.outcome, KeyOutcome::Consumed);
                    let m = event.modifiers;
                    log(&format!(
                        "收键 vk={} ctrl={} alt={} shift={} caps={} en={} char={:?} candidates={} preedit={preedit:?} consumed={consumed}",
                        event.virtual_key,
                        m.ctrl,
                        m.alt,
                        m.shift,
                        m.caps,
                        m.english_mode,
                        event.character,
                        response.frame.candidates.items.len()
                    ));
                    Next::Document {
                        commit: response.commit,
                        preedit,
                        consumed,
                    }
                }
                Ok(KeyReply::NeedSelection { request }) => {
                    log(&format!("翻译选中文字：Server 请读选区 request={request}"));
                    Next::ReadSelection { request }
                }
                Err(error) => {
                    log(&format!("转发按键失败，放行并断开，下一键重连: {error}"));
                    *guard = None;
                    self.last_connect_failure.set(None);
                    self.shared.end_composing();
                    Next::Abort
                }
            }
        };
        match next {
            // 放行的键 Server 没动缓冲区，不碰文档（应用处理这个键时光标可能会移）。
            Next::Document {
                consumed: false, ..
            } => false,
            Next::Document {
                commit, preedit, ..
            } => {
                self.update_document(pic, commit, preedit);
                true
            }
            // 读选区是异步的：先吃掉这个键，选区文本在回调里发给 Server。
            Next::ReadSelection { request } => {
                self.read_selection(pic, request);
                true
            }
            Next::Abort => false,
        }
    }
}
