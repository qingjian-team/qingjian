//! 后台构建云服务及其本地用量记录，安装到引擎时不读取文件。

use std::path::PathBuf;

use qingjian_core::{Engine, GlossFiller, NoGlossFiller, NoPredictor, Predictor};
use qingjian_predict::{CloudGlossFiller, CloudPredictor, PredictConfig};

pub(crate) struct CloudServices {
    /// 已完成初始化的联想服务，失败或关闭时为本地空实现。
    predictor: Box<dyn Predictor>,
    /// 已完成初始化的释义兜底服务。
    filler: Box<dyn GlossFiller>,
}

impl CloudServices {
    pub(crate) fn load(config: &PredictConfig, usage: Option<PathBuf>) -> Self {
        let mut services = Self {
            predictor: Box::new(NoPredictor),
            filler: Box::new(NoGlossFiller),
        };
        if !config.enabled {
            return services;
        }
        match CloudPredictor::new_with_usage(config, usage.clone()) {
            Ok(predictor) => {
                services.predictor = Box::new(predictor);
                tracing::info!(model = %config.model, "cloud prediction enabled");
            }
            Err(error) => {
                tracing::warn!(%error, "cloud prediction unavailable, using local candidates")
            }
        }
        match CloudGlossFiller::new_with_usage(config, usage) {
            Ok(filler) => services.filler = Box::new(filler),
            Err(error) => tracing::warn!(%error, "cloud glossary fallback unavailable"),
        }
        services
    }

    pub(crate) fn install(self, engine: &mut Engine) {
        engine.set_predictor(self.predictor);
        engine.set_gloss_filler(self.filler);
    }
}
