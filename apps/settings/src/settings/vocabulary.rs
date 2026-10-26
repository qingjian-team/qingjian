//! 生词本页的消息：切视图、语言、日子，筛选、排序、翻页，标认识、删除、清空，导出。

use bamboo_slip::TextEdit;

use super::Settings;
use crate::message::{FieldId, Message};
use crate::vocabulary::Vocabulary;

impl Settings {
    /// 进页时重读（输入法在此期间记了新词），搜索框清空。
    pub(super) fn open_vocabulary(&mut self) {
        self.vocabulary = Vocabulary::load(&self.config.general);
        self.fields
            .insert(FieldId::VocabularySearch, TextEdit::new(String::new()));
    }

    pub(super) fn vocabulary_message(&mut self, message: Message) {
        let query = self
            .fields
            .get(&FieldId::VocabularySearch)
            .map(|edit| edit.text().to_owned())
            .unwrap_or_default();
        let Some(vocabulary) = self.vocabulary.as_mut() else {
            return;
        };
        let confirm_clear = std::mem::take(&mut vocabulary.confirm_clear);
        let result = match message {
            Message::VocabularyView(view) => {
                vocabulary.view = view;
                vocabulary.page = 0;
                Ok(())
            }
            Message::VocabularyLanguage(language) => {
                vocabulary.set_language(language);
                Ok(())
            }
            Message::VocabularyDay(day) => {
                vocabulary.day = day;
                vocabulary.page = 0;
                Ok(())
            }
            Message::VocabularyFilter(filter) => {
                vocabulary.filter = filter;
                vocabulary.page = 0;
                Ok(())
            }
            Message::VocabularySort(sort) => {
                vocabulary.sort = sort;
                vocabulary.page = 0;
                Ok(())
            }
            Message::VocabularyPage(page) => {
                vocabulary.page = page;
                Ok(())
            }
            Message::VocabularyKnown(word, known) => vocabulary.set_known(&word, known),
            Message::VocabularyRemove(word) => vocabulary.remove(&word),
            Message::VocabularyClear if confirm_clear => vocabulary.clear().map(|()| {
                self.notice = Some(("vocabulary", "生词本已清空".to_owned()));
            }),
            Message::VocabularyClear => {
                vocabulary.confirm_clear = true;
                Ok(())
            }
            Message::VocabularyExport(format) => match vocabulary.export(format, &query) {
                Ok(Some(path)) => {
                    self.notice =
                        Some(("vocabulary-export", format!("已导出到 {}", path.display())));
                    Ok(())
                }
                Ok(None) => Ok(()),
                Err(error) => {
                    self.notice = Some(("vocabulary-export", error));
                    Ok(())
                }
            },
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.notice = Some(("vocabulary", error));
        }
    }
}
