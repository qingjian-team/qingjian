//! 配置热加载：Server 每秒空闲时看 `config.toml` 的 mtime，改了就重读并重新应用到 Engine 与 [`RouterConfig`]，
//! 不用重启 Server（与 macOS 壳每秒看 mtime 对齐）。便宜的设置（模糊音 / 双拼 / 模式键 / 每页数等）无条件重设，
//! 贵的（云联想重建、词库重装）按 `[predict]` / `[dictionaries]` 有没有变来做。

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use qingjian_core::{Engine, NoGlossFiller, NoPredictor};
use qingjian_platform::{Config, DictionariesConfig, extra_dictionaries};
use qingjian_predict::{CloudGlossFiller, CloudPredictor, PredictConfig};

use super::{Router, RouterConfig};

/// 热加载所需的状态：配置路径、随包 / 用户词库目录、上次的 mtime，与已应用的 predict / dictionaries（判有没有变）。
pub struct ConfigReload {
    /// `config.toml` 路径。
    config_path: PathBuf,

    /// 随包领域词库目录（重装词库用）。
    bundled_dicts_dir: Option<PathBuf>,

    /// 用户数据目录（用户导入词库在其 `dicts/` 下）。
    user_dir: Option<PathBuf>,

    /// 上次看到的文件 mtime；变了才重载。
    last_mtime: Option<SystemTime>,

    /// 已应用的 `[predict]`：变了才重建云联想。
    applied_predict: PredictConfig,

    /// 已应用的 `[dictionaries]`：变了才重装附加词库。
    applied_dictionaries: DictionariesConfig,
}

/// 文件 mtime，取不到为 `None`。
fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// 按 `[predict]` 接云联想与释义兜底；关着或缺密钥就退回本地（`NoPredictor` / `NoGlossFiller`）。
fn attach_cloud(engine: &mut Engine, predict: &PredictConfig) {
    if !predict.enabled {
        engine.set_predictor(Box::new(NoPredictor));
        engine.set_gloss_filler(Box::new(NoGlossFiller));
        return;
    }
    match CloudPredictor::new(predict) {
        Ok(predictor) => engine.set_predictor(Box::new(predictor)),
        Err(error) => {
            tracing::warn!(%error, "云联想接入失败，退回本地");
            engine.set_predictor(Box::new(NoPredictor));
        }
    }
    match CloudGlossFiller::new(predict) {
        Ok(filler) => engine.set_gloss_filler(Box::new(filler)),
        Err(error) => {
            tracing::warn!(%error, "释义兜底接入失败，退回本地");
            engine.set_gloss_filler(Box::new(NoGlossFiller));
        }
    }
}

impl Router {
    /// 开启配置热加载：记下路径与当前已应用的 predict / dictionaries，之后 [`Self::poll_config_reload`] 到 mtime 变了就重载。
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

    /// 空闲时每秒调一次：`config.toml` 的 mtime 变了就重读并应用。解析失败保持原配置、只记错误（不每秒重试同一个坏文件）。
    pub fn poll_config_reload(&mut self) {
        let Some(reload) = &self.reload else {
            return;
        };
        let current = mtime(&reload.config_path);
        if current == reload.last_mtime {
            return;
        }
        let path = reload.config_path.clone();
        if let Some(reload) = &mut self.reload {
            reload.last_mtime = current;
        }
        match Config::load(&path) {
            Ok(config) => {
                self.apply_config(&config);
                tracing::info!("配置已热加载");
            }
            Err(error) => tracing::error!(%error, "配置热加载解析失败，保持原配置"),
        }
    }

    /// 把新配置应用到 Engine 与 [`RouterConfig`]。学习语言变了仍需重启（要换释义表 / 等级表），不在这里处理。
    fn apply_config(&mut self, config: &Config) {
        self.engine.set_fuzzy(config.fuzzy);
        self.engine.set_shuangpin(config.general.shuangpin());
        self.engine.set_mode_keys(config.shortcut.mode);
        self.config = RouterConfig::from(config);

        // predict / dictionaries 变了才重建，避免每次热加载都重连网络、重装词库。
        let Some(reload) = &self.reload else {
            return;
        };
        let predict_changed = config.predict != reload.applied_predict;
        let dictionaries_changed = config.dictionaries != reload.applied_dictionaries;
        let bundled = reload.bundled_dicts_dir.clone();
        let user = reload.user_dir.clone();

        if predict_changed {
            attach_cloud(&mut self.engine, &config.predict);
            if let Some(reload) = &mut self.reload {
                reload.applied_predict = config.predict.clone();
            }
        }
        if dictionaries_changed {
            let dicts =
                extra_dictionaries::load(bundled.as_deref(), user.as_deref(), &config.dictionaries);
            let count = dicts.len();
            self.engine.set_extra_dictionaries(dicts);
            tracing::info!(count, "附加词库已热重装");
            if let Some(reload) = &mut self.reload {
                reload.applied_dictionaries = config.dictionaries.clone();
            }
        }
    }
}
