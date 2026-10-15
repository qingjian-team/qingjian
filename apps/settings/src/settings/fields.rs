//! 输入框：编辑状态与配置之间的换入、存回。

use bamboo_slip::TextEdit;

use super::Settings;
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
            FieldId::PhraseCode | FieldId::PhraseText => return,
        };
        self.fields.insert(id, TextEdit::new(value));
    }

    /// 存一个输入框：没改、清空了都不存（清空等于不改）。
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
            FieldId::PhraseCode | FieldId::PhraseText => return,
            FieldId::ApiKey if text.is_empty() => {}
            // 密码框看不见内容，粘贴多了（带上了终端提示符、命令）用户发现不了
            FieldId::ApiKey if text.chars().any(|c| !c.is_ascii_graphic()) => {
                self.notice = Some((
                    "cloud",
                    "密钥没有保存：里面有空格或非英文字符，多半是粘贴时多带了别的内容".to_owned(),
                ));
            }
            FieldId::ApiKey => {
                qingjian_platform::logs::secrets::register(&text);
                self.write("predict", id.key(), text);
                self.notice = Some(("cloud", "密钥已保存".to_owned()));
            }
        }
        self.reset_field(id);
    }
}
