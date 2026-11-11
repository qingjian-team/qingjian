//! 输入框：编辑状态与配置之间的换入、存回。

use bamboo_slip::TextEdit;
use qingjian_core::CustomRule;

use super::Settings;
use crate::i18n::{tr, tr_args};
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
            FieldId::FuzzyCustom => self.config.fuzzy.custom.join(" "),
            FieldId::VocabularySearch => String::new(),
            FieldId::PhraseCode | FieldId::PhraseWeight | FieldId::PhraseText => return,
        };
        self.fields.insert(id, TextEdit::new(value));
    }

    /// 存一个输入框：没改的不存；清空等于不改，保持半角的符号与自定义模糊音除外（清空就是一个都不要）。
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
            FieldId::FuzzyCustom => self.commit_fuzzy_custom(&text),
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

    /// 存自定义模糊音：认得出的按规范写法存、去重；删掉的规则从关着的名单里也去掉；认不出的提示一句，不存。
    fn commit_fuzzy_custom(&mut self, text: &str) {
        let (rules, bad) = parse_custom_rules(text);
        let fuzzy = &self.config.fuzzy;
        if rules != fuzzy.custom {
            let off: Vec<String> = fuzzy
                .custom_off
                .iter()
                .filter(|rule| rules.contains(rule))
                .cloned()
                .collect();
            let off_changed = off != fuzzy.custom_off;
            self.write("fuzzy", "custom", toml_edit::Array::from_iter(&rules));
            if off_changed {
                self.write("fuzzy", "custom_off", toml_edit::Array::from_iter(&off));
            }
        }
        if !bad.is_empty() {
            let rules = bad.join(" ");
            self.notice = Some((
                "fuzzy-custom",
                tr_args("input.fuzzy_custom_bad", &[("rules", &rules)]),
            ));
        }
    }
}

/// 输入框里的几条规则：认得出的（规范写法、去重）与认不出的原文。等号两边的空格、全角等号都认，
/// 规则之间空格、逗号、顿号、分号隔开都行。
fn parse_custom_rules(text: &str) -> (Vec<String>, Vec<String>) {
    let mut joined = text.replace('＝', "=");
    while joined.contains(" =") || joined.contains("= ") {
        joined = joined.replace(" =", "=").replace("= ", "=");
    }
    let mut rules = Vec::new();
    let mut bad = Vec::new();
    for item in joined
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '，' | '、' | ';' | '；'))
        .filter(|item| !item.is_empty())
    {
        match CustomRule::parse(item) {
            Some(rule) if !rules.contains(&rule.text()) => rules.push(rule.text()),
            Some(_) => {}
            None => bad.push(item.to_owned()),
        }
    }
    (rules, bad)
}

#[cfg(test)]
mod tests {
    use super::parse_custom_rules;

    #[test]
    fn custom_rules_are_normalized_and_bad_ones_reported() {
        let (rules, bad) = parse_custom_rules("ian = iang，FA=hua  ian＝iang zh=ang");
        assert_eq!(rules, ["ian=iang", "fa=hua"]);
        assert_eq!(bad, ["zh=ang"]);
    }
}
