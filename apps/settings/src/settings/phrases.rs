//! 自定义短语的编辑卡片：打开、关闭、存回配置。

use bamboo_slip::TextEdit;
use qingjian_core::CustomPhrase;
use qingjian_platform::Config;

use super::Settings;
use crate::message::FieldId;
use crate::phrase_editor::PhraseEditor;

impl Settings {
    pub(super) fn open_phrase(&mut self, index: Option<usize>) {
        let (editor, code, text) =
            match index.and_then(|i| Some((i, self.config.custom_phrases.get(i)?))) {
                Some((i, phrase)) => (
                    PhraseEditor::edit(i, phrase),
                    phrase.code.clone(),
                    phrase.text.clone(),
                ),
                None => (PhraseEditor::new(), String::new(), String::new()),
            };
        self.fields.insert(FieldId::PhraseCode, TextEdit::new(code));
        self.fields
            .insert(FieldId::PhraseText, TextEdit::multiline(text));
        self.phrase = Some(editor);
    }

    pub(super) fn close_phrase(&mut self) {
        self.phrase = None;
        self.fields.remove(&FieldId::PhraseCode);
        self.fields.remove(&FieldId::PhraseText);
    }

    /// 存卡片（`delete` 时删掉这条）：整张表校验写回，失败把原因留在卡片里。
    pub(super) fn save_phrase(&mut self, delete: bool) {
        let Some(editor) = self.phrase.clone() else {
            return;
        };
        let field = |id| {
            self.fields
                .get(&id)
                .map(|edit: &TextEdit| edit.text().to_owned())
                .unwrap_or_default()
        };
        let phrase = (!delete).then(|| CustomPhrase {
            code: field(FieldId::PhraseCode).trim().to_owned(),
            text: field(FieldId::PhraseText),
            position: editor.position,
            enabled: editor.enabled,
        });
        let saved = editor
            .apply(&self.config.custom_phrases, phrase)
            .and_then(|list| self.write_phrases(&list));
        match saved {
            Ok(()) => self.close_phrase(),
            Err(error) => {
                if let Some(editor) = self.phrase.as_mut() {
                    editor.error = Some(error);
                    editor.confirm_delete = false;
                }
            }
        }
    }

    pub(super) fn write_phrases(&mut self, list: &[CustomPhrase]) -> Result<(), String> {
        Config::set_custom_phrases(&self.path, list)?;
        self.reload();
        Ok(())
    }
}
