//! 符号面板翻译组里的按键：字母照常打拼音，空格 / 数字 / 点候选把词选进框里（不进应用），英文模式下敲的字直接进框；
//! `[` `]` 换目标语言；打拼音时 ←→ 挪高亮、翻页键翻候选，没在打拼音时 ←→ / Home / End 挪光标（按住 Shift 扩选区）、⌘A 全选、Delete 删选中的字或光标后的字；回车上屏译文（还没好就马上发、再按一次），Esc 放弃，退格先删拼音再删框里的字。
//! 译文由 [`Host::poll_prediction`](crate::host::Host::poll_prediction) 收，[`crate::imk::panel_translation_changed`] 写回应用的 preedit。

use objc2_app_kit::NSEvent;
use qingjian_platform::Modifiers;

use super::PanelKey;
use crate::host;
use crate::imk::{QingjianInputController, TextClient};

impl QingjianInputController {
    pub(super) fn handle_translate_key(
        &self,
        event: &NSEvent,
        key: u16,
        pressed: Modifiers,
        client: TextClient<'_>,
    ) -> PanelKey {
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if pressed.command || pressed.control {
            // ⌘A 全选框里的字，别的组合键照常交给应用
            let letter_a = event
                .charactersIgnoringModifiers()
                .is_some_and(|c| c.to_string().eq_ignore_ascii_case("a"));
            let select_all = letter_a && pressed.command && !pressed.control && !pressed.option;
            if !select_all || composing {
                return Some(false);
            }
            if host::with(|h| h.engine.select_all_panel_text()).unwrap_or(false) {
                self.render(client);
            }
            return Some(true);
        }
        let extend = pressed.shift;
        match key {
            48 => {
                let delta = if pressed.shift { -1 } else { 1 };
                self.switch_panel(|panel| panel.switch_group(delta), client);
                return Some(true);
            }
            53 => {
                self.discard_box(client);
                return Some(true);
            }
            51 => {
                if composing {
                    host::with(|h| h.engine.backspace());
                    self.edited_box(client);
                } else if host::with(|h| h.engine.panel_backspace()).unwrap_or(false) {
                    self.edited_box(client);
                } else {
                    self.close_panel(client);
                }
                return Some(true);
            }
            36 | 76 => {
                if composing {
                    self.pick_highlighted_into_box(client);
                }
                self.commit_box(client);
                return Some(true);
            }
            123 | 124 => {
                let delta = if key == 123 { -1 } else { 1 };
                if composing {
                    self.navigate_candidate(delta, client);
                } else if host::with(|h| h.engine.move_panel_cursor(delta, extend)).unwrap_or(false)
                {
                    self.render(client);
                }
                return Some(true);
            }
            // Home / End 把光标挪到最前 / 最后，Delete 删选中的字或光标后一个字
            115 | 119 if !composing => {
                let position = if key == 115 { 0 } else { usize::MAX };
                if host::with(|h| h.engine.place_panel_cursor(position, extend)).unwrap_or(false) {
                    self.render(client);
                }
                return Some(true);
            }
            117 if !composing => {
                if host::with(|h| h.engine.panel_delete()).unwrap_or(false) {
                    self.edited_box(client);
                }
                return Some(true);
            }
            125 | 126 | 116 | 121 => {
                if composing {
                    let delta = if matches!(key, 126 | 116) { -1 } else { 1 };
                    self.turn_page(delta, client);
                }
                return Some(true);
            }
            _ => {}
        }
        let text = event
            .characters()
            .map(|t| t.to_string())
            .unwrap_or_default();
        let mut chars = text.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return Some(true);
        };
        if c.is_control() {
            return Some(true);
        }
        if c == '[' || c == ']' {
            let step = if c == '[' { -1 } else { 1 };
            self.switch_panel(|panel| panel.switch_section(step), client);
            return Some(true);
        }
        if host::with(|h| h.english_mode()).unwrap_or(false) {
            let c = if host::with(|h| h.english_letter_uppercase()).unwrap_or(false) {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            };
            host::with(|h| h.engine.push_panel_text(c.encode_utf8(&mut [0; 4])));
            self.edited_box(client);
            return Some(true);
        }
        if composing {
            if c == ' ' {
                self.pick_highlighted_into_box(client);
                return Some(true);
            }
            if let Some(digit) = c.to_digit(10).filter(|d| *d > 0) {
                self.pick_into_box(digit as usize - 1, client);
                return Some(true);
            }
            if let Some(step) = Self::page_step(c) {
                self.turn_page(step, client);
                return Some(true);
            }
        }
        if c.is_ascii_alphabetic() || (composing && c == '\'') {
            host::with(|h| h.engine.push(c.to_ascii_lowercase()));
            self.edited_box(client);
            return Some(true);
        }
        // 标点、数字与空格：正在打的拼音先按第一个候选选进去，再把字加在后面
        if composing {
            self.pick_highlighted_into_box(client);
        }
        host::with(|h| h.engine.push_panel_char(c));
        self.edited_box(client);
        Some(true)
    }

    /// 点或按数字选当前一页第 `offset` 个候选进框。
    pub(super) fn pick_into_box(&self, offset: usize, client: TextClient<'_>) {
        let picked = host::with(|h| {
            let index = h.session.index_on_page(offset)?;
            let candidate = h.session.candidate(index)?;
            Some(h.engine.commit_to_panel(&candidate))
        })
        .flatten()
        .unwrap_or(false);
        if picked {
            self.edited_box(client);
        }
    }

    fn pick_highlighted_into_box(&self, client: TextClient<'_>) {
        let picked = host::with(|h| {
            let candidate = h.session.candidate(h.session.highlighted)?;
            Some(h.engine.commit_to_panel(&candidate))
        })
        .flatten()
        .unwrap_or(false);
        if picked {
            self.edited_box(client);
        }
    }

    /// 框里或拼音变了：重查候选、重画，开始等停键。
    fn edited_box(&self, client: TextClient<'_>) {
        host::with(|h| h.watch_panel_translation());
        self.refresh(client);
    }

    /// 回车：译文好了就上屏并收起面板；还没好就不等停键马上发，等它回来再按一次。
    fn commit_box(&self, client: TextClient<'_>) {
        let text = host::with(|h| {
            let text = h.engine.take_panel_translation();
            if text.is_none() {
                h.engine.hurry_panel_translation();
                h.watch_panel_translation();
            }
            text
        })
        .flatten();
        match text {
            Some(text) => {
                tracing::debug!(chars = text.chars().count(), "panel translation committed");
                client.insert_text(&text);
                self.refresh(client);
            }
            None => self.render(client),
        }
    }

    /// Esc：框里有字就连拼音一起扔掉；空框时同别的组一样回到候选。
    fn discard_box(&self, client: TextClient<'_>) {
        let empty = host::with(|h| {
            h.engine
                .symbol_panel()
                .is_none_or(|panel| panel.translate_source().is_empty())
        })
        .unwrap_or(true);
        if empty {
            self.close_panel(client);
            return;
        }
        host::with(|h| h.engine.clear());
        client.set_marked_text("", 0);
        self.refresh(client);
    }
}
