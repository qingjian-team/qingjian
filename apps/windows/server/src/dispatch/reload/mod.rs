//! 配置热加载：空闲时看 `config.toml` 的 mtime，改了就重读并应用（与 macOS 壳对齐）。
//! 便宜的设置无条件重设；云联想 / 附加词库只在对应分节变了才重建。热加载状态在 [`ConfigReload`]。

mod state;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use qingjian_core::{Engine, NoGlossFiller, NoPredictor};
use qingjian_platform::{Config, extra_dictionaries};
use qingjian_predict::{CloudGlossFiller, CloudPredictor, PredictConfig};

pub(super) use self::state::ConfigReload;
use super::{Router, RouterConfig};

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

impl Router {
    /// `config.toml` 路径；没开热加载（测试）时为 `None`。
    pub(super) fn config_path(&self) -> Option<&Path> {
        self.reload
            .as_ref()
            .map(|reload| reload.config_path.as_path())
    }

    /// 开启热加载：记下路径与当前已应用的 predict / dictionaries。
    pub fn watch_config(
        &mut self,
        config: &Config,
        config_path: PathBuf,
        bundled_dicts_dir: Option<PathBuf>,
        user_dir: Option<PathBuf>,
    ) {
        let last_mtime = mtime(&config_path);
        self.reload = Some(ConfigReload {
            config_path,
            bundled_dicts_dir,
            user_dir,
            last_mtime,
            applied_predict: config.predict.clone(),
            applied_dictionaries: config.dictionaries.clone(),
        });
    }

    /// 空闲时每秒调一次。解析失败保持原配置，mtime 照记（不每秒重试同一个坏文件）。
    pub fn poll_config_reload(&mut self) {
        let Some(reload) = &mut self.reload else {
            return;
        };
        let current = mtime(&reload.config_path);
        if current == reload.last_mtime {
            return;
        }
        reload.last_mtime = current;
        let path = reload.config_path.clone();
        match Config::load(&path) {
            Ok(config) => {
                self.apply_config(&config);
                tracing::info!("配置已热加载");
            }
            Err(error) => tracing::error!(%error, "配置热加载解析失败，保持原配置"),
        }
    }

    /// 应用新配置。学习语言变了仍需重启（要换释义表 / 等级表）。
    fn apply_config(&mut self, config: &Config) {
        self.engine.set_fuzzy(config.fuzzy);
        self.engine.set_shuangpin(config.general.shuangpin());
        self.engine.set_mode_keys(config.shortcut.mode);
        self.config = RouterConfig::from(config);
        self.reconcile_status();

        let Some(reload) = &mut self.reload else {
            return;
        };
        if config.predict != reload.applied_predict {
            attach_cloud(&mut self.engine, &config.predict);
            reload.applied_predict = config.predict.clone();
        }
        if config.dictionaries != reload.applied_dictionaries {
            let dicts = extra_dictionaries::load(
                reload.bundled_dicts_dir.as_deref(),
                reload.user_dir.as_deref(),
                &config.dictionaries,
            );
            tracing::info!(count = dicts.len(), "附加词库已热重装");
            self.engine.set_extra_dictionaries(dicts);
            reload.applied_dictionaries = config.dictionaries.clone();
        }
    }
}
