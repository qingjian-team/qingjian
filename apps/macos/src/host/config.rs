//! 配置热加载：config.toml 改了就整份重新套用到 Engine 与窗口；激活期间的定时事务。

use super::init::load_glossary;
use super::*;

impl Host {
    /// 把当前配置推给 Engine 与界面：模糊音 / 模式键 / 翻页 / 外观直接设；学习语言变了换释义表；
    /// `[predict]` 变了（或 `force`）才重建 Predictor；最后刷新云朵标识、菜单勾选与设置窗口。
    pub fn apply_config(&mut self, force: bool) {
        let config = self.settings.config().clone();
        self.engine.set_fuzzy(config.fuzzy);
        self.engine.set_mode_keys(config.shortcut.mode);
        self.engine.set_shuangpin(config.general.shuangpin());
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
        self.apply_learning_language(&config.general.learning_language);
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

    /// 学习语言变了就换释义表；文件缺失或坏了保持原样，只记日志。
    pub(super) fn apply_learning_language(&mut self, code: &str) {
        let Ok(language) = code.parse::<Language>() else {
            tracing::warn!(code, "不认识的学习语言，保持不变");
            return;
        };
        if language == self.learning_language {
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
                self.learning_language = language;
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
}
