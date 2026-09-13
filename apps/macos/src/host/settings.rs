//! 菜单与偏好设置窗口的动作：只改 config.toml（或触发一次性操作），改完由 apply_config 统一生效。

use super::diagnostics::{copy_to_pasteboard, open_with_system};
use super::*;

impl Host {
    /// 表格中的启用开关只修改所选规则。
    pub fn set_phrase_enabled(&mut self, index: usize, enabled: bool) {
        let mut phrases = self.settings.config().custom_phrases.clone();
        let Some(phrase) = phrases.get_mut(index) else {
            return;
        };
        phrase.enabled = enabled;
        let Some(path) = self.settings.path() else {
            return;
        };
        let result = qingjian_platform::Config::set_custom_phrases(path, &phrases);
        self.settings.reload();
        self.apply_config(false);
        if let Err(error) = result {
            self.preferences.set_status(&error);
        }
    }

    /// 菜单动作。开关类先落盘再热加载，菜单勾选状态永远来自文件里的值。
    pub fn perform(&mut self, action: MenuAction) {
        tracing::info!(?action, "菜单");
        match action {
            MenuAction::ToggleCloud => {
                let on = !self.settings.config().predict.enabled;
                if self.settings.set_bool("predict", "enabled", on) {
                    self.apply_config(false);
                }
            }
            MenuAction::ToggleFuzzy(index) => {
                let name = FuzzyRules::NAMES[index];
                let on = !self.settings.config().fuzzy.is_on(name);
                if self.settings.set_bool("fuzzy", name, on) {
                    self.apply_config(false);
                }
            }
            MenuAction::OpenPreferences => {
                self.preferences.sync_usage(
                    &self.engine.usage_summary(),
                    &self.engine.vocabulary_summary(),
                    self.engine.learning_language(),
                );
                self.preferences.show();
            }
            MenuAction::OpenLogs => {
                if let Some(dir) = logging::log_dir() {
                    open_with_system(&[&dir.to_string_lossy()]);
                }
            }
        }
    }

