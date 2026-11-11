//! 只应用后台准备好的资源与内存配置，不在输入线程扫描或加载文件。

use std::time::Instant;

use qingjian_platform::Config;

use super::prepared::PreparedReload;
use crate::dispatch::{Router, RouterConfig};

impl Router {
    pub(super) fn apply_reload(&mut self, mut prepared: PreparedReload) {
        if let Some(config) = &prepared.config {
            self.apply_config(config);
            tracing::info!("configuration reloaded");
        }
        if let Some(schemes) = prepared.schemes {
            self.apply_prepared_schemes(schemes);
        }
        if let Some(dictionaries) = prepared.dictionaries {
            self.engine.set_extra_dictionaries(dictionaries);
            self.engine
                .set_blocked_words(prepared.state.applied_dictionaries.blocked.clone());
            self.engine
                .set_base_words(prepared.state.applied_dictionaries.base);
        }
        if let Some(english) = prepared.english {
            self.engine.set_english_translator(english);
        }
        if let Some(translator) = prepared.translator {
            self.engine.set_translator(translator);
        }
        if let Some(model) = prepared.model {
            self.apply_model_config(&model);
        }
        if let Some(cloud) = prepared.cloud {
            self.reload_cloud(cloud);
        }
        if !self.engine.prediction_enabled() {
            self.config.cloud_slots = 0;
        }
        prepared.state.last_check = Instant::now();
        self.reload = Some(prepared.state);
    }

    fn apply_config(&mut self, config: &Config) {
        self.engine.set_fuzzy(config.fuzzy);
        self.engine
            .set_shuangpin_raw_preedit(config.general.shuangpin_raw_preedit);
        self.engine
            .set_shuangpin_full_pinyin(config.general.shuangpin_full_pinyin);
        self.engine
            .set_hold_selections(config.general.hold_selections);
        self.engine
            .set_context_memory(config.model.context_memory_seconds);
        self.engine
            .set_shift_letter_compose(config.general.shift_letter.compose());
        self.engine.set_learning(config.general.learning);
        self.engine
            .set_auto_phrase(config.auto_phrase.enabled, config.auto_phrase.max_chars);
        self.engine.set_chinese_first(config.general.chinese_first);
        self.engine
            .set_spelling_correction(config.general.spelling_correction);
        self.engine
            .set_english_in_chinese(config.general.english_in_chinese);
        self.engine
            .set_english_max_in_chinese(config.general.english_max_in_chinese());
        self.engine
            .set_english_min_position(config.general.english_min_position());
        self.engine
            .set_emoji_limit(usize::from(config.general.emoji.get()));
        self.engine
            .set_kaomoji_limit(usize::from(config.general.kaomoji.get()));
        self.engine
            .set_symbol_limit(usize::from(config.general.symbols.get()));
        self.engine.set_traditional(config.general.traditional());
        self.engine
            .set_half_width_symbols(&config.general.half_width_symbols);
        self.engine.set_mode_keys(config.shortcut.mode);
        if let Err(error) = self
            .engine
            .set_custom_phrases(config.custom_phrases.clone())
        {
            tracing::warn!(%error, "invalid custom phrases, keeping previous phrases");
        }
        self.config = RouterConfig::from(config);
        self.config.page_size = self.config.page_size.clamp(1, 9);
        if !self.config.english_mode {
            for info in self.sessions.values_mut() {
                info.english = false;
            }
        }
    }
}
