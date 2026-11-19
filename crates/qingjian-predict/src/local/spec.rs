//! llama-server 的启动参数：来自 `[predict]` 的 `local_*` 几项；参数相同的客户端共用一个服务。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::config::PredictConfig;

/// 服务的模型名（`--alias`）：换模型时认端口上那个还没停的服务是不是青简起的，看的就是它。
pub(crate) const ALIAS: &str = "qingjian-local";

/// 青简给的参数（需 llama.cpp b10141 起）：
/// - 上下文 8192 分两路并发，每路 4096 token，联想、释义与测试连接同时来也够用；
/// - 关掉思考：联想要快，Qwen3.5 这类模型缺省会先想一大段，正文是空的；
/// - 只认同源的浏览器请求、关掉 `/slots`（那里能读到请求的原文）：网页读不到打过的字；不开网页界面；
/// - 上下文检查点与提示缓存限小：混合注意力的模型（Qwen3.5）也能复用前缀，又不多占内存。
const DEFAULT_ARGS: &[&str] = &[
    "-c",
    "8192",
    "-np",
    "2",
    "--reasoning",
    "off",
    "--cors-origins",
    "localhost",
    "--no-slots",
    "--no-webui",
    "--ctx-checkpoints",
    "4",
    "--cache-ram",
    "256",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchSpec {
    /// GGUF 模型文件，原样传给 `-m`：服务回报的模型路径与它比，认出是不是同一个模型。
    pub model: String,

    /// 配置里指定的 llama-server；`None` 时自己找。
    pub server: Option<PathBuf>,

    /// 监听的端口（地址固定 127.0.0.1）。
    pub port: u16,

    /// 用户追加的参数。
    pub extra: Vec<String>,

    /// 用户给的环境变量；变量名为空或带 `=` 的已去掉（系统不认这样的名字）。
    pub env: BTreeMap<String, String>,
}

impl LaunchSpec {
    /// 没用本机模型返回 `None`。
    pub fn from_config(config: &PredictConfig) -> Option<Self> {
        let server = config.local_server.trim();
        config.uses_local_model().then(|| Self {
            model: config.local_model.trim().to_owned(),
            server: (!server.is_empty()).then(|| PathBuf::from(server)),
            port: config.served_port(),
            extra: config.local_args.clone(),
            env: config
                .local_env
                .iter()
                .map(|(name, value)| (name.trim().to_owned(), value.clone()))
                .filter(|(name, _)| !name.is_empty() && !name.contains('='))
                .collect(),
        })
    }

    /// 完整的命令行参数：模型与地址在前，青简的缺省居中，用户追加的在最后（同一参数 llama-server 以后写的为准）。
    pub fn arguments(&self) -> Vec<OsString> {
        let port = self.port.to_string();
        let mut arguments: Vec<OsString> = [
            "-m",
            self.model.as_str(),
            "--host",
            "127.0.0.1",
            "--port",
            port.as_str(),
            "--alias",
            ALIAS,
        ]
        .into_iter()
        .chain(DEFAULT_ARGS.iter().copied())
        .map(OsString::from)
        .collect();
        arguments.extend(self.extra.iter().map(OsString::from));
        arguments
    }
}
