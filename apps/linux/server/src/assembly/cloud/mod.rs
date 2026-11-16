//! 启动与配置热加载共用的云联想装配；关闭或配置无效时只用本地候选。

use std::path::PathBuf;

use qingjian_core::Engine;
use qingjian_predict::PredictConfig;

mod services;

pub(crate) use services::CloudServices;

/// 按 `[predict]` 接云联想、释义兜底与生词本整句翻译；关着或缺密钥就退回本地实现。
/// 用量累计到 `usage_path`（用户目录的 `api-usage.json`），设置统计页读同一份。启动与热加载共用。
pub fn attach_cloud(engine: &mut Engine, predict: &PredictConfig, usage_path: Option<PathBuf>) {
    CloudServices::load(predict, usage_path).install(engine);
}
