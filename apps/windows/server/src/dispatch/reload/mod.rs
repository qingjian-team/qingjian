//! 配置热加载：空闲时看 `config.toml` 的 mtime，改了就重读并应用（与 macOS 壳对齐）。
//! 便宜的设置无条件重设；云联想 / 释义表按配置变化重建，附加词库也检查文件增删与更新。热加载状态在 [`ConfigReload`]。

mod state;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use qingjian_core::{Engine, Language, NoGlossFiller, NoPredictor, NoTranslator};
use qingjian_platform::{Config, code_tables};
use qingjian_predict::{CloudGlossFiller, CloudPredictor, PredictConfig};

pub(super) use self::state::ConfigReload;
pub use self::state::DataDirs;

/// 看配置文件 mtime 的最短间隔；工人循环空闲时按它等，重排的短节拍来得更勤时按这个节流。
pub(super) const CONFIG_POLL_INTERVAL: Duration = Duration::from_secs(1);
use super::{Router, RouterConfig};
use crate::assembly;

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// 按 `[predict]` 接云联想与释义兜底；关着或缺密钥就退回本地实现。启动与热加载共用。
pub fn attach_cloud(engine: &mut Engine, predict: &PredictConfig) {
    if !predict.enabled {
        tracing::info!("云联想未开启（[predict] enabled = false）");
        engine.set_predictor(Box::new(NoPredictor));
        engine.set_gloss_filler(Box::new(NoGlossFiller));
        return;
    }
    match CloudPredictor::new(predict) {
        Ok(predictor) => {
            engine.set_predictor(Box::new(predictor));
            tracing::info!(model = %predict.model, "云联想已接入");
        }
        Err(error) => {
            tracing::warn!(%error, "云联想接入失败（缺 API key？），退回本地候选");
            engine.set_predictor(Box::new(NoPredictor));
        }
    }
    match CloudGlossFiller::new(predict) {
        Ok(filler) => engine.set_gloss_filler(Box::new(filler)),
        Err(error) => {
            tracing::warn!(%error, "释义兜底未启用");
            engine.set_gloss_filler(Box::new(NoGlossFiller));
        }
    }
}

/// 学习语言变了就换释义表：关是不翻译；换语言重装随包 + 个人释义表，没有这门语言的表或装不上就保持原样。
/// 换成功（或关掉）返回 true。
fn swap_translator(
    engine: &mut Engine,
    language: Option<Language>,
    root: &Path,
    user_dir: Option<&Path>,
) -> bool {
    let Some(language) = language else {
        engine.set_translator(Box::new(NoTranslator));
        tracing::info!("学习语言已关，不显示译文");
        return true;
    };
    let Some(path) = assembly::glossary_file(root, language) else {
        tracing::warn!(
            language = language.code(),
            "没有这门语言的释义表，学习语言不变"
        );
        return false;
    };
    match assembly::load_glossary(language, &path, user_dir) {
        Ok(glossary) => {
            tracing::info!(language = language.code(), "释义表已切换");
            engine.set_translator(Box::new(glossary));
            true
        }
        Err(error) => {
            tracing::warn!(%error, "释义表加载失败，学习语言不变");
            false
        }
    }
}

impl Router {
    /// `config.toml` 路径；没开热加载（测试）时为 `None`。
    pub(super) fn config_path(&self) -> Option<&Path> {
        self.reload
            .as_ref()
            .map(|reload| reload.config_path.as_path())
    }

    /// 开启热加载：记下路径与当前已应用的 predict / dictionaries / aux_code / 学习语言，
    /// 以及启动用的那批数据目录。目录必须与启动同款语义（`dicts/` / `codes/`），
    /// 热加载才找得到文件。
    pub fn watch_config(
        &mut self,
        config: &Config,
        config_path: PathBuf,
        root: PathBuf,
        dirs: DataDirs,
    ) {
        let last_mtime = mtime(&config_path);
        let code_files = dirs.code_snapshot();
        let dictionary_files = dirs.dict_snapshot();
        self.reload = Some(ConfigReload {
            config_path,
            last_check: Instant::now(),
            root,
            dirs,
            code_files,
            last_mtime,
            applied_predict: config.predict.clone(),
            applied_dictionaries: config.dictionaries.clone(),
            applied_aux_code: config.aux_code.clone(),
            dictionary_files,
            applied_language: assembly::learning_language(config),
        });
    }

