//! 配置热加载：config.toml 改了就整份重新套用到 Engine 与窗口；激活期间的定时事务。

mod text_replacements;
mod watch;

pub(super) use text_replacements::TextReplacement;
pub(super) use watch::ConfigWatch;

use super::init::load_glossary;
use super::*;

impl Host {
    /// 把当前配置推给 Engine 与界面：模糊音 / 模式键 / 翻页 / 外观直接设；学习语言变了换释义表；
    /// `[predict]` 变了（或 `force`）才重建 Predictor；最后刷新云朵标识、菜单勾选与设置窗口。
    pub fn apply_config(&mut self, force: bool) {
        let config = self.settings.config().clone();
        self.engine.set_fuzzy(config.fuzzy);
        self.engine.set_traditional_mode(config.general.traditional);
        self.engine
            .set_full_width_punctuation(config.general.full_width_punctuation);
        self.apply_custom_phrases(&config);
        self.engine.set_mode_keys(config.shortcut.mode);
        self.engine.set_chinese_first(config.general.chinese_first);
        self.engine
            .set_shift_letter_compose(config.general.shift_letter.compose());
        self.apply_scheme(config.general.scheme(), config.general.wubi());
        self.engine
            .set_shuangpin_raw_preedit(config.general.shuangpin_raw_preedit);
        self.engine.set_learning(config.general.learning);
        logging::set_level(config.general.log_level);
        self.translation_keys = config.shortcut.translation_keys();
        self.delete_keys = config.shortcut.delete_keys();
        self.translate_keys = config.shortcut.translate_selection;
        self.page_size = config.general.page_size();
        self.cloud_slots = config.predict.slots;
        self.page_keys = config.general.page_keys();
        self.preedit_mode = config.general.preedit;
        self.english_candidates = config.general.english_candidates;
        self.apps = config.apps.clone();
        self.window.set_theme(config.general.theme);
        self.window.set_layout(config.general.layout);
        if self.layout != config.general.layout
            || self.horizontal_grid != config.general.horizontal_grid
        {
            self.layout = config.general.layout;
            self.horizontal_grid = config.general.horizontal_grid;
            self.session.collapse();
        }
        self.window.set_font(&config.general.font);
        self.window.set_renderer(config.general.renderer);
        self.apply_learning_language(&config.general);
        if self.input_log_enabled != Some(config.general.input_log) {
            self.input_log_enabled = Some(config.general.input_log);
            self.open_input_log(config.general.input_log);
        }
        if force || config.predict != self.applied_predict {
            if config.predict.enabled {
                // 没密钥等失败只记日志、退回不联想：输入优先于一切附加功能
                match CloudPredictor::new(&config.predict) {
                    Ok(predictor) => self.engine.set_predictor(Box::new(predictor)),
                    Err(error) => {
                        tracing::warn!(%error, "云联想未启用");
                        self.engine.set_predictor(Box::new(NoPredictor));
                    }
                }
                // 释义兜底随云联想一起开：释义表里没有的词上屏后问云端写进个人释义表
                match CloudGlossFiller::new(&config.predict) {
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
        if self.applied_model.as_ref() != Some(&config.model) {
            if config.model.enabled {
                self.load_local_model();
            } else {
                self.unload_local_model();
            }
            self.applied_model = Some(config.model.clone());
        }
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
        );
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
    }

    /// 激活期间的定时器每秒调一次：看配置文件，再看学习数据要不要落盘。
    /// 学习数据原本只在停用输入法时保存，进程被 launchd 杀掉就丢一整段；现在最多丢 [`LEARNING_FLUSH_INTERVAL`] 这么久。
    /// 没有新数据时 flush 是空操作（各表按 dirty 位判断），不会每分钟碰一次磁盘。
    pub fn tick(&mut self) {
        self.reload_config_if_changed();
        let learned = self.engine.poll_glosses();
        if learned > 0 {
            tracing::info!(learned, "释义兜底写入个人释义表");
        }
        if self.last_flush.elapsed() >= LEARNING_FLUSH_INTERVAL {
            self.engine.flush_learning();
            self.last_flush = std::time::Instant::now();
        }
    }

    /// 按配置的两条轴装配引擎：拼音侧（全拼 / 双拼 / 注音 / 关）与形码侧（五笔）。
    /// 两边都开就是混输——编码打全的形码候选在前，见 `Engine::query_mixed`。
    ///
    /// 开了形码却找不到码表时只警告并退回只用拼音——配置说五笔、引擎一个字都打不出更糟。
    fn apply_scheme(&mut self, pinyin: Scheme, wubi: bool) {
        self.engine.set_shuangpin(pinyin.shuangpin());
        self.engine.set_zhuyin_mode(pinyin == Scheme::Zhuyin);
        // 拼音侧关掉且形码开着才是「只用形码」；两边都关着时留拼音兜底（否则一个候选都没有）
        self.engine.set_phonetic(pinyin.is_on() || !wubi);
        if !wubi {
            self.engine.set_code_table(None);
            return;
        }
        let Some(path) = paths::code_table_path() else {
            tracing::warn!("选了形码方案但找不到码表，仍按拼音输入；随包数据里应当带一份");
            self.engine.set_code_table(None);
            return;
        };
        match qingjian_dictionary::CodeTable::from_path(&path) {
            Ok(table) => {
                tracing::info!(table = %path.display(), entries = table.len(), "形码码表已载入");
                self.engine.set_code_table(Some(table));
            }
            Err(error) => {
                tracing::error!(%error, table = %path.display(), "形码码表读不了，仍按拼音输入");
                self.engine.set_code_table(None);
            }
        }
    }
}
