//! 配置热加载：config.toml 改了就整份重新套用到 Engine 与窗口；激活期间的定时事务。

mod text_replacements;
mod watch;

pub(super) use text_replacements::TextReplacement;
pub(super) use watch::ConfigWatch;

use qingjian_platform::Config;
use qingjian_render::TextSizes;

use super::init::load_glossary;
use super::*;

impl Host {
    /// 把当前配置推给 Engine 与界面：模糊音 / 模式键 / 翻页 / 外观直接设；学习语言变了换释义表；
    /// `[predict]` 变了（或 `force`）才重建 Predictor；最后刷新云朵标识、菜单勾选与设置窗口。
    pub fn apply_config(&mut self, force: bool) {
        let config = self.settings.config().clone();
        self.engine.set_fuzzy(config.fuzzy);
        self.engine.set_traditional(config.general.traditional());
        self.engine
            .set_full_width_punctuation(config.general.full_width_punctuation);
        self.apply_custom_phrases(&config);
        self.engine.set_mode_keys(config.shortcut.mode);
        self.engine.set_chinese_first(config.general.chinese_first);
        self.engine
            .set_emoji_limit(usize::from(config.general.emoji.get()));
        self.engine
            .set_kaomoji_limit(usize::from(config.general.kaomoji.get()));
        self.engine
            .set_symbol_limit(usize::from(config.general.symbols.get()));
        self.engine
            .set_english_in_chinese(config.general.english_in_chinese);
        self.engine
            .set_english_max_in_chinese(config.general.english_max_in_chinese());
        self.engine
            .set_english_min_position(config.general.english_min_position());
        self.engine.set_auto_space(config.general.auto_space);
        self.engine
            .set_shift_letter_compose(config.general.shift_letter.compose());
        self.apply_scheme(&config);
        self.engine
            .set_aux_code_key(config.general.aux_code_key(), config.general.page_keys());
        self.engine
            .set_aux_keep_empty(config.general.aux_code_keep_empty);
        self.engine.set_aux_enabled(config.aux_code.enabled);
        self.engine.set_aux_show(config.general.aux_code_show);
        self.engine.set_show_tones(config.general.show_tones);
        self.aux_code_show = config.general.aux_code_show;
        if force || self.applied_aux_code.as_ref() != Some(&config.aux_code) {
            self.reload_aux_codes();
        }
        self.engine
            .set_shuangpin_raw_preedit(config.general.shuangpin_raw_preedit);
        self.engine
            .set_hold_selections(config.general.hold_selections);
        self.engine
            .set_context_memory(config.model.context_memory_seconds);
        self.engine.set_learning(config.general.learning);
        logging::set_level(config.general.log_level);
        self.translation_keys = config.shortcut.translation_keys();
        self.translate_keys = config.shortcut.translate_selection;
        self.punctuation_keys = config.shortcut.toggle_punctuation;
        self.predict_keys = config.shortcut.predict;
        self.page_size = config.general.page_size();
        self.cloud_slots = config.predict.slots;
        self.page_keys = config.general.page_keys();
        self.extra_page_keys = config.shortcut.extra_page_keys;
        self.highlight_keys = config.shortcut.highlight_keys;
        self.punct_commits = config.general.punct_commits;
        self.preedit_mode = config.general.preedit;
        self.translation_pos = config.general.translation_pos;
        self.english_candidates = config.general.english_candidates;
        self.apps = config.apps.clone();
        self.window.set_appearance(config.general.appearance());
        self.key_hint_mode = config.general.key_hint;
        self.window
            .set_key_hint_opacity(config.general.key_hint_opacity());
        self.window
            .set_key_hint_size(config.general.key_hint_size());
        self.window.set_layout(config.general.layout);
        if self.layout != config.general.layout
            || self.horizontal_grid != config.general.horizontal_grid
        {
            self.layout = config.general.layout;
            self.horizontal_grid = config.general.horizontal_grid;
            self.session.collapse();
        }
        self.window.set_font(&config.general.font);
        self.themes.refresh();
        self.window
            .set_render_theme(self.render_theme(&config.general));
        self.window.set_renderer(config.general.renderer);
        self.apply_learning_language(&config.general);
        if self.input_log_enabled != Some(config.general.input_log) {
            self.input_log_enabled = Some(config.general.input_log);
            self.open_input_log(config.general.input_log);
        }
        if force || config.predict != self.applied_predict {
            if config.predict.enabled {
                // 请求与 Token 记进统计页的云服务用量
                let usage = paths::user_data_dir().map(|dir| dir.join(API_USAGE_FILE));
                // 没密钥等失败只记日志、退回不联想：输入优先于一切附加功能
                match CloudPredictor::new_with_usage(&config.predict, usage.clone()) {
                    Ok(predictor) => self.engine.set_predictor(Box::new(predictor)),
                    Err(error) => {
                        tracing::warn!(%error, "云联想未启用");
                        self.engine.set_predictor(Box::new(NoPredictor));
                    }
                }
                // 释义兜底随云联想一起开：释义表里没有的词上屏后问云端写进个人释义表
                match CloudGlossFiller::new_with_usage(&config.predict, usage) {
                    Ok(filler) => self.engine.set_gloss_filler(Box::new(filler)),
                    Err(error) => {
                        tracing::warn!(%error, "释义兜底未启用");
                        self.engine.set_gloss_filler(Box::new(NoGlossFiller));
                    }
                }
            } else {
                self.engine.set_predictor(Box::new(NoPredictor));
                self.engine.set_gloss_filler(Box::new(NoGlossFiller));
            }
            self.monitor.stop();
            self.sentence = None;
            self.applied_predict = config.predict.clone();
        }
        if force || config.dictionaries != self.applied_dictionaries {
            self.reload_dictionaries();
        }
        // 只有开关与选的模型变了才换；档位、前文记多久当场改
        let wanted = (config.model.enabled, config.model.name);
        let applied = self
            .applied_model
            .as_ref()
            .map(|model| (model.enabled, model.name));
        if applied != Some(wanted) {
            self.unload_local_model();
            if config.model.enabled {
                self.load_local_model(config.model.name);
            }
        }
        self.engine
            .set_choice_tier(config.model.zhiyi_tier.choice_tier());
        self.applied_model = Some(config.model.clone());
        let cloud_active = self.engine.prediction_enabled();
        self.indicator.set_cloud(cloud_active);
        self.indicator.update();
        self.menu.sync(&config, cloud_active, self.settings.error());
        let key_present = config
            .predict
            .api_key
            .as_deref()
            .is_some_and(|key| !key.trim().is_empty())
            || std::env::var(&config.predict.api_key_env).is_ok_and(|key| !key.trim().is_empty());
        self.dictionary_list = self.dictionary_infos();
        self.preferences.sync(
            &config,
            key_present,
            self.settings.error(),
            &self.dictionary_list,
            &self.update_status,
        );
        self.sync_update();
    }

