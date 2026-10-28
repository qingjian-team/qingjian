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
                tracing::info!(models = %user.join(qingjian_offline::MODEL_DIR).display(), "本地选区翻译已启用（模型按需加载）");
                Box::new(provider)
            }
            Err(error) => {
                tracing::warn!(%error, "本地翻译线程启动失败");
                Box::new(NoPredictor)
            }
        },
        None => {
            tracing::warn!(user_directory_available = user.is_some(), runtime_available = root.join("onnxruntime.dll").is_file() || root.join("target/installer/onnxruntime/onnxruntime.dll").is_file(), bundle_root = %root.display(), "已开启离线翻译，但缺少用户目录或 ONNX 运行库，提供方未就绪");
            Box::new(NoPredictor)
        }
    };
    // 选择本地翻译后，即使加载失败也不把原文改发到云端。
    engine.set_translation_predictor(Some(provider));
    if !engine.translation_enabled() {
        tracing::warn!("离线翻译配置已开启，但提供方未就绪；请检查前面的目录或线程启动错误");
    }
}
