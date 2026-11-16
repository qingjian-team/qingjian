//! 文件扫描、配置解析、词库和方案读取都在后台完成。

use qingjian_core::{Language, NoTranslator, Translator};
use qingjian_dictionary::Dictionary;
use qingjian_platform::{Config, LocalModelConfig, PreparedScheme, SchemeSlot};
use qingjian_predict::API_USAGE_FILE;

use super::{ConfigReload, mtime};
use crate::assembly::{self, CloudServices};

pub(super) struct PreparedReload {
    /// 与本批资源对应的配置和文件指纹。
    pub(super) state: ConfigReload,
    /// 成功解析的新配置；解析失败时保留正在使用的配置。
    pub(super) config: Option<Config>,
    /// 已经读好的双拼布局和形码码表。
    pub(super) schemes: Option<PreparedScheme>,
    /// 已读好的附加词库；空数组表示清除。
    pub(super) dictionaries: Option<Vec<Dictionary>>,
    /// 下载的英文释义表变化后的替换器。
    pub(super) english: Option<Box<dyn Translator>>,
    /// 学习语言或释义表变化后的替换器。
    pub(super) translator: Option<Box<dyn Translator>>,
    /// 需要启动或卸载的本地模型配置。
    pub(super) model: Option<LocalModelConfig>,
    /// 已初始化的云服务，包括从磁盘读取的用量记录。
    pub(super) cloud: Option<CloudServices>,
}

impl PreparedReload {
    pub(super) fn load(mut state: ConfigReload, slot: SchemeSlot, code_loaded: bool) -> Self {
        let current = mtime(&state.config_path);
        let config = if current != state.last_mtime {
            match Config::load(&state.config_path) {
                Ok(config) => Some(config),
                Err(error) => {
                    tracing::error!(%error, "failed to reload configuration, keeping previous configuration");
                    None
                }
            }
        } else {
            None
        };
        state.last_mtime = current;
        if let Some(config) = &config {
            state.applied_config = config.clone();
        }
        let files = state.dict_snapshot();
        let dictionaries = if files != state.dictionary_files
            || state.applied_config.dictionaries != state.applied_dictionaries
        {
            state.dictionary_files = files;
            state.applied_dictionaries = state.applied_config.dictionaries.clone();
            Some(state.load_dictionaries())
        } else {
            None
        };
        let files = state.glossary_snapshot();
        let glossaries_changed = files != state.glossary_files;
        state.glossary_files = files;
        let english = glossaries_changed.then(|| {
            assembly::glossary_file(
                &state.root,
                Some(&state.user.downloads()),
                Language::Chinese,
            )
            .and_then(|path| assembly::load_english_glossary(&path))
            .map_or_else(
                || Box::new(NoTranslator) as Box<dyn Translator>,
                |glossary| Box::new(glossary),
            )
        });
        let language = assembly::learning_language(&state.applied_config);
        let translator = if glossaries_changed || language != state.applied_language {
            Self::load_translator(language, &state)
        } else {
            None
        };
        if translator.is_some() {
            state.applied_language = language;
        }
        let files = state.scheme_snapshot();
        let schemes = if config.is_some() || files != state.scheme_files {
            state.scheme_files = files;
            Some(slot.prepare(
                &state.applied_config,
                Some(&state.user.custom()),
                Some(&state.root.join("data/generated/schemes")),
                code_loaded,
            ))
        } else {
            None
        };
        let model = (state.applied_config.model != state.applied_model)
            .then(|| state.applied_config.model.clone());
        if let Some(model) = &model {
            state.applied_model = model.clone();
        }
        let cloud = if state.applied_config.predict != state.applied_predict {
            state.applied_predict = state.applied_config.predict.clone();
            Some(CloudServices::load(
                &state.applied_predict,
                Some(state.user.records().join(API_USAGE_FILE)),
            ))
        } else {
            None
        };
        Self {
            state,
            config,
            schemes,
            dictionaries,
            english,
            translator,
            model,
            cloud,
        }
    }

    fn load_translator(
        language: Option<Language>,
        state: &ConfigReload,
    ) -> Option<Box<dyn Translator>> {
        let Some(language) = language else {
            return Some(Box::new(NoTranslator));
        };
        let Some(path) =
            assembly::glossary_file(&state.root, Some(&state.user.downloads()), language)
        else {
            tracing::warn!(
                language = language.code(),
                "glossary unavailable, keeping previous learning language"
            );
            return None;
        };
        match assembly::load_glossary(language, &path, Some(&state.user.learning())) {
            Ok(glossary) => Some(Box::new(glossary)),
            Err(error) => {
                tracing::warn!(%error, "failed to load glossary, keeping previous learning language");
                None
            }
        }
    }
}
