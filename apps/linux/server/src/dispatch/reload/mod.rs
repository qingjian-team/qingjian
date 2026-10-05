//! 配置热加载：空闲时看 `config.toml` 的 mtime，改了就重读并应用（与 Windows Server 的 `dispatch/reload` 对齐）。
//! 只重设启动时会应用的那些项；便宜的无条件重设，本地模型 / 释义表 / 附加词库按配置变化重建，
//! 用户 `dicts/` 目录的文件增删也跟着重装。拼音显示位置在会话握手时交给插件，改了要重启服务。

mod state;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use qingjian_core::{Language, NoTranslator};
use qingjian_platform::Config;

pub(crate) use self::state::ConfigReload;
use super::{Router, RouterConfig};
use crate::assembly;

/// 看配置文件 mtime 的最短间隔。
const CONFIG_POLL_INTERVAL: Duration = Duration::from_secs(1);

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

impl Router {
    /// 开启热加载：记下路径、数据目录与当前已应用的 dictionaries / model / 学习语言。
    pub fn watch_config(
        &mut self,
        config: &Config,
        config_path: PathBuf,
        root: PathBuf,
        user_dir: PathBuf,
    ) {
        let mut reload = ConfigReload {
            last_mtime: mtime(&config_path),
            config_path,
            last_check: Instant::now(),
            root,
            user_dir,
            dictionary_files: Vec::new(),
            applied_dictionaries: config.dictionaries.clone(),
            applied_model: config.model.clone(),
            applied_language: assembly::learning_language(config),
        };
        reload.dictionary_files = reload.dict_snapshot();
        self.reload = Some(reload);
    }

    /// `tick` 里调；一秒内只真正看一次。解析失败保持原配置，mtime 照记（不每秒重试同一个坏文件）。
    pub(super) fn poll_config_reload(&mut self) {
        let Some(reload) = &mut self.reload else {
            return;
        };
        if reload.last_check.elapsed() < CONFIG_POLL_INTERVAL {
            return;
        }
        reload.last_check = Instant::now();
        let files = reload.dict_snapshot();
        if files != reload.dictionary_files {
            self.engine
                .set_extra_dictionaries(reload.load_dictionaries());
            reload.dictionary_files = files;
        }
        let current = mtime(&reload.config_path);
        if current == reload.last_mtime {
            return;
        }
        reload.last_mtime = current;
        match Config::load(&reload.config_path) {
            Ok(config) => {
                self.apply_config(&config);
                tracing::info!("配置已热加载");
            }
            Err(error) => tracing::error!(%error, "配置热加载解析失败，保持原配置"),
        }
    }

    /// 应用新配置，项目与 `main.rs` 启动时设的一致。
    fn apply_config(&mut self, config: &Config) {
        self.engine.set_fuzzy(config.fuzzy);
        self.engine.set_shuangpin(config.general.shuangpin());
        self.engine
            .set_shuangpin_raw_preedit(config.general.shuangpin_raw_preedit);
        self.engine.set_zhuyin_mode(config.general.is_zhuyin());
        self.engine
            .set_shift_letter_compose(config.general.shift_letter.compose());
        self.engine.set_learning(config.general.learning);
        self.engine.set_chinese_first(config.general.chinese_first);
        self.engine.set_mode_keys(config.shortcut.mode);
        if let Err(error) = self
            .engine
            .set_custom_phrases(config.custom_phrases.clone())
        {
            tracing::warn!(%error, "自定义短语有误，保持原来的");
        }
        self.config = RouterConfig::from(config);
        self.config.page_size = self.config.page_size.clamp(1, 9);

        let Some(mut reload) = self.reload.take() else {
            return;
        };
        if config.model != reload.applied_model {
            self.apply_model_config(&config.model);
            reload.applied_model = config.model.clone();
        }
        let language = assembly::learning_language(config);
        if language != reload.applied_language && self.swap_translator(language, &reload) {
            reload.applied_language = language;
        }
        if config.dictionaries != reload.applied_dictionaries {
            reload.applied_dictionaries = config.dictionaries.clone();
            self.engine
                .set_extra_dictionaries(reload.load_dictionaries());
        }
        self.reload = Some(reload);
    }

    /// 学习语言变了就换释义表：关是不翻译；没有这门语言的表或装不上就保持原样。换成功（或关掉）返回 true。
    fn swap_translator(&mut self, language: Option<Language>, reload: &ConfigReload) -> bool {
        let Some(language) = language else {
            self.engine.set_translator(Box::new(NoTranslator));
            tracing::info!("学习语言已关，不显示译文");
            return true;
        };
        let Some(path) = assembly::glossary_file(&reload.root, language) else {
            tracing::warn!(
                language = language.code(),
                "没有这门语言的释义表，学习语言不变"
            );
            return false;
        };
        match assembly::load_glossary(language, &path, Some(&reload.user_dir)) {
            Ok(glossary) => {
                tracing::info!(language = language.code(), "释义表已切换");
                self.engine.set_translator(Box::new(glossary));
                true
            }
            Err(error) => {
                tracing::warn!(%error, "释义表加载失败，学习语言不变");
                false
            }
        }
    }
}