    /// 空闲时调；一秒内只真正看一次。配置文件或用户 `codes/` 下的文件变了就重装；
    /// 解析失败保持原配置，mtime 照记（不每秒重试同一个坏文件）。
    pub fn poll_config_reload(&mut self) {
        let Some(reload) = &mut self.reload else {
            return;
        };
        if reload.last_check.elapsed() < CONFIG_POLL_INTERVAL {
            return;
        }
        reload.last_check = Instant::now();
        // 用户 `dicts/` 目录文件增删或更新：与配置改动无关，下一拍就生效
        let files = reload.dirs.dict_snapshot();
        if files != reload.dictionary_files {
            // 配置损坏也继续使用上次有效的词库开关；文件变化不触发配置重试。
            self.engine
                .set_extra_dictionaries(reload.load_dictionaries());
            reload.dictionary_files = files;
        }
        let config_changed = {
            let current = mtime(&reload.config_path);
            let changed = current != reload.last_mtime;
            reload.last_mtime = current;
            changed
        };
        // 用户 `codes/` 下的文件增删或更新（设置页刚导入 / 移除一张码表）：不必等配置改动，下一拍就生效
        let codes_changed = {
            let current = reload.dirs.code_snapshot();
            let changed = current != reload.code_files;
            reload.code_files = current;
            changed
        };
        let path = reload.config_path.clone();
        if codes_changed {
            self.reload_aux_codes();
        }
        if !config_changed {
            return;
        }
        match Config::load(&path) {
            Ok(config) => {
                self.apply_config(&config);
                tracing::info!("配置已热加载");
            }
            Err(error) => tracing::error!(%error, "配置热加载解析失败，保持原配置"),
        }
    }

    /// 应用新配置。学习语言变了换释义表（词汇等级表启动时已全装，不用换）。
    fn apply_config(&mut self, config: &Config) {
        self.engine.set_fuzzy(config.fuzzy);
        // 拼音侧与形码侧一起装配（双拼 / 注音 / 混输都在里面）
        self.reload_code_table(config.general.scheme(), config.general.wubi());
        self.engine.set_traditional_mode(config.general.traditional);
        self.engine.set_learning(config.general.learning);
        self.engine.set_mode_keys(config.shortcut.mode);
        self.engine
            .set_aux_code_key(config.general.aux_code_key(), config.general.page_keys());
        self.engine
            .set_aux_keep_empty(config.general.aux_code_keep_empty);
        self.engine.set_aux_enabled(config.aux_code.enabled);
        self.engine.set_aux_show(config.general.aux_code_show);
        self.engine.set_chinese_first(config.general.chinese_first);
        self.engine
            .set_shift_letter_compose(config.general.shift_letter.compose());
        let previous = self.config.render_settings();
        self.config = RouterConfig::from(config);
        let settings = self.config.render_settings();
        if settings != previous {
            self.candidates.configure(settings);
        }
        self.reconcile_status();
        self.apply_model_config(&config.model);

        let Some(reload) = &mut self.reload else {
            return;
        };
        if config.predict != reload.applied_predict {
            attach_cloud(&mut self.engine, &config.predict);
            reload.applied_predict = config.predict.clone();
        }
        let language = assembly::learning_language(config);
        if language != reload.applied_language
            && swap_translator(
                &mut self.engine,
                language,
                &reload.root,
                reload.dirs.user_root.as_deref(),
            )
        {
            reload.applied_language = language;
        }
        if config.dictionaries != reload.applied_dictionaries {
            reload.applied_dictionaries = config.dictionaries.clone();
            self.engine
                .set_extra_dictionaries(reload.load_dictionaries());
        }
        if config.aux_code != reload.applied_aux_code {
            reload.applied_aux_code = config.aux_code.clone();
            self.reload_aux_codes();
        }
    }

    /// 按当前配置重装辅码码表：`codes/` 目录变了或 `[aux_code]` 变了都走这里。
    fn reload_aux_codes(&mut self) {
        let Some(reload) = &self.reload else {
            return;
        };
        let tables = code_tables::load(
            reload.dirs.bundled_codes.as_deref(),
            reload.dirs.user_codes.as_deref(),
            &reload.applied_aux_code,
        );
        if let Some(reload) = &mut self.reload {
            reload.code_files = reload.dirs.code_snapshot();
        }
        tracing::info!(count = tables.len(), "辅码码表已重装");
        self.engine.set_aux_codes(tables);
    }
}
