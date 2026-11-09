//! 输入框：编辑状态与配置之间的换入、存回。

use bamboo_slip::TextEdit;

use super::Settings;
use crate::i18n::tr;
use crate::message::FieldId;

impl Settings {
    pub(super) fn with_fields(mut self) -> Self {
        for id in FieldId::ALL {
            self.reset_field(id);
        }
        self
    }

    /// 输入框换回配置里的值（密钥框总是空的，不显示已存的密钥）。
    pub(super) fn reset_field(&mut self, id: FieldId) {
        let predict = &self.config.predict;
        let value = match id {
            FieldId::BaseUrl => predict.base_url.clone(),
            FieldId::Model => predict.model.clone(),
            FieldId::ApiKey => String::new(),
            FieldId::HalfWidthSymbols => self.config.general.half_width_symbols.clone(),
            FieldId::VocabularySearch => String::new(),
            FieldId::PhraseCode | FieldId::PhraseWeight | FieldId::PhraseText => return,
        };
        self.fields.insert(id, TextEdit::new(value));
    }

    /// 存一个输入框：没改的不存；清空等于不改，保持半角的符号除外（清空就是一个都不保持）。
    pub(super) fn commit(&mut self, id: FieldId) {
        let Some(text) = self
            .fields
            .get(&id)
            .map(|edit| edit.text().trim().to_owned())
        else {
            return;
        };
        match id {
            FieldId::BaseUrl | FieldId::Model => {
                let current = match id {
                    FieldId::BaseUrl => &self.config.predict.base_url,
                    _ => &self.config.predict.model,
                };
                if !text.is_empty() && text != *current {
                    self.write("predict", id.key(), text);
                }
            }
            // 只留会转全角的标点，存下来的就是生效的
            FieldId::HalfWidthSymbols => {
                let mut symbols = String::new();
                for c in text.chars() {
                    if qingjian_core::punctuation::converts(c) && !symbols.contains(c) {
                        symbols.push(c);
                    }
                }
                if symbols != self.config.general.half_width_symbols {
                    self.write("general", id.key(), symbols);
                }
            }
            FieldId::PhraseCode
            | FieldId::PhraseWeight
            | FieldId::PhraseText
            | FieldId::VocabularySearch => return,
            FieldId::ApiKey if text.is_empty() => {}
            // 密码框看不见内容，粘贴多了（带上了终端提示符、命令）用户发现不了
            FieldId::ApiKey if text.chars().any(|c| !c.is_ascii_graphic()) => {
                self.notice = Some(("cloud", tr("cloud.bad_key").to_owned()));
            }
            FieldId::ApiKey => {
                qingjian_platform::logs::secrets::register(&text);
                self.write("predict", id.key(), text);
                self.notice = Some(("cloud", tr("cloud.key_saved").to_owned()));
            }
        }
        self.reset_field(id);
    }
}
