//! 配置热加载：按键路径上周期探测 mtime，变了就重载并热应用。

use std::path::PathBuf;

use qingjian_platform::Config;

use super::paths::{load_glossary, mtime_of};
use super::{CONFIG_CHECK_INTERVAL, Host};

impl Host {
    /// 开启配置热加载：记下文件与当前 mtime，之后按键路径上周期探测。
    pub fn watch_config(&mut self, path: PathBuf) {
        self.config_mtime = mtime_of(&path);
        self.config_file = Some(path);
    }

    /// 把一份配置整份套用到 Engine 与本地状态（初装与热加载同一条路，漏一处就是静默失效）。
    /// 学习语言的释义表切换不在这里：要 data_dir 且初装时已装好，见 [`Self::maybe_reload_config`]。
    pub(super) fn apply_config(&mut self, config: &Config) {
        self.engine.set_fuzzy(config.fuzzy);
        self.engine
            .set_full_width_punctuation(config.general.full_width_punctuation);
        if let Err(error) = self
            .engine
            .set_custom_phrases(config.custom_phrases.clone())
        {
            tracing::warn!(%error, "自定义短语配置未应用");
        }
        self.engine.set_mode_keys(config.shortcut.mode);
        self.engine.set_shuangpin(config.general.shuangpin());
        self.translation_mods = config.shortcut.translation_keys();
        self.delete_mods = config.shortcut.delete_keys();
        self.page_size = config.general.page_size();
        self.page_keys = config.general.page_keys();
        self.english_candidates = config.general.english_candidates;
        self.apps = config.apps.clone();
        self.preedit_mode = config.general.preedit;
        crate::logging::set_level(config.general.log_level);
        // 附加词库（随包领域词库 + 用户目录 user-dicts/）：整份重扫——只有配置真变了才走到这里，
        // 重扫一次的代价是 mmap 十来个文件，可接受，省一份「上次配置」状态。
        self.engine
            .set_extra_dictionaries(qingjian_platform::extra_dictionaries::load(
                Some(&self.data_dir.join("dist/dicts")),
                Some(&self.data_dir.join("user-dicts")),
                &config.dictionaries,
            ));
        // 输入日志：变了才换 logger（换会 flush 旧的）。
        if self.input_log_enabled != Some(config.general.input_log) {
            self.input_log_enabled = Some(config.general.input_log);
            if config.general.input_log {
                let path = self.data_dir.join("input-log.jsonl");
                tracing::info!(path = %path.display(), "输入日志开着");
                self.engine
                    .set_input_logger(Box::new(qingjian_learning::InputLog::open(path)));
                self.engine.log_session(env!("CARGO_PKG_VERSION"), "linux");
            } else {
                self.engine
                    .set_input_logger(Box::new(qingjian_core::engine::NoInputLogger));
            }
        }
        if config.model.enabled != self.model_enabled {
            self.model_enabled = config.model.enabled;
            if self.model_enabled {
                self.load_local_model();
            } else {
                self.unload_local_model();
            }
        }
    }

    /// 配置文件变了就重载并热应用（整份套用见 [`Self::apply_config`]，另加学习语言的释义表切换）。
    pub(super) fn maybe_reload_config(&mut self) {
        if self.last_config_check.elapsed() < CONFIG_CHECK_INTERVAL {
            return;
        }
        self.last_config_check = std::time::Instant::now();
        let Some(path) = self.config_file.clone() else {
            return;
        };
        let mtime = mtime_of(&path);
        if mtime == self.config_mtime {
            return;
        }
        self.config_mtime = mtime;
        let config = match Config::load(&path) {
            Ok(config) => config,
            Err(error) => {
                // 笔误不打断输入：保留旧配置继续跑，等用户改对再生效。
                tracing::error!(%error, "配置重载失败,维持旧配置");
                return;
            }
        };
        self.apply_config(&config);
        let wanted = load_glossary(&self.data_dir, &config.general.learning_language);
        if wanted.as_ref().map(|(l, _)| *l) != self.learning_language
            && let Some((language, glossary)) = wanted
        {
            self.engine.set_translator(Box::new(glossary));
            self.learning_language = Some(language);
        }
        tracing::info!("配置已热加载");
        if self.composing() {
            self.refresh();
        }
    }
}
