use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::Path;

use qingjian_core::PredictionPolicy;
use serde::{Deserialize, Serialize};

/// 本机模型服务的缺省端口：避开 llama-server 自己的 8080 等常用端口，也不在系统分配临时端口的区间里。
pub(crate) const DEFAULT_LOCAL_PORT: u16 = 18686;

/// 网络预测与整句候选翻译共用的服务配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PredictConfig {
    /// 是否启用。
    pub enabled: bool,

    /// 停顿后自动联想；关闭时仅显式触发。
    pub automatic: bool,

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

    /// 两次网络请求起始之间的最短间隔（毫秒），缓存命中不等待。
    pub min_interval_ms: u64,

    /// 光标前最多发多少个字符。
    pub lookback: usize,

    /// 光标后最多发多少个字符。
    pub lookahead: usize,

    /// 云端词最多补进候选窗口第一页末尾几格；0 表示不要云端词，只要整句补全。
    pub slots: usize,

    /// 组句中要不要整句补全。
    pub sentence: bool,

    /// 上屏的整句翻成学习语言，记进生词本的时间线（只发那一句）。
    pub vocabulary_translation: bool,

    /// 推理强度，随请求发 `reasoning_effort`：`none` 关掉模型的思考（联想要的是快，不是想），
    /// 其余 minimal / low / medium / high / xhigh 照传；留空则不发（给不认这个参数的接口）。
    /// DeepSeek V4 这类默认带思考的模型不关会把 token 预算全花在思考上，正文为空。
    pub reasoning_effort: String,

    /// 本机模型：GGUF 模型文件的完整路径。填了就用 llama.cpp 的 llama-server 在本机运行它，
    /// 请求都发给它，`base_url`、`model` 与密钥不再使用；留空则用接口。
    pub local_model: String,

    /// llama-server 程序的完整路径；留空则在 PATH 与常见安装位置里找。
    pub local_server: String,

    /// 本机服务监听的端口（只监听 127.0.0.1）；0 按缺省端口算。
    pub local_port: u16,

    /// 追加给 llama-server 的参数，排在青简给的参数后面，同一参数以这里的为准。
    pub local_args: Vec<String>,

    /// 给 llama-server 设的环境变量，加在输入法自己的环境上：有几块显卡时用 `CUDA_VISIBLE_DEVICES` 指定用哪块，
    /// 自己编译的版本要的开关也写这里。让服务要密钥的两个变量照样去掉。
    pub local_env: BTreeMap<String, String>,
}

impl Default for PredictConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            automatic: true,
            base_url: "https://api.deepseek.com".to_owned(),
            model: "deepseek-flash".to_owned(),
            api_key: None,
            api_key_env: "QINGJIAN_API_KEY".to_owned(),
            timeout_ms: 5000,
            debounce_ms: 300,
            min_interval_ms: 3000,
            lookback: 64,
            lookahead: 32,
            slots: 2,
            sentence: true,
            vocabulary_translation: true,
            reasoning_effort: "none".to_owned(),
            local_model: String::new(),
            local_server: String::new(),
            local_port: DEFAULT_LOCAL_PORT,
            local_args: Vec::new(),
            local_env: BTreeMap::new(),
        }
    }
}

impl PredictConfig {
    /// 配置里的密钥优先，其次环境变量；两边都没有返回 `None`。
    pub fn resolve_api_key(&self) -> Option<String> {
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
            automatic: self.automatic,
            before: self.lookback,
            after: self.lookahead,
            slots: self.slots,
            // 比槽位多要两条，与本地候选重复的去掉后还能填满；问字模式的答案也按这个数要
            max_items: self.slots.max(1) + 2,
            sentence: self.sentence,
        }
    }

    /// 用不用本机模型：填了模型文件就用。
    pub fn uses_local_model(&self) -> bool {
        !self.local_model.trim().is_empty()
    }

    /// 本机服务实际监听的端口。
    pub fn served_port(&self) -> u16 {
        match self.local_port {
            0 => DEFAULT_LOCAL_PORT,
            port => port,
        }
    }

    /// 请求实际发往的接口地址：用本机模型时是本机服务，否则是 `base_url`。
    pub fn endpoint(&self) -> String {
        if self.uses_local_model() {
            format!("http://127.0.0.1:{}/v1", self.served_port())
        } else {
            self.base_url.clone()
        }
    }

    /// 请求、日志与界面里的模型名：本机模型是文件名（不含扩展名），否则是 `model`。
    pub fn model_name(&self) -> String {
        if !self.uses_local_model() {
            return self.model.clone();
        }
        let path = self.local_model.trim();
        Path::new(path).file_stem().map_or_else(
            || path.to_owned(),
            |stem| stem.to_string_lossy().into_owned(),
        )
    }

    /// 发请求要不要密钥：本机模型与本机地址（`localhost`、`127.0.0.1`、`::1`）上的服务不要。
    pub fn needs_api_key(&self) -> bool {
        !self.uses_local_model() && !is_loopback(&self.base_url)
    }
}

/// 接口地址是不是本机：主机名是 `localhost`，或是环回地址（`127.0.0.0/8`、`::1`）。
pub(crate) fn is_loopback(base_url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(base_url.trim()) else {
        return false;
    };
    url.host_str().is_some_and(|host| {
        let host = host.trim_start_matches('[').trim_end_matches(']');
        host.eq_ignore_ascii_case("localhost")
            || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_addresses_are_recognized_by_host() {
        for url in [
            "http://127.0.0.1:8080/v1",
            "http://localhost:1234/v1",
            "http://LOCALHOST/v1",
            "http://127.1.2.3/v1",
            "http://[::1]:11434/v1",
        ] {
            assert!(is_loopback(url), "{url}");
        }
        for url in [
            "https://api.deepseek.com",
            "http://192.168.1.5:8080/v1",
            "https://example.com/localhost",
            "http://localhost.example.com/v1",
            "不是地址",
        ] {
            assert!(!is_loopback(url), "{url}");
        }
    }

    #[test]
    fn local_model_replaces_the_service_and_needs_no_key() {
        let remote = PredictConfig::default();
        assert!(!remote.uses_local_model());
        assert!(remote.needs_api_key());
        assert_eq!(remote.endpoint(), "https://api.deepseek.com");
        assert_eq!(remote.model_name(), "deepseek-flash");

        let own_server = PredictConfig {
            base_url: "http://localhost:1234/v1".to_owned(),
            ..PredictConfig::default()
        };
        assert!(!own_server.needs_api_key());

        let local = PredictConfig {
            local_model: " /models/Qwen3.5-2B-Q6_K.gguf ".to_owned(),
            ..PredictConfig::default()
        };
        assert!(local.uses_local_model());
        assert!(!local.needs_api_key());
        assert_eq!(local.endpoint(), "http://127.0.0.1:18686/v1");
        assert_eq!(local.model_name(), "Qwen3.5-2B-Q6_K");

        let zero_port = PredictConfig {
            local_port: 0,
            ..local
        };
        assert_eq!(zero_port.served_port(), DEFAULT_LOCAL_PORT);
    }
}
