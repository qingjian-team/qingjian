//! 启动与配置热加载共用的云联想装配；关闭或配置无效时只用本地候选。

use std::path::PathBuf;

use qingjian_core::{Engine, NoGlossFiller, NoPredictor};
use qingjian_predict::{CloudGlossFiller, CloudPredictor, PredictConfig};

/// 按 `[predict]` 接云联想、释义兜底与生词本整句翻译；关着或缺密钥就退回本地实现。
/// 用量累计到 `usage_path`（用户目录的 `api-usage.json`），设置统计页读同一份。启动与热加载共用。
pub fn attach_cloud(engine: &mut Engine, predict: &PredictConfig, usage_path: Option<PathBuf>) {
    if !predict.enabled && !predict.sentence_translation {
        engine.set_predictor(Box::new(NoPredictor));
        engine.set_gloss_filler(Box::new(NoGlossFiller));
        return;
    }
    match CloudPredictor::new_with_usage(predict, usage_path.clone()) {
        Ok(predictor) => {
            engine.set_predictor(Box::new(predictor));
            tracing::info!(model = %predict.model, "云联想已启用");
        }
        Err(error) => {
            engine.set_predictor(Box::new(NoPredictor));
            tracing::warn!(%error, "云联想未启用，使用本地候选");
        }
    }
    if predict.enabled {
        match CloudGlossFiller::new_with_usage(predict, usage_path) {
            Ok(filler) => engine.set_gloss_filler(Box::new(filler)),
            Err(error) => {
                engine.set_gloss_filler(Box::new(NoGlossFiller));
                tracing::warn!(%error, "释义兜底未启用");
            }
        }
    } else {
        engine.set_gloss_filler(Box::new(NoGlossFiller));
    }
}