    /// 配置里的自定义短语，`[general] system_text_replacements` 开着时再并上系统的文本替换，一起推给 Engine。
    fn apply_custom_phrases(&mut self, config: &qingjian_platform::Config) {
        let phrases = if config.general.system_text_replacements {
            qingjian_core::custom_phrase::merge_replacements(
                &config.custom_phrases,
                self.text_replacements
                    .iter()
                    .map(|(code, text)| (code.as_str(), text.as_str())),
            )
        } else {
            config.custom_phrases.clone()
        };
        if let Err(error) = self.engine.set_custom_phrases(phrases) {
            tracing::warn!(%error, "自定义短语配置未应用");
        }
    }

    /// 重读系统的文本替换（激活输入法时调，系统设置里改过就能跟上）；列表变了才重新套用短语。
    pub fn refresh_text_replacements(&mut self) {
        let latest = text_replacements::read_system();
        if latest == self.text_replacements {
            return;
        }
        tracing::info!(count = latest.len(), "系统文本替换已读取");
        self.text_replacements = latest;
        let config = self.settings.config().clone();
        if config.general.system_text_replacements {
            self.apply_custom_phrases(&config);
        }
    }

    /// 学习语言变了就换释义表，`off` 换成不翻译；文件缺失或坏了保持原样，只记日志。
    pub(super) fn apply_learning_language(&mut self, general: &GeneralConfig) {
        if general.learning_language_off() {
            if self.learning_language.take().is_some() {
                self.engine.set_translator(Box::new(NoTranslator));
                tracing::info!("学习语言已关，不显示译文");
            }
            return;
        }
        let code = general.learning_language.as_str();
        let Ok(language) = code.parse::<Language>() else {
            tracing::warn!(code, "不认识的学习语言，保持不变");
            return;
        };
        if self.learning_language == Some(language) {
            return;
        }
        match load_glossary(language) {
            Ok(glossary) => {
                tracing::info!(
                    language = language.code(),
                    glosses = glossary.len(),
                    "释义表已切换"
                );
                self.engine.set_translator(Box::new(glossary));
                self.learning_language = Some(language);
            }
            Err(error) => tracing::warn!(%error, "释义表加载失败，学习语言不变"),
        }
    }

