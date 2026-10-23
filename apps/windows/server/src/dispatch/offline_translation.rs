//! 为 Engine 装配进程内选区翻译；壳只传目录，不执行分词或模型推理。

use std::path::Path;

use qingjian_core::{Engine, NoPredictor, Predictor};

pub fn attach_offline_translation(
    engine: &mut Engine,
    enabled: bool,
    user: Option<&Path>,
    root: &Path,
) {
    if !enabled {
        tracing::info!("本地选区翻译未开启（[offline_translation] enabled = false）");
        engine.set_translation_predictor(None);
        return;
    }
    let runtime = [
        root.join("onnxruntime.dll"),
        root.join("target/installer/onnxruntime/onnxruntime.dll"),
    ]
    .into_iter()
    .find(|path| path.is_file());
    let provider: Box<dyn Predictor> = match user.zip(runtime) {
        Some((user, runtime)) => match qingjian_offline::NativePredictor::new(
            user.join(qingjian_offline::MODEL_DIR),
            runtime,
        ) {
            Ok(provider) => {
                tracing::info!("本地选区翻译已启用");
                Box::new(provider)
            }
            Err(error) => {
                tracing::warn!(%error, "本地翻译线程启动失败");
                Box::new(NoPredictor)
            }
        },
        None => {
            tracing::warn!("缺少本地翻译目录或 ONNX 运行库");
            Box::new(NoPredictor)
        }
    };
    // 选择本地翻译后，即使加载失败也不把原文改发到云端。
    engine.set_translation_predictor(Some(provider));
}
