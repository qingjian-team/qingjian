use qingjian_core::PredictionPolicy;
use serde::{Deserialize, Serialize};

/// 云联想配置。默认**关闭**，开启后光标附近的文本会发往 `base_url`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PredictConfig {
    /// offline 只查本地短句，cloud 复用已有云服务；不自动切换收费接口。
    pub provider: String,
    /// 用户主动触发的选区长文翻译后端；cloud 复用已有 API。
    pub selection_provider: String,
    /// 是否启用。
    pub enabled: bool,

    /// OpenAI 兼容接口地址（不含 `/chat/completions`）。
    pub base_url: String,

    /// 模型名。
    pub model: String,

    /// 密钥。留空则读 `api_key_env` 指定的环境变量。
    pub api_key: Option<String>,

    /// 存放密钥的环境变量名。
    pub api_key_env: String,

    /// 单次请求超时（毫秒），超时即丢。
    pub timeout_ms: u64,

    /// 防抖：停止敲键多久之后才真正发请求（毫秒）。
    pub debounce_ms: u64,

    /// 光标前最多发多少个字符。
    pub lookback: usize,

    /// 光标后最多发多少个字符。
    pub lookahead: usize,

    /// 云端词最多补进候选窗口第一页末尾几格；0 表示不要云端词，只要整句补全。
    pub slots: usize,

    /// 组句中要不要整句补全。
    pub sentence: bool,

    /// 推理强度，随请求发 `reasoning_effort`：`none` 关掉模型的思考（联想要的是快，不是想），
    /// 其余 minimal / low / medium / high / xhigh 照传；留空则不发（给不认这个参数的接口）。
    /// DeepSeek V4 这类默认带思考的模型不关会把 token 预算全花在思考上，正文为空。
    pub reasoning_effort: String,

    /// 停顿后为完整中文候选生成整句译文，可用译词快捷键上屏。
    pub sentence_translation: bool,

    /// 长段落翻译每一段网络请求的超时，与日常联想分开。
    pub translation_timeout_ms: u64,

    /// 原文到目标译文的术语映射，每项格式为 原文=英文|日文。
    pub terminology: Vec<String>,
}

impl Default for PredictConfig {
    fn default() -> Self {
        Self {
            provider: "cloud".to_owned(),
            selection_provider: "inherit".to_owned(),
            enabled: false,
            base_url: "https://api.deepseek.com".to_owned(),
            model: "deepseek-v4-flash".to_owned(),
            api_key: None,
            api_key_env: "QINGJIAN_API_KEY".to_owned(),
            timeout_ms: 5000,
            debounce_ms: 300,
            lookback: 64,
            lookahead: 32,
            slots: 2,
            sentence: true,
            reasoning_effort: "none".to_owned(),
            sentence_translation: false,
            translation_timeout_ms: 30_000,
            terminology: Vec::new(),
        }
    }
}

impl PredictConfig {
    /// 配置里的密钥优先，其次环境变量；两边都没有返回 `None`。
    pub fn resolve_api_key(&self) -> Option<String> {
        if self.provider == "offline" && self.selection_provider != "cloud" {
            return Some("offline-no-network".to_owned());
        }
        self.api_key
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(str::to_owned)
            .or_else(|| std::env::var(&self.api_key_env).ok())
            .filter(|k| !k.trim().is_empty())
    }

    pub fn policy(&self) -> PredictionPolicy {
        PredictionPolicy {
            before: self.lookback,
            after: self.lookahead,
            slots: self.slots,
            // 比槽位多要两条，与本地候选重复的去掉后还能填满；问字模式的答案也按这个数要
            max_items: self.slots.max(1) + 2,
            sentence: self.sentence,
            sentence_translation: self.sentence_translation,
        }
    }
}