    /// 配置文件被手改过就热加载；激活输入法时和监视定时器都会调。
    pub fn reload_config_if_changed(&mut self) {
        if self.settings.reload_if_changed() {
            self.apply_config(false);
        }
        self.reload_aux_codes_if_changed();
        if code_word_files() != self.code_word_files {
            let config = self.settings.config().clone();
            self.apply_scheme(&config);
        }
        if super::dictionaries::dictionary_files() != self.dictionary_files {
            self.reload_dictionaries();
        }
    }

    /// 激活期间的定时器每秒调一次：看配置文件，再看学习数据要不要落盘。
    /// 学习数据原本只在停用输入法时保存，进程被 launchd 杀掉就丢一整段；现在最多丢 [`LEARNING_FLUSH_INTERVAL`] 这么久。
    /// 配置里选的主题，盖上设置里的字号与动画开关。
    fn render_theme(&self, general: &GeneralConfig) -> qingjian_render::Theme {
        let sizes = TextSizes {
            candidate: general.candidate_font_size.get(),
            annotation: general.annotation_font_size.get(),
        };
        self.themes
            .resolve(general.theme_id(), false)
            .with_text_sizes(sizes)
            .with_animations(general.animations)
    }

    /// 没有新数据时 flush 是空操作（各表按 dirty 位判断），不会每分钟碰一次磁盘。
    pub fn tick(&mut self) {
        self.reload_config_if_changed();
        // 用户主题目录里的 theme.json 改了就重读，当前主题当场换上（边改主题边看效果）
        if self.themes.refresh() {
            let general = self.settings.config().general.clone();
            tracing::info!(id = general.theme_id(), "主题目录有变化，已重读");
            self.window.set_render_theme(self.render_theme(&general));
        }
        let learned = self.engine.poll_glosses();
        if learned > 0 {
            tracing::info!(learned, "释义兜底写入个人释义表");
        }
        if self.last_flush.elapsed() >= LEARNING_FLUSH_INTERVAL {
            self.engine.flush_learning();
            self.last_flush = std::time::Instant::now();
        }
        if let Some(updates) = &self.updates {
            updates.poll(&self.settings.config().update);
        }
        self.sync_update();
    }

    /// 检查更新的状态变了就刷菜单里的「有新版本」与「关于」页；没变什么都不做。
    pub fn sync_update(&mut self) {
        let Some(updates) = &self.updates else {
            return;
        };
        let config = self.settings.config();
        let status = UpdateStatus {
            available: updates.available(&config.update).map(|found| found.version),
            checking: updates.checking(),
            checked: updates.checked_at() > 0,
            dev_build: updates.is_dev_build(),
        };
        if status != self.update_status {
            self.menu.sync_update(status.available.as_deref());
            self.preferences.sync_update(config, &status);
            self.update_status = status;
        }
    }

    /// 按配置的两条轴装配引擎：拼音侧（全拼 / 双拼 / 注音 / 关）与形码侧（随包或用户方案），
    /// 找法、退回拼音与「没变不重读」见 [`qingjian_platform::SchemeSlot`]。
    fn apply_scheme(&mut self, config: &Config) {
        self.scheme_slot.apply(
            &mut self.engine,
            config,
            paths::user_data_dir().as_deref(),
            paths::bundled_schemes_dir().as_deref(),
        );
        self.code_word_files = code_word_files();
    }
}

/// 用户 `code_dicts/` 的快照，只 stat。
fn code_word_files() -> Vec<(std::path::PathBuf, Option<std::time::SystemTime>, u64)> {
    paths::user_data_dir()
        .map(|dir| {
            let dir = qingjian_platform::code_schemes::user_words_dir(&dir);
            qingjian_platform::code_schemes::words_snapshot(&dir)
        })
        .unwrap_or_default()
}