    /// 设置窗口里改了一个控件：写配置、热加载；写不成（非法组合、空文本）也要把控件同步回真实值。
    pub fn change_setting(&mut self, setting: Setting, value: SettingValue) {
        // 密钥值不进日志
        tracing::info!(?setting, "设置");
        let config = self.settings.config().clone();
        match (setting, value) {
            (Setting::NewPhrase, _) => {
                self.preferences.edit_phrase(&config, None);
                return;
            }
            (Setting::EditPhrase, _) => {
                if let Some(index) = self.preferences.selected_phrase() {
                    self.preferences.edit_phrase(&config, Some(index));
                }
                return;
            }
            (Setting::CancelPhraseEdit, _) => {
                self.preferences.close_phrase_editor();
                return;
            }

            (Setting::PhraseDraft, _) => return,
            (Setting::SelectPhrase, SettingValue::Index(index)) => {
                self.preferences.select_phrase(&config, index);
                return;
            }
            (Setting::SavePhrase | Setting::DeletePhrase, _) => {
                let (index, draft) = self.preferences.phrase_draft();
                let mut phrases = config.custom_phrases.clone();
                if setting == Setting::DeletePhrase {
                    let Some(index) = self.preferences.selected_phrase() else {
                        return;
                    };
                    phrases.remove(index);
                } else if let Some(index) = index {
                    phrases[index] = draft;
                } else {
                    phrases.push(draft);
                }
                let Some(path) = self.settings.path() else {
                    return;
                };
                if let Err(error) = qingjian_platform::Config::set_custom_phrases(path, &phrases) {
                    self.preferences.set_phrase_error(&error);
                    self.preferences.set_status(&error);
                    return;
                }
                self.preferences.close_phrase_editor();
                self.settings.reload();
                self.apply_config(false);
                if setting == Setting::SavePhrase {
                    self.preferences.select_phrase(
                        self.settings.config(),
                        index.map_or(phrases.len(), |i| i + 1),
                    );
                }
                self.preferences.set_status("快捷文本已保存");
                return;
            }
            (Setting::FullWidthPunctuation, SettingValue::Index(index)) => {
                self.settings
                    .set_bool("general", "full_width_punctuation", index == 0);
            }
            (Setting::LearningLanguage, SettingValue::Index(index)) => {
                if let Some(language) = self.languages.get(index) {
                    self.settings
                        .set_value("general", "learning_language", language.code());
                }
            }
            (Setting::PageSize, SettingValue::Index(index)) => {
                self.settings
                    .set_value("general", "page_size", index as i64 + 1);
            }
            (Setting::PageKeys, SettingValue::Index(index)) => {
                if let Some(pair) = PAGE_KEY_OPTIONS.get(index) {
                    self.settings.set_value("general", "page_keys", *pair);
                }
            }
            (Setting::Theme, SettingValue::Index(index)) => {
                if let Some(theme) = ThemeMode::ALL.get(index) {
                    self.settings.set_value("general", "theme", theme.key());
                }
            }
            (Setting::Layout, SettingValue::Index(index)) => {
                if let Some(layout) = LayoutMode::ALL.get(index) {
                    self.settings.set_value("general", "layout", layout.key());
                }
            }
            (Setting::Preedit, SettingValue::Index(index)) => {
                if let Some(mode) = PreeditMode::ALL.get(index) {
                    self.settings.set_value("general", "preedit", mode.key());
                }
            }
            (Setting::ExpressionKey | Setting::QuestionKey, SettingValue::Index(index)) => {
                if let Some(&key) = ModeKeys::CANDIDATES.get(index) {
                    let mut keys = config.shortcut.mode.sanitized();
                    let name = if setting == Setting::ExpressionKey {
                        keys.expression = key;
                        "expression"
                    } else {
                        keys.question = key;
                        "question"
                    };
                    if keys.is_valid() {
                        self.settings.set_value("shortcut", name, key.to_string());
                    } else {
                        tracing::warn!("表达式键与问字键不能相同，未改");
                    }
                }
            }
            (
                Setting::TranslationKeys | Setting::TranslationSecondKeys,
                SettingValue::Text(text),
            ) => match text.parse::<Modifiers>() {
                Ok(chosen) => {
                    let (first, second) = config.shortcut.translation_keys();
                    let (name, other) = if setting == Setting::TranslationKeys {
                        ("translation", second)
                    } else {
                        ("translation_second", first)
                    };
                    if chosen == other {
                        tracing::warn!("两组译词快捷键不能相同，未改");
                    } else {
                        self.settings.set_value("shortcut", name, chosen.key());
                    }
                }
                Err(error) => tracing::warn!(%error, "修饰键组合不合法，未改"),
            },
            (Setting::DeleteCandidateKeys, SettingValue::Text(text)) => {
                match text.parse::<Modifiers>() {
                    Ok(chosen) => {
                        let (first, second) = config.shortcut.translation_keys();
                        if chosen == first || chosen == second {
                            tracing::warn!("删候选的快捷键不能与译词快捷键相同，未改");
                        } else {
                            self.settings
                                .set_value("shortcut", "delete_candidate", chosen.key());
                        }
                    }
                    Err(error) => tracing::warn!(%error, "修饰键组合不合法，未改"),
                }
            }
            (Setting::TranslateSelectionKeys, SettingValue::Text(text)) => {
                match text.parse::<KeyCombo>() {
                    Ok(combo) => {
                        self.settings.set_value(
                            "shortcut",
                            "translate_selection",
                            combo.key_string(),
                        );
                    }
                    Err(error) => tracing::warn!(%error, "快捷键不合法，未改"),
                }
            }
            (Setting::ResetShortcuts, _) => {
                let defaults = ShortcutConfig::default();
                self.settings
                    .set_value("general", "page_keys", PAGE_KEY_OPTIONS[0]);
                self.settings.set_value(
                    "shortcut",
                    "expression",
                    defaults.mode.expression.to_string(),
                );
                self.settings
                    .set_value("shortcut", "question", defaults.mode.question.to_string());
                self.settings
                    .set_value("shortcut", "translation", defaults.translation.key());
                self.settings.set_value(
                    "shortcut",
                    "translation_second",
                    defaults.translation_second.key(),
                );
                self.settings.set_value(
                    "shortcut",
                    "translate_selection",
                    defaults.translate_selection.key_string(),
                );
                self.settings.set_value(
                    "shortcut",
                    "delete_candidate",
                    defaults.delete_candidate.key(),
                );
            }
            (Setting::DictionaryEnabled(index), SettingValue::Bool(on)) => {
                if let Some(info) = self.dictionary_list.get(index).cloned() {
                    if info.builtin {
                        self.set_domain_enabled(&info.stem, on);
                    } else {
                        self.set_dictionary_enabled(&info.stem, on);
                    }
                }
            }
            (Setting::DictionaryRemove(index), _) => {
                self.remove_dictionary(index);
                return;
            }
            (Setting::ImportDictionary, _) => {
                if let Some(path) = crate::preferences::choose_dictionary_file() {
                    self.import_dictionary(&path);
                }
                return;
            }
            (Setting::Fuzzy(index), SettingValue::Bool(on)) => {
                self.settings
                    .set_bool("fuzzy", FuzzyRules::NAMES[index], on);
            }
            (Setting::CloudEnabled, SettingValue::Bool(on)) => {
                self.settings.set_bool("predict", "enabled", on);
            }
            (Setting::LocalModelEnabled, SettingValue::Bool(on)) => {
                self.settings.set_bool("model", "enabled", on);
            }
            (Setting::CloudSlots, SettingValue::Index(index)) => {
                self.settings.set_value("predict", "slots", index as i64);
            }
            (Setting::EnglishCandidates, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "english_candidates", on);
            }
            // 勾上写缺省的终端 / 编辑器列表，去掉写空表；手改过的列表勾一下就回缺省
            (Setting::EnglishCandidatesOffInApps, SettingValue::Bool(on)) => {
                let apps: toml_edit::Array = if on {
                    DEFAULT_ENGLISH_CANDIDATES_OFF.iter().copied().collect()
                } else {
                    toml_edit::Array::new()
                };
                self.settings
                    .set_value("apps", "english_candidates_off", apps);
            }
            // 弹出菜单第 0 项是「关」，之后按 ShuangpinScheme::ALL 的顺序
            (Setting::Shuangpin, SettingValue::Index(index)) => {
                let key = index
                    .checked_sub(1)
                    .and_then(|i| ShuangpinScheme::ALL.get(i))
                    .map_or("", |scheme| scheme.key());
                self.settings.set_value("general", "shuangpin", key);
            }
            // 文本框失焦也会发 action：值没变就不写，免得每次切窗口都重写一遍配置
            (Setting::BaseUrl, SettingValue::Text(text)) => {
                let text = text.trim();
                if !text.is_empty() && text != config.predict.base_url {
                    self.settings.set_value("predict", "base_url", text);
                }
            }
            (Setting::Model, SettingValue::Text(text)) => {
                let text = text.trim();
                if !text.is_empty() && text != config.predict.model {
                    self.settings.set_value("predict", "model", text);
                }
            }
            (Setting::ApiKey, SettingValue::Text(text)) => {
                let text = text.trim();
                if !text.is_empty() && self.settings.set_env_var(&config.predict.api_key_env, text)
                {
                    // 密钥换了必须重建 Predictor
                    self.apply_config(true);
                    return;
                }
            }
            (Setting::TestCloud, _) => {
                self.start_cloud_test();
                return;
            }
            (Setting::OpenConfigFile, _) => {
                if let Some(path) = self.settings.path() {
                    open_with_system(&["-t", &path.to_string_lossy()]);
                }
                return;
            }
            (Setting::InputLog, SettingValue::Bool(on)) => {
                self.settings.set_bool("general", "input_log", on);
            }
            (Setting::ClearInputLog, _) => {
                self.clear_input_log();
                return;
            }
            (Setting::VerboseLog, SettingValue::Bool(on)) => {
                let level = if on { LogLevel::Debug } else { LogLevel::Info };
                self.settings.set_value("general", "log_level", level.key());
            }
            (Setting::OpenWebsite, _) => {
                open_with_system(&[crate::preferences::WEBSITE_URL]);
                return;
            }
            (Setting::OpenRepository, _) => {
                open_with_system(&[crate::preferences::REPOSITORY_URL]);
                return;
            }
            (Setting::OpenLogDirectory, _) => {
                if let Some(dir) = logging::log_dir() {
                    open_with_system(&[&dir.to_string_lossy()]);
                }
                return;
            }
            (Setting::CopyDiagnostics, _) => {
                copy_to_pasteboard(&self.diagnostics());
                self.preferences
                    .set_status("诊断信息已复制到剪贴板，粘贴给作者即可");
                return;
            }
            (setting, value) => tracing::warn!(?setting, ?value, "设置项与控件值不匹配"),
        }
        self.apply_config(false);
    }
}
