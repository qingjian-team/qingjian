//! 壳交过来的云端配置（JSON，含解好的密钥）与安卓上的请求节奏。

use serde::Deserialize;

use qingjian_predict::PredictConfig;

/// 键盘设置里填的云端配置（Kotlin 解出密钥后整份交过来）。不派生 `Debug`，免得密钥被打进日志。
#[derive(Deserialize)]
pub struct CloudSettings {
    /// OpenAI 兼容接口地址（不含 `/chat/completions`）。
    pub base_url: String,

    pub model: String,

    pub api_key: String,

    /// 云联想与整句补全开着（且当前网络允许）；关着时只做显式的翻译请求。
    #[serde(default)]
    pub predict: bool,
}

impl CloudSettings {
    /// 候选整句翻译、生词本整句翻译全关；[`Self::predict`] 关着时联想与整句补全也关，只有显式的翻译请求会发出去。
    /// 开着时停键 `DEBOUNCE_MS` 后发组句联想，云端词最多 `SLOTS` 个。边打边翻要跟得上，请求间隔比 PC 短；防抖在 worker 里。
    pub fn predict_config(&self) -> PredictConfig {
        PredictConfig {
            enabled: true,
            automatic: self.predict,
            base_url: self.base_url.trim().to_owned(),
            model: self.model.trim().to_owned(),
            api_key: Some(self.api_key.trim().to_owned()),
            slots: if self.predict { SLOTS } else { 0 },
            sentence: self.predict,
            sentence_translation: false,
            vocabulary_translation: false,
            timeout_ms: TIMEOUT_MS,
            debounce_ms: DEBOUNCE_MS,
            min_interval_ms: if self.predict {
                PREDICT_INTERVAL_MS
            } else {
                MIN_INTERVAL_MS
            },
            ..PredictConfig::default()
        }
    }
}

/// 翻译一句话的超时：比联想长，句子长的模型要多想一会儿。
const TIMEOUT_MS: u64 = 10_000;

/// 只翻译时两次请求起始的最短间隔。
const MIN_INTERVAL_MS: u64 = 300;

/// 开着云联想时的最短间隔：手机上停键 500 ms 已经滤掉多数半截请求，再隔 1 秒省流量和费用（PC 是 3 秒）。
const PREDICT_INTERVAL_MS: u64 = 1_000;

/// 停键多久才发联想：手机打字比电脑慢、停顿多，比 PC 的 300 ms 长。
const DEBOUNCE_MS: u64 = 500;

/// 云端词最多几个（同 PC 缺省）。
const SLOTS: usize = 2;
