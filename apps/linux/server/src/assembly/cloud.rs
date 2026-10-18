//! 启动与配置热加载共用的云联想装配；关闭或配置无效时只用本地候选。

use qingjian_core::{Engine, NoPredictor};
use qingjian_predict::{CloudPredictor, PredictConfig};

pub fn attach_cloud(engine: &mut Engine, config: &PredictConfig) {
    if !config.enabled {
        engine.set_predictor(Box::new(NoPredictor));
        return;
    }
    match CloudPredictor::new(config) {
        Ok(predictor) => {
            engine.set_predictor(Box::new(predictor));
            tracing::info!(model = %config.model, "云联想已启用");
        }
        Err(error) => {
            engine.set_predictor(Box::new(NoPredictor));
            tracing::warn!(%error, "云联想未启用，使用本地候选");
        }
    }
}
