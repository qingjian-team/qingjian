//! 输入框的编辑状态：文字、光标、选区、输入法正在组的字。由使用方保存，输入框产出的 [`Edit`] 交给 [`TextEdit::apply`]。
//! 多行的（[`TextEdit::multiline`]）回车换行、上下键换行，行首行尾按当前行算。

use std::ops::Range;

use crate::ui::{KeyCode, KeyPress, TextInput};

/// 输入框里发生的一次编辑。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    Key(KeyPress),

    Text(TextInput),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextEdit {
    text: String,

    /// 光标（字节下标，落在字符边界上）。
    caret: usize,

    /// 选区的另一端；与光标相同就是没有选区。
    anchor: usize,

    /// 输入法正在组的字，画在光标处，上屏前不算进文字。
    preedit: String,

    multiline: bool,
}

impl TextEdit {
    /// 光标放在末尾。
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let end = text.len();
        Self {
            text,
            caret: end,
            anchor: end,
            preedit: String::new(),
            multiline: false,
        }
    }

    /// 多行：保留换行，回车插入换行。
    pub fn multiline(text: impl Into<String>) -> Self {
        Self {
            multiline: true,
            ..Self::new(text.into().replace("\r\n", "\n").replace('\r', "\n"))
        }
    }

    pub fn is_multiline(&self) -> bool {
        self.multiline
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// 选中的范围（字节，从小到大）；没有选区时是空范围。
    pub fn selection(&self) -> Range<usize> {
        self.caret.min(self.anchor)..self.caret.max(self.anchor)
    }

    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    /// 应用一次编辑；复制、剪切时返回要放上剪贴板的文字。
    pub fn apply(&mut self, edit: &Edit) -> Option<String> {
        match edit {
            Edit::Text(TextInput::Insert(text)) => {
                self.preedit.clear();
                // 单行：粘贴进来的换行、制表符都去掉（密钥后面常带着换行）；多行留着换行
                let text = text.replace("\r\n", "\n").replace('\r', "\n");
                let multiline = self.multiline;
                let clean: String = text
                    .chars()
                    .filter(|&c| !c.is_control() || (multiline && c == '\n'))
                    .collect();
                self.replace_selection(&clean);
                None
            }
            Edit::Text(TextInput::Preedit(text)) => {
                self.preedit = text.clone();
                None
            }
            Edit::Key(press) => self.key(press),
        }
    }

    fn key(&mut self, press: &KeyPress) -> Option<String> {
        let shift = press.modifiers.shift;
        let command = press.modifiers.command;
        match &press.key {
            KeyCode::Enter if self.multiline => {
                self.preedit.clear();
                self.replace_selection("\n");
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown if self.multiline => {
                let target = self.vertical(press.key == KeyCode::ArrowDown);
                self.move_to(target, shift);
            }
            KeyCode::ArrowLeft => {
                let target = if command {
                    self.line_start(self.caret)
                } else if !shift && !self.selection().is_empty() {
                    self.selection().start
                } else {
                    self.previous(self.caret)
                };
                self.move_to(target, shift);
            }
            KeyCode::ArrowRight => {
                let target = if command {
                    self.line_end(self.caret)
                } else if !shift && !self.selection().is_empty() {
                    self.selection().end
                } else {
                    self.next(self.caret)
                };
                self.move_to(target, shift);
            }
            KeyCode::Home => self.move_to(self.line_start(self.caret), shift),
            KeyCode::End => self.move_to(self.line_end(self.caret), shift),
            KeyCode::Backspace => {
                if self.selection().is_empty() {
                    self.anchor = if command {
                        self.line_start(self.caret)
                    } else {
                        self.previous(self.caret)
                    };
                }
                self.replace_selection("");
            }
            KeyCode::Delete => {
                if self.selection().is_empty() {
                    self.anchor = self.next(self.caret);
                }
                self.replace_selection("");
            }
            KeyCode::Character(text) if command => match text.as_str() {
                "a" => {
                    self.anchor = 0;
                    self.caret = self.text.len();
                }
                "c" => return self.selected(),
                "x" => {
                    let copied = self.selected();
                    self.replace_selection("");
                    return copied;
                }
                _ => {}
            },
            _ => {}
        }
        None
    }

    fn selected(&self) -> Option<String> {
        let range = self.selection();
        (!range.is_empty()).then(|| self.text[range].to_owned())
    }

    fn move_to(&mut self, target: usize, extend: bool) {
        self.caret = target;
        if !extend {
            self.anchor = target;
        }
    }

    fn replace_selection(&mut self, text: &str) {
        let range = self.selection();
        self.text.replace_range(range.clone(), text);
        self.caret = range.start + text.len();
        self.anchor = self.caret;
    }

    /// `at` 所在行的行首（单行就是 0）。
    fn line_start(&self, at: usize) -> usize {
        self.text[..at].rfind('\n').map_or(0, |index| index + 1)
    }

    /// `at` 所在行的行尾（换行符前；单行就是末尾）。
    fn line_end(&self, at: usize) -> usize {
        self.text[at..]
            .find('\n')
            .map_or(self.text.len(), |index| at + index)
    }

    /// 上一行或下一行里同一列（按字数算）的位置；行短就到行尾，没有那一行就到头或到尾。
    fn vertical(&self, down: bool) -> usize {
        let start = self.line_start(self.caret);
        let column = self.text[start..self.caret].chars().count();
        let target_start = if down {
            let end = self.line_end(self.caret);
            if end == self.text.len() {
                return end;
            }
            end + 1
        } else {
            if start == 0 {
                return 0;
            }
            self.line_start(start - 1)
        };
        let target_end = self.line_end(target_start);
        self.text[target_start..target_end]
            .char_indices()
            .nth(column)
            .map_or(target_end, |(index, _)| target_start + index)
    }

    fn previous(&self, at: usize) -> usize {
        self.text[..at]
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index)
    }

    fn next(&self, at: usize) -> usize {
        self.text[at..]
            .chars()
            .next()
            .map_or(at, |c| at + c.len_utf8())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Modifiers;

    fn key(edit: &mut TextEdit, key: KeyCode, modifiers: Modifiers) -> Option<String> {
        edit.apply(&Edit::Key(KeyPress::new(key, modifiers)))
    }

    fn insert(edit: &mut TextEdit, text: &str) {
        edit.apply(&Edit::Text(TextInput::Insert(text.to_owned())));
    }

    #[test]
    fn typing_moving_and_deleting_respect_character_boundaries() {
        let mut edit = TextEdit::new("青简");
        insert(&mut edit, "ab");
        assert_eq!(edit.text(), "青简ab");
        key(&mut edit, KeyCode::ArrowLeft, Modifiers::default());
        key(&mut edit, KeyCode::ArrowLeft, Modifiers::default());
        key(&mut edit, KeyCode::Backspace, Modifiers::default());
        assert_eq!(edit.text(), "青ab");
        key(&mut edit, KeyCode::Delete, Modifiers::default());
        assert_eq!(edit.text(), "青b");
    }

    #[test]
    fn selection_is_replaced_copied_and_cut() {
        let command = Modifiers {
            command: true,
            ..Modifiers::default()
        };
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        let mut edit = TextEdit::new("hello world");
        key(&mut edit, KeyCode::ArrowLeft, shift);
        key(&mut edit, KeyCode::ArrowLeft, shift);
        assert_eq!(
            key(&mut edit, KeyCode::Character("c".to_owned()), command),
            Some("ld".to_owned())
        );
        insert(&mut edit, "LD\n");
        assert_eq!(edit.text(), "hello worLD");
        key(&mut edit, KeyCode::Character("a".to_owned()), command);
        assert_eq!(
            key(&mut edit, KeyCode::Character("x".to_owned()), command),
            Some("hello worLD".to_owned())
        );
        assert_eq!(edit.text(), "");
    }

    #[test]
    fn multiline_keeps_newlines_and_moves_between_lines() {
        let mut edit = TextEdit::multiline("第一行\r\nab");
        assert_eq!(edit.text(), "第一行\nab");
        key(&mut edit, KeyCode::ArrowUp, Modifiers::default());
        // 第二行第 2 列 → 第一行第 2 列
        insert(&mut edit, "|");
        assert_eq!(edit.text(), "第一|行\nab");
        key(&mut edit, KeyCode::End, Modifiers::default());
        key(&mut edit, KeyCode::Enter, Modifiers::default());
        insert(&mut edit, "x\ny");
        assert_eq!(edit.text(), "第一|行\nx\ny\nab");
        key(&mut edit, KeyCode::ArrowDown, Modifiers::default());
        key(&mut edit, KeyCode::ArrowDown, Modifiers::default());
        assert_eq!(edit.caret(), edit.text().len());

        let mut single = TextEdit::new("");
        insert(&mut single, "a\nb");
        assert_eq!(single.text(), "ab");
    }

    #[test]
    fn preedit_is_shown_until_committed() {
        let mut edit = TextEdit::new("");
        edit.apply(&Edit::Text(TextInput::Preedit("ni".to_owned())));
        assert_eq!((edit.text(), edit.preedit()), ("", "ni"));
        insert(&mut edit, "你");
        assert_eq!((edit.text(), edit.preedit()), ("你", ""));
    }
}
