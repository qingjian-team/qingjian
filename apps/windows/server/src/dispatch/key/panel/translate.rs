//! 符号面板翻译组里的按键：字母照常打拼音（交回 [`Router::apply_key`] 的组句路径），空格 / 数字把候选选进框里（不进应用），
//! 英文模式下敲的字直接进框；`[` `]` 换目标语言；打拼音时 ←→ 挪高亮、↑↓ 与翻页键翻候选，
//! 没在打拼音时 ←→ / Home / End 挪光标（按住 Shift 扩选区）、Ctrl+A 全选、Delete 删选中的字或光标后的字；
//! 回车上屏译文（还没好就马上发、再按一次），Esc 放弃，退格先删拼音再删框里的字。与 macOS 壳的 `handle_translate_key` 对齐。

use qingjian_platform::protocol::KeyEvent;

use crate::dispatch::Router;
use crate::dispatch::key::{Effect, codes};

impl Router {
    pub(super) fn apply_translate_key(&mut self, event: &KeyEvent) -> Option<Effect> {
        let composing = !self.engine.composition().is_empty();
        let modifiers = event.modifiers;
        if modifiers.has_command_key() {
            let select_all = event.virtual_key == codes::KEY_A
                && modifiers.ctrl
                && !modifiers.alt
                && !modifiers.win
                && !composing;
            if select_all {
                self.engine.select_all_panel_text();
                return Some(Effect::Navigated);
            }
            return Some(Effect::Passthrough);
        }
        let extend = modifiers.shift;
        let page = self.config.page_size as isize;
        match event.virtual_key {
            codes::TAB => {
                let delta = if event.modifiers.shift { -1 } else { 1 };
                self.switch_panel(|panel| panel.switch_group(delta));
                return Some(Effect::Navigated);
            }
            codes::ESCAPE => return Some(self.discard_box()),
            codes::BACK => {
                if composing {
                    self.engine.backspace();
                } else if !self.engine.panel_backspace() {
                    self.close_panel();
                }
                return Some(Effect::Changed(None));
            }
            codes::RETURN => {
                if composing {
                    self.pick_highlighted_into_box();
                }
                return Some(match self.engine.take_panel_translation() {
                    Some(text) => {
                        tracing::debug!(
                            chars = text.chars().count(),
                            "panel translation committed"
                        );
                        self.panel = None;
                        Effect::Changed(Some(text))
                    }
                    // 拼音选进了框要重建组句；没打拼音时只重画，重建会把在飞的译文请求一起作废
                    None => {
                        self.engine.hurry_panel_translation();
                        if composing {
                            Effect::Changed(None)
                        } else {
                            Effect::Navigated
                        }
                    }
                });
            }
            codes::LEFT | codes::RIGHT => {
                let delta = if event.virtual_key == codes::LEFT {
                    -1
                } else {
                    1
                };
                if composing {
                    return Some(self.move_box_highlight(delta));
                }
                self.engine.move_panel_cursor(delta, extend);
                return Some(Effect::Navigated);
            }
            // Home / End 把光标挪到最前 / 最后，Delete 删选中的字或光标后一个字
            codes::HOME | codes::END if !composing => {
                let position = if event.virtual_key == codes::HOME {
                    0
                } else {
                    usize::MAX
                };
                self.engine.place_panel_cursor(position, extend);
                return Some(Effect::Navigated);
            }
            codes::DELETE if !composing => {
                self.engine.panel_delete();
                return Some(Effect::Changed(None));
            }
            codes::UP | codes::PRIOR => return Some(self.move_box_highlight(-page)),
            codes::DOWN | codes::NEXT => return Some(self.move_box_highlight(page)),
            _ => {}
        }
        let c = event.character.filter(|c| !c.is_control())?;
        if c == '[' || c == ']' {
            let step = if c == '[' { -1 } else { 1 };
            self.switch_panel(|panel| panel.switch_section(step));
            return Some(Effect::Navigated);
        }
        if event.modifiers.caps || event.modifiers.english_mode {
            if composing {
                self.pick_highlighted_into_box();
            }
            self.engine.push_panel_text(c.encode_utf8(&mut [0; 4]));
            return Some(Effect::Changed(None));
        }
        if composing {
            if c == ' ' {
                self.pick_highlighted_into_box();
                return Some(Effect::Changed(None));
            }
            if let Some(digit) = codes::digit(event) {
                let first = self.highlight / self.config.page_size * self.config.page_size;
                self.pick_into_box(first + digit - 1);
                return Some(Effect::Changed(None));
            }
            if let Some(step) = self.page_step(event) {
                return Some(self.move_box_highlight(step * page));
            }
        }
        if c.is_ascii_alphabetic() || (composing && c == '\'') {
            // 拼音照常组句；面板不收起
            return None;
        }
        // 标点、数字与空格：正在打的拼音先按高亮候选选进去，再把字加在后面
        if composing {
            self.pick_highlighted_into_box();
        }
        self.engine.push_panel_char(c);
        Some(Effect::Changed(None))
    }

    /// 选第 `index` 个候选进框（点框里的候选也走这里）。
    pub(in crate::dispatch) fn pick_into_box(&mut self, index: usize) -> bool {
        let Some(candidate) = self.layout_candidate(index) else {
            return false;
        };
        self.engine.commit_to_panel(&candidate)
    }

    fn pick_highlighted_into_box(&mut self) {
        self.pick_into_box(self.highlight);
    }

    /// 高亮挪 `delta` 个，夹在候选范围里。
    fn move_box_highlight(&mut self, delta: isize) -> Effect {
        let count = self.candidate_count();
        if count > 0 {
            self.highlight =
                (self.highlight as isize + delta).clamp(0, count as isize - 1) as usize;
            self.navigated = true;
        }
        Effect::Navigated
    }

    /// Esc：框里有字就连拼音一起扔掉；空框时同别的组一样回到候选。
    fn discard_box(&mut self) -> Effect {
        let empty = self
            .engine
            .symbol_panel()
            .is_none_or(|panel| panel.translate_source().is_empty());
        if empty {
            self.close_panel();
        } else {
            self.engine.clear();
            self.panel = None;
        }
        Effect::Changed(None)
    }
}
