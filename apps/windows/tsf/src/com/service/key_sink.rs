//! `ITfKeyEventSink`：所有键先经 `OnTestKeyDown` 判吃不吃（[`TextService_Impl::would_eat`]，与 Router 的分派对齐），
//! 吃的键在 `OnKeyDown` 里转发给 Server 并按结果更新文档；单击中英切换键（`[shortcut] switch_mode`）的判定与保留键命中也在这里。
//! 上下文禁了键盘（密码框，见 [`context`](crate::com::context)）时没在组句的键一律放行。

use windows::Win32::Foundation::{FALSE, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_CAPITAL;
use windows::Win32::UI::TextServices::{ITfContext, ITfKeyEventSink_Impl};
use windows::core::{BOOL, GUID, Ref, Result};

use qingjian_platform::protocol::{KeyEvent, KeyOutcome};

use super::TextService_Impl;
use super::next::Next;
use crate::client::KeyReply;
use crate::com::composition::preedit_string;
use crate::com::key::event::{digit_key, is_edit, is_letter, is_mode_letter, is_nav, to_key_event};
use crate::com::key::preserved;
use crate::com::log::log;

impl ITfKeyEventSink_Impl for TextService_Impl {
    /// 失焦：把敲了一半的拼音原样落定（对应 macOS 的 `commitComposition`）。焦点本身交给
    /// [`TextService_Impl::set_thread_focus`]；切窗口时这条回调不触发，靠的是 [`crate::com::focus`]。
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        let foreground = fforeground.as_bool();
        if !foreground {
            self.commit_pending();
        }
        self.set_thread_focus(foreground);
        Ok(())
    }

    /// 所有键（含之后被吃掉的）都先经过这里，Shift 单击的判定放在这一层。
    fn OnTestKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let vk = wparam.0 as u32;
        self.note_key_down(vk, lparam);
        if self.keyboard_disabled(&pic) {
            return Ok(FALSE);
        }
        Ok(self.would_eat(&self.key_event(vk)).into())
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let vk = wparam.0 as u32;
        self.note_key_down(vk, lparam);
        if self.keyboard_disabled(&pic) {
            return Ok(FALSE);
        }
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

    /// 保留键命中：Ctrl+Space 直接切中英（切换键在 DLL 侧，不进 Server）；
    /// 「翻译选中文字」当作按下了那个组合键转发给 Server（绕过 `would_eat`）。
    fn OnPreservedKey(&self, pic: Ref<ITfContext>, rguid: *const GUID) -> Result<BOOL> {
        let guid = unsafe { *rguid };
        log(&format!("保留键命中 guid={guid:?}"));
        if guid == preserved::GUID_SWITCH_MODE {
            if self.keyboard_disabled(&pic) {
                return Ok(FALSE);
            }
            self.set_english_mode(!self.mode_state.english());
            return Ok(true.into());
        }
        if guid != preserved::GUID_TRANSLATE || self.keyboard_disabled(&pic) {
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
    /// 没在组句时看上下文有没有禁键盘（密码框）：禁了整键放行、不组句。组句中不看——那段组句是我们自己的，
    /// 应用要禁会先终止它。每键两次 compartment 读取，微秒级。
    fn keyboard_disabled(&self, pic: &Ref<ITfContext>) -> bool {
        if self.shared.composing() {
            return false;
        }
        let Ok(context) = pic.ok() else {
            return false;
        };
        let disabled = crate::com::context::keyboard_disabled(context);
        if disabled {
            log("上下文禁用键盘（密码框），放行");
        }
        disabled
    }

    fn key_event(&self, vk: u32) -> KeyEvent {
        to_key_event(vk, self.mode_state.english())
    }

    fn note_key_down(&self, vk: u32, lparam: LPARAM) {
        self.key_tap
            .key_down(vk, lparam, self.mode_state.switch_keys());
    }

    fn note_key_up(&self, vk: u32) {
        if vk == u32::from(VK_CAPITAL.0) {
            self.mode_state.notify();
        }
        if self.key_tap.key_up(vk, self.mode_state.switch_keys()) {
            self.set_english_mode(!self.mode_state.english());
        }
    }

    /// 这个键吃不吃，与 Router 的分派对齐；`OnTestKeyDown` 用，无副作用。判定见 [`eats_key`]。
    ///
    /// 带 Ctrl/Alt/Win 只有组句中的「修饰键 + 数字」送 Server（译词 / 删候选），其余归应用（翻译选中文字走保留键）；
    /// 字母只有「中文模式、没在组句、按住 Shift 的大写」归应用（`[general] shift_letter = "compose"` 时也吃，
    /// 让它起一段组句），其中 V / U / I 仍送 Server：双拼下是表达式 / 问字入口；
    /// 组句中功能键 / 方向键 / 可打印字符都吃；没在组句时数字 / 标点也先「测吃」送去转全角（中英各有一份开关），
    /// Server 不转的回 Passthrough 再放行；`?` 是问字前缀。
    fn would_eat(&self, event: &KeyEvent) -> bool {
        let shift_letter_compose = self
            .input_settings
            .get()
            .is_some_and(|input| input.shift_letter_compose);
        eats_key(
            event,
            self.shared.composing(),
            self.shared.translating(),
            shift_letter_compose,
        )
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
        // 没连上 Server（没起、刚重启、转发失败后的退避期）：只吃「可能是在打拼音」的键，
        // 别让拼音字母漏进应用；标点 / 数字 / 英文与 Caps 下的字母本来就会原样交给应用，
        // 这里放行——一律吃掉会表现为「按了没反应」（连不上时按 `-`、数字都没反应）。
        if !self.ensure_connected() {
            let eat = eats_without_server(&event);
            if eat {
                log(&format!(
                    "没连上 Server，吃掉 vk={} char={:?}",
                    event.virtual_key, event.character
                ));
            }
            return eat;
        }
        // OnTestKeyDown 已声明吃的可打印字符，Server 放行时由输入法自己插入：退回应用的话，企业微信 /
        // 微信 / notepad++ 这类自绘输入框会把它丢掉。功能键（无字符）仍交给应用。
        let passthrough_char = event.character.filter(|c| !c.is_control());
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
                    // 「只在候选窗口」模式应用里不放行内拼音（那一行由 Server 画在候选窗口顶部）。
                    let preedit = if response.frame.preedit_mode.inline() {
                        preedit_string(&response.frame)
                    } else {
                        String::new()
                    };
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
        // 带 Ctrl / Alt / Win 的组合（翻译保留键）放行时仍交还应用，别把热键的字母插进文档。
        let insertable = !event.modifiers.has_command_key();
        match (next, passthrough_char) {
            // 放行 + 没在组句 + 可打印字符：输入法插入，吃掉；Server 顺带交出的英文直输段字母拼在前面。
            (
                Next::Document {
                    consumed: false,
                    commit,
                    preedit,
                },
                Some(c),
            ) if insertable && preedit.is_empty() => {
                let mut text = commit.unwrap_or_default();
                text.push(c);
                self.update_document(pic, Some(text), String::new());
                true
            }
            // 放行的功能键：Server 没动缓冲区，交还应用（应用处理这个键时光标可能会移）。
            (
                Next::Document {
                    consumed: false, ..
                },
                _,
            ) => false,
            (
                Next::Document {
                    commit, preedit, ..
                },
                _,
            ) => {
                self.update_document(pic, commit, preedit);
                true
            }
            // 读选区是异步的：先吃掉这个键，选区文本在回调里发给 Server。
            (Next::ReadSelection { request }, _) => {
                self.read_selection(pic, request);
                true
            }
            (Next::Abort, _) => false,
        }
    }
}

/// 连不上 Server 时这个键吃不吃。
///
/// 只有「中文模式下可能是拼音」的字母要吃掉：漏进应用会变成一串字母，比什么都不出更难看。
/// 标点 / 数字（会话外本来就走 Passthrough 交给应用）与英文模式、Caps 亮着时敲的字母都放行——
/// 一律吃掉会表现为「按了没反应」，而且连日志都不留（这就是「中文模式按 `-` 偶发无反应」的成因：
/// 断连到重连成功之间的键全被吞了）。
fn eats_without_server(event: &KeyEvent) -> bool {
    !event.modifiers.has_command_key()
        && is_letter(event.virtual_key)
        && !(event.modifiers.caps || event.modifiers.english_mode)
}

/// 这个键吃不吃（[`TextService_Impl::would_eat`] 的纯逻辑，便于单测）。
///
/// - 翻译评审中一律吃，交给 Server 定接受 / 取消；
/// - 带 Ctrl / Alt / Win：只有组句中的「修饰键 + 数字」吃（译词 / 删候选），其余归应用（翻译选中文字走保留键）；
/// - 字母只有「中文模式、没在组句、按住 Shift 的大写」归应用，其中 V / U / I 仍吃：双拼下是表达式 / 问字入口。
///   `[general] shift_letter = "compose"`（Server 经 [`InputSettings`](qingjian_platform::protocol::InputSettings) 下发）时这种大写也吃：送去 Core 起一段组句，
///   `⇧C` 接 `pan` 才能出「C盘」；组句一开始，后面的 Shift 字母本来就被 `composing` 兜住；
/// - 组句中功能键 / 方向键 / 可打印字符都吃；
/// - 没在组句时数字 / 标点也先「测吃」送去转全角（中英各有一份开关），Server 不转的回 Passthrough 再放行；`?` 是问字前缀。
fn eats_key(
    event: &KeyEvent,
    composing: bool,
    translating: bool,
    shift_letter_compose: bool,
) -> bool {
    if translating {
        return true;
    }
    let modifiers = event.modifiers;
    if modifiers.has_command_key() {
        return composing && digit_key(event.virtual_key);
    }
    let vk = event.virtual_key;
    if is_letter(vk) {
        return modifiers.caps
            || modifiers.english_mode
            || !modifiers.shift
            || composing
            || shift_letter_compose
            || is_mode_letter(vk);
    }
    if composing {
        return is_edit(vk) || is_nav(vk) || event.character.is_some_and(|c| !c.is_control());
    }
    event
        .character
        .is_some_and(|c| c.is_ascii_punctuation() || c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use qingjian_platform::protocol::{KeyEvent, KeyModifiers};

    use super::{eats_key, eats_without_server};
    use crate::com::key::event::to_key_event;

    /// 中文模式（`caps` / `english_mode` 都灭）。
    const CHINESE: (bool, bool) = (false, false);

    fn key(vk: u32, caps: bool, english_mode: bool) -> qingjian_platform::protocol::KeyEvent {
        let mut event = to_key_event(vk, english_mode);
        event.modifiers.caps = caps;
        event.modifiers.english_mode = english_mode;
        event
    }

    fn with_modifiers(vk: u32, character: char, modifiers: KeyModifiers) -> KeyEvent {
        let mut event = KeyEvent::new(vk, Some(character), modifiers);
        event.modifiers = modifiers;
        event
    }

    #[test]
    fn shifted_letters_without_composition_go_to_the_app() {
        // 中文模式、没在组句：Shift 敲的大写缺省**归应用**（原样打出来）
        let shifted = KeyModifiers {
            shift: true,
            ..KeyModifiers::default()
        };
        assert!(!eats_key(
            &with_modifiers(0x41, 'A', shifted),
            false,
            false,
            false
        ));
        // `[general] shift_letter = "compose"`：没在组句也吃，送去起一段组句（⇧C 接 pan 出 C盘）
        assert!(eats_key(
            &with_modifiers(0x41, 'A', shifted),
            false,
            false,
            true
        ));
        // 组句一开始，后面的 Shift 字母就被 `composing` 兜住，一律吃
        assert!(eats_key(
            &with_modifiers(0x41, 'A', shifted),
            true,
            false,
            false
        ));
        // 双拼下 Shift + V / U / I 是表达式 / 问字入口：没在组句也吃
        assert!(eats_key(
            &with_modifiers(0x56, 'V', shifted),
            false,
            false,
            false
        ));
        // 不带 Shift 的字母本来就吃
        assert!(eats_key(
            &with_modifiers(0x41, 'a', KeyModifiers::default()),
            false,
            false,
            false
        ));
        // Caps 亮着（直通大写）也吃，由我们插入
        let caps = KeyModifiers {
            caps: true,
            ..KeyModifiers::default()
        };
        assert!(eats_key(
            &with_modifiers(0x41, 'A', caps),
            false,
            false,
            false
        ));
        // 带 Ctrl 的组合键归应用，翻译评审中一律吃
        let ctrl_c = KeyModifiers {
            ctrl: true,
            ..KeyModifiers::default()
        };
        assert!(!eats_key(
            &with_modifiers(0x43, 'c', ctrl_c),
            false,
            false,
            false
        ));
        assert!(eats_key(
            &with_modifiers(0x43, 'c', ctrl_c),
            false,
            true,
            false
        ));
    }

    #[test]
    fn letters_are_eaten_only_in_chinese_mode() {
        let (caps, english) = CHINESE;
        assert!(eats_without_server(&key(0x41, caps, english))); // A
        assert!(!eats_without_server(&key(0x41, true, english))); // Caps 亮着是直通英文
        assert!(!eats_without_server(&key(0x41, caps, true))); // 英文模式
    }

    #[test]
    fn punctuation_digits_and_command_keys_go_to_the_app() {
        let (caps, english) = CHINESE;
        // `-`（0xBD）、数字 2、`@`：中文模式会话外都是原样交给应用的键
        assert!(!eats_without_server(&key(0xBD, caps, english)));
        assert!(!eats_without_server(&key(0x32, caps, english)));
        assert!(!eats_without_server(&key(0x32, true, english)));
        // Ctrl+C 这类组合键一律归应用
        let mut combo = key(0x43, caps, english);
        combo.modifiers = KeyModifiers {
            ctrl: true,
            ..combo.modifiers
        };
        assert!(!eats_without_server(&combo));
    }
}
